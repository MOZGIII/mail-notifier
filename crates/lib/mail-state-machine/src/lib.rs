//! Event deducer.
//!
//! A state machine that tracks per-mailbox unread counts and deduces
//! when to emit events such as "new mail" based on count changes.
//!
//! [`State`] owns the primary [`slotmap::SlotMap`] for entry keys and
//! stores caller-supplied values alongside the internal tracking state.
//! App-level data goes in via the `UserData` type parameter — no secondary
//! map needed, so keys are always valid by construction.

use slotmap::{Key, SlotMap};
use supervisor::SupervisorEvent;

pub use mail_state_machine_core::HasUnread;

/// The outcome of processing a workload update.
///
/// Returned by [`State::process_workload_update`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Effects {
    /// Whether new mail was detected — the unread count for this specific
    /// mailbox increased compared to the previous known value.
    pub new_mail: bool,

    /// The new total unread count across all tracked mailboxes, if it
    /// changed as a result of this update.
    ///
    /// [`None`] when the total is unchanged.
    pub total_unread_changed: Option<u32>,
}

/// Read-only tracking state for a single mailbox.
///
/// Obtained via [`Entry::tracked`].  All fields are public for direct
/// read access, but the state machine controls mutation internally.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tracked {
    /// Last known unread count, or [`None`] if no update has been
    /// received yet (no baseline established).
    pub unread: Option<u32>,

    /// Whether the mailbox monitor is currently active (connected).
    pub active: bool,
}

/// Combined entry: caller-supplied user data plus tracking state.
///
/// Returned by [`State::get`], [`State::get_mut`], and [`State::iter`].
/// The [`user_data`](Self::user_data) field is directly accessible; the
/// tracking state is readable via [`tracked`](Self::tracked).  Internal
/// fields are private so callers cannot corrupt the state machine.
#[derive(Debug, Clone)]
pub struct Entry<UserData> {
    /// Caller-supplied user data.
    pub user_data: UserData,
    /// Tracking state (read-only to callers).
    tracked: Tracked,
}

impl<UserData> Entry<UserData> {
    /// Tracking state for this mailbox.
    pub fn tracked(&self) -> &Tracked {
        &self.tracked
    }
}

/// State machine that tracks per-mailbox unread counts and deduces events.
///
/// Owns a single [`SlotMap`] that stores both the caller-supplied data
/// and the internal tracking state for each entry.  Because there is
/// only one map, every key returned by [`insert`](Self::insert) is
/// guaranteed to be valid for all methods — there is no secondary
/// store that could fall out of sync.
///
/// Generic over the entry key `K` ([`slotmap::Key`]) and the
/// app-level user data `UserData`.
#[derive(Debug)]
pub struct State<K: Key, UserData> {
    /// Tracked entries.
    entries: SlotMap<K, Entry<UserData>>,
    /// Cached total unread count across all tracked entries.
    total_unread: u32,
}

impl<K: Key, UserData> Default for State<K, UserData> {
    fn default() -> Self {
        Self {
            entries: SlotMap::with_key(),
            total_unread: 0,
        }
    }
}

impl<K: Key, UserData> State<K, UserData> {
    /// Create a new, empty state machine.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new mailbox entry with the given user data and return
    /// its key.
    ///
    /// The entry starts with no unread count — the first
    /// [`process_workload_update`](Self::process_workload_update) call will
    /// establish the baseline without emitting [`Effects::new_mail`].
    pub fn insert(&mut self, user_data: UserData) -> K {
        self.entries.insert(Entry {
            user_data,
            tracked: Tracked {
                unread: None,
                active: false,
            },
        })
    }

    /// Process a workload update for a mailbox entry.
    ///
    /// Accepts any payload implementing [`HasUnread`] and returns
    /// [`Effects`] describing what happened:
    ///
    /// - [`new_mail`](Effects::new_mail) is `true` when the new `unread`
    ///   count is strictly greater than the previously recorded count for
    ///   this entry **and** a baseline was already established. The very
    ///   first update for an entry establishes a baseline and never sets
    ///   `new_mail`.
    ///
    /// - [`total_unread_changed`](Effects::total_unread_changed) is
    ///   [`Some(new_total)`] when the global total changed.
    pub fn process_workload_update(&mut self, entry: K, payload: &impl HasUnread) -> Effects {
        let unread = payload.unread();
        let old_total = self.total_unread;
        let mut new_mail = false;

        if let Some(state) = self.entries.get_mut(entry) {
            let prev = state.tracked.unread.unwrap_or(0);
            self.total_unread = self.total_unread - prev + unread;
            if let Some(old) = state.tracked.unread
                && unread > old
            {
                new_mail = true;
            }
            state.tracked.unread = Some(unread);
        }

        let total_unread_changed = if self.total_unread != old_total {
            Some(self.total_unread)
        } else {
            None
        };

        Effects {
            new_mail,
            total_unread_changed,
        }
    }

    /// Process a [`SupervisorEvent`], updating the active state of the
    /// entry.
    ///
    /// [`SupervisorEvent::Started`] marks the entry as active; all other
    /// variants mark it as inactive.
    pub fn process_supervisor_update<T, E>(&mut self, entry: K, event: &SupervisorEvent<T, E>) {
        let active = matches!(event, SupervisorEvent::Started);
        if let Some(state) = self.entries.get_mut(entry) {
            state.tracked.active = active;
        }
    }

    /// Total unread count across all tracked mailboxes.
    pub fn total_unread(&self) -> u32 {
        self.total_unread
    }

    /// Number of tracked mailbox entries.
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Look up an entry by key.
    ///
    /// Returns the [`Entry`] which gives access to both the
    /// caller-supplied [`user_data`](Entry::user_data) and the
    /// [`tracked`](Entry::tracked) state in a single lookup.
    pub fn get(&self, key: K) -> Option<&Entry<UserData>> {
        self.entries.get(key)
    }

    /// Look up an entry by key, mutably.
    ///
    /// The caller may modify [`Entry::user_data`]; the tracking state
    /// remains controlled by the state machine.
    pub fn get_mut(&mut self, key: K) -> Option<&mut Entry<UserData>> {
        self.entries.get_mut(key)
    }

    /// Iterate over all entries, yielding `(key, &entry)`.
    pub fn iter(&self) -> impl Iterator<Item = (K, &Entry<UserData>)> {
        self.entries.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test payload carrying an unread count.
    struct Payload(u32);

    impl HasUnread for Payload {
        fn unread(&self) -> u32 {
            self.0
        }
    }

    #[test]
    fn baseline_does_not_fire_new_mail() {
        let mut state = State::<slotmap::DefaultKey, ()>::new();
        let inbox = state.insert(());
        let effects = state.process_workload_update(inbox, &Payload(5));
        assert!(!effects.new_mail);
        assert_eq!(effects.total_unread_changed, Some(5));
    }

    #[test]
    fn baseline_zero_does_not_change_total() {
        let mut state = State::<slotmap::DefaultKey, ()>::new();
        let inbox = state.insert(());
        let effects = state.process_workload_update(inbox, &Payload(0));
        assert!(!effects.new_mail);
        assert_eq!(effects.total_unread_changed, None);
    }

    #[test]
    fn increase_fires_new_mail() {
        let mut state = State::<slotmap::DefaultKey, ()>::new();
        let inbox = state.insert(());
        state.process_workload_update(inbox, &Payload(3));

        let effects = state.process_workload_update(inbox, &Payload(4));
        assert!(effects.new_mail);
        assert_eq!(effects.total_unread_changed, Some(4));
    }

    #[test]
    fn decrease_does_not_fire() {
        let mut state = State::<slotmap::DefaultKey, ()>::new();
        let inbox = state.insert(());
        state.process_workload_update(inbox, &Payload(5));

        let effects = state.process_workload_update(inbox, &Payload(3));
        assert!(!effects.new_mail);
        assert_eq!(effects.total_unread_changed, Some(3));
    }

    #[test]
    fn same_count_does_not_fire() {
        let mut state = State::<slotmap::DefaultKey, ()>::new();
        let inbox = state.insert(());
        state.process_workload_update(inbox, &Payload(5));

        let effects = state.process_workload_update(inbox, &Payload(5));
        assert!(!effects.new_mail);
        assert_eq!(effects.total_unread_changed, None);
    }

    #[test]
    fn independent_mailboxes_fire_independently() {
        let mut state = State::<slotmap::DefaultKey, ()>::new();
        let a = state.insert(());
        let b = state.insert(());
        // Baselines.
        state.process_workload_update(a, &Payload(3));
        state.process_workload_update(b, &Payload(7));

        // +1 on a, -1 on b → should still fire new_mail for a.
        let effects_a = state.process_workload_update(a, &Payload(4));
        assert!(effects_a.new_mail);
        assert_eq!(effects_a.total_unread_changed, Some(11));

        let effects_b = state.process_workload_update(b, &Payload(6));
        assert!(!effects_b.new_mail);
        assert_eq!(effects_b.total_unread_changed, Some(10));
    }

    #[test]
    fn plus_one_minus_one_total_unchanged() {
        let mut state = State::<slotmap::DefaultKey, ()>::new();
        let a = state.insert(());
        let b = state.insert(());
        state.process_workload_update(a, &Payload(3));
        state.process_workload_update(b, &Payload(7));
        // total = 10

        // +1 on a → total 11
        state.process_workload_update(a, &Payload(4));

        // -1 on b → total back to 10
        let effects = state.process_workload_update(b, &Payload(6));
        assert_eq!(effects.total_unread_changed, Some(10));

        // Now set both to same → no total change.
        let effects = state.process_workload_update(a, &Payload(4));
        assert_eq!(effects.total_unread_changed, None);
    }

    #[test]
    fn total_unread_sums_all() {
        let mut state = State::<slotmap::DefaultKey, ()>::new();
        let a = state.insert(());
        let b = state.insert(());
        state.process_workload_update(a, &Payload(3));
        state.process_workload_update(b, &Payload(7));

        assert_eq!(state.total_unread(), 10);
    }

    #[test]
    fn unread_for_returns_current_count() {
        let mut state = State::<slotmap::DefaultKey, ()>::new();
        let inbox = state.insert(());
        // Freshly inserted — no updates yet.
        assert_eq!(state.get(inbox).map(|e| e.tracked().unread), Some(None));

        state.process_workload_update(inbox, &Payload(5));
        assert_eq!(state.get(inbox).map(|e| e.tracked().unread), Some(Some(5)));
    }

    #[test]
    fn entry_count_tracks_entries() {
        let mut state = State::<slotmap::DefaultKey, ()>::new();
        assert_eq!(state.entry_count(), 0);

        state.insert(());
        state.insert(());
        assert_eq!(state.entry_count(), 2);
    }

    #[test]
    fn increase_from_zero_fires() {
        let mut state = State::<slotmap::DefaultKey, ()>::new();
        let inbox = state.insert(());
        state.process_workload_update(inbox, &Payload(0));

        let effects = state.process_workload_update(inbox, &Payload(1));
        assert!(effects.new_mail);
        assert_eq!(effects.total_unread_changed, Some(1));
    }

    #[test]
    fn reconnect_fires_on_higher_count() {
        let mut state = State::<slotmap::DefaultKey, ()>::new();
        let inbox = state.insert(());
        state.process_workload_update(inbox, &Payload(5));

        // Reconnect reports a higher count — new_mail fires.
        let effects = state.process_workload_update(inbox, &Payload(7));
        assert!(effects.new_mail);
        assert_eq!(effects.total_unread_changed, Some(7));
    }

    #[test]
    fn reconnect_same_count_does_not_fire() {
        let mut state = State::<slotmap::DefaultKey, ()>::new();
        let inbox = state.insert(());
        state.process_workload_update(inbox, &Payload(5));

        // Reconnect reports the same count — no new_mail, no total change.
        let effects = state.process_workload_update(inbox, &Payload(5));
        assert!(!effects.new_mail);
        assert_eq!(effects.total_unread_changed, None);
    }

    #[test]
    fn get_returns_stored_value() {
        let mut state = State::<slotmap::DefaultKey, &str>::new();
        let inbox = state.insert("inbox");
        assert_eq!(state.get(inbox).map(|e| &e.user_data), Some(&"inbox"));
    }

    #[test]
    fn get_mut_modifies_stored_value() {
        let mut state = State::<slotmap::DefaultKey, String>::new();
        let inbox = state.insert("inbox".to_owned());
        if let Some(entry) = state.get_mut(inbox) {
            entry.user_data.push_str("_modified");
        }
        assert_eq!(
            state.get(inbox).map(|e| e.user_data.as_str()),
            Some("inbox_modified"),
        );
    }

    #[test]
    fn iter_yields_all_entries() {
        let mut state = State::<slotmap::DefaultKey, &str>::new();
        let a = state.insert("a");
        let b = state.insert("b");
        state.process_workload_update(a, &Payload(3));
        state.process_workload_update(b, &Payload(7));

        let items: Vec<(_, Option<u32>, &&str)> = state
            .iter()
            .map(|(k, e)| (k, e.tracked().unread, &e.user_data))
            .collect();
        assert_eq!(items.len(), 2);
        assert!(items.contains(&(a, Some(3), &&"a")));
        assert!(items.contains(&(b, Some(7), &&"b")));
    }

    #[test]
    fn process_supervisor_started_sets_active() {
        let mut state = State::<slotmap::DefaultKey, ()>::new();
        let inbox = state.insert(());
        assert!(!state.get(inbox).unwrap().tracked().active);

        state.process_supervisor_update(
            inbox,
            &SupervisorEvent::<core::convert::Infallible, ()>::Started,
        );
        assert!(state.get(inbox).unwrap().tracked().active);
    }

    #[test]
    fn process_supervisor_error_clears_active() {
        let mut state = State::<slotmap::DefaultKey, ()>::new();
        let inbox = state.insert(());

        state.process_supervisor_update(
            inbox,
            &SupervisorEvent::<core::convert::Infallible, ()>::Started,
        );
        assert!(state.get(inbox).unwrap().tracked().active);

        state.process_supervisor_update(
            inbox,
            &SupervisorEvent::<core::convert::Infallible, ()>::Error {
                error: (),
                next_retry_in: std::time::Duration::ZERO,
            },
        );
        assert!(!state.get(inbox).unwrap().tracked().active);
    }
}
