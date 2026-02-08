//! Event deducer.
//!
//! A state machine that tracks per-mailbox unread counts and deduces
//! when to emit events such as "new mail" based on count changes.

use slotmap::{Key, SecondaryMap};

/// The outcome of processing an unread count update.
///
/// Returned by [`State::process_unread_update`].
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

/// Per-entry tracked state.
#[derive(Debug, Clone, Copy)]
struct EntryState {
    /// Last known unread count.
    unread: u32,
    /// Whether the baseline has been established.
    ///
    /// When `false`, the next update sets the baseline without emitting
    /// [`Effects::new_mail`].
    has_baseline: bool,
}

/// State machine that tracks per-mailbox unread counts and deduces events.
///
/// Generic over the mailbox entry key `K`, which must be a [`slotmap::Key`].
#[derive(Debug)]
pub struct State<K: Key> {
    /// Tracked per-entry state, keyed by mailbox entry.
    entries: SecondaryMap<K, EntryState>,
    /// Cached total unread count across all tracked entries.
    total_unread: u32,
}

impl<K: Key> Default for State<K> {
    fn default() -> Self {
        Self {
            entries: SecondaryMap::new(),
            total_unread: 0,
        }
    }
}

impl<K: Key> State<K> {
    /// Create a new, empty state machine.
    pub fn new() -> Self {
        Self::default()
    }

    /// Process an updated unread count for a mailbox entry.
    ///
    /// Returns [`Effects`] describing what happened:
    ///
    /// - [`new_mail`](Effects::new_mail) is `true` when the new `unread`
    ///   count is strictly greater than the previously recorded count for
    ///   this entry **and** a baseline was already established. The very
    ///   first update for an entry (or the first after
    ///   [`reset_entry`](Self::reset_entry)) establishes a baseline and
    ///   never sets `new_mail`.
    ///
    /// - [`total_unread_changed`](Effects::total_unread_changed) is
    ///   [`Some(new_total)`] when the global total changed.
    pub fn process_unread_update(&mut self, entry: K, unread: u32) -> Effects {
        let old_total = self.total_unread;
        let mut new_mail = false;

        if let Some(state) = self.entries.get_mut(entry) {
            let prev = state.unread;
            self.total_unread = self.total_unread - prev + unread;
            if state.has_baseline && unread > prev {
                new_mail = true;
            }
            state.unread = unread;
            state.has_baseline = true;
        } else {
            self.total_unread += unread;
            self.entries.insert(
                entry,
                EntryState {
                    unread,
                    has_baseline: true,
                },
            );
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

    /// Reset the new-mail detection baseline for a mailbox entry.
    ///
    /// The next [`process_unread_update`](Self::process_unread_update)
    /// call for this entry will establish a fresh baseline without
    /// emitting [`Effects::new_mail`].
    ///
    /// The entry's unread count is preserved so that
    /// [`total_unread`](Self::total_unread) remains stable — useful when
    /// a connection drops and you don't want the icon badge to flicker.
    pub fn reset_entry(&mut self, entry: K) {
        if let Some(state) = self.entries.get_mut(entry) {
            state.has_baseline = false;
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

    /// Unread count for a specific entry, if tracked.
    pub fn unread_for(&self, entry: K) -> Option<u32> {
        self.entries.get(entry).map(|s| s.unread)
    }
}

#[cfg(test)]
mod tests {
    use slotmap::SlotMap;

    use super::*;

    /// Helper: insert a named slot and return its key.
    fn key(
        slots: &mut SlotMap<slotmap::DefaultKey, &str>,
        name: &'static str,
    ) -> slotmap::DefaultKey {
        slots.insert(name)
    }

    #[test]
    fn baseline_does_not_fire_new_mail() {
        let mut slots = SlotMap::new();
        let inbox = key(&mut slots, "inbox");
        let mut state = State::new();
        let effects = state.process_unread_update(inbox, 5);
        assert!(!effects.new_mail);
        assert_eq!(effects.total_unread_changed, Some(5));
    }

    #[test]
    fn baseline_zero_does_not_change_total() {
        let mut slots = SlotMap::new();
        let inbox = key(&mut slots, "inbox");
        let mut state = State::new();
        let effects = state.process_unread_update(inbox, 0);
        assert!(!effects.new_mail);
        assert_eq!(effects.total_unread_changed, None);
    }

    #[test]
    fn increase_fires_new_mail() {
        let mut slots = SlotMap::new();
        let inbox = key(&mut slots, "inbox");
        let mut state = State::new();
        state.process_unread_update(inbox, 3);

        let effects = state.process_unread_update(inbox, 4);
        assert!(effects.new_mail);
        assert_eq!(effects.total_unread_changed, Some(4));
    }

    #[test]
    fn decrease_does_not_fire() {
        let mut slots = SlotMap::new();
        let inbox = key(&mut slots, "inbox");
        let mut state = State::new();
        state.process_unread_update(inbox, 5);

        let effects = state.process_unread_update(inbox, 3);
        assert!(!effects.new_mail);
        assert_eq!(effects.total_unread_changed, Some(3));
    }

    #[test]
    fn same_count_does_not_fire() {
        let mut slots = SlotMap::new();
        let inbox = key(&mut slots, "inbox");
        let mut state = State::new();
        state.process_unread_update(inbox, 5);

        let effects = state.process_unread_update(inbox, 5);
        assert!(!effects.new_mail);
        assert_eq!(effects.total_unread_changed, None);
    }

    #[test]
    fn independent_mailboxes_fire_independently() {
        let mut slots = SlotMap::new();
        let a = key(&mut slots, "a");
        let b = key(&mut slots, "b");
        let mut state = State::new();
        // Baselines.
        state.process_unread_update(a, 3);
        state.process_unread_update(b, 7);

        // +1 on a, -1 on b → should still fire new_mail for a.
        let effects_a = state.process_unread_update(a, 4);
        assert!(effects_a.new_mail);
        assert_eq!(effects_a.total_unread_changed, Some(11));

        let effects_b = state.process_unread_update(b, 6);
        assert!(!effects_b.new_mail);
        assert_eq!(effects_b.total_unread_changed, Some(10));
    }

    #[test]
    fn plus_one_minus_one_total_unchanged() {
        let mut slots = SlotMap::new();
        let a = key(&mut slots, "a");
        let b = key(&mut slots, "b");
        let mut state = State::new();
        state.process_unread_update(a, 3);
        state.process_unread_update(b, 7);
        // total = 10

        // +1 on a → total 11
        state.process_unread_update(a, 4);

        // -1 on b → total back to 10
        let effects = state.process_unread_update(b, 6);
        assert_eq!(effects.total_unread_changed, Some(10));

        // Now set both to same → no total change.
        let effects = state.process_unread_update(a, 4);
        assert_eq!(effects.total_unread_changed, None);
    }

    #[test]
    fn total_unread_sums_all() {
        let mut slots = SlotMap::new();
        let a = key(&mut slots, "a");
        let b = key(&mut slots, "b");
        let mut state = State::new();
        state.process_unread_update(a, 3);
        state.process_unread_update(b, 7);

        assert_eq!(state.total_unread(), 10);
    }

    #[test]
    fn reset_entry_makes_next_update_baseline() {
        let mut slots = SlotMap::new();
        let inbox = key(&mut slots, "inbox");
        let mut state = State::new();
        state.process_unread_update(inbox, 5);
        state.reset_entry(inbox);

        // After reset the next update is a baseline again — no new_mail.
        let effects = state.process_unread_update(inbox, 10);
        assert!(!effects.new_mail);
        // But total DID change (5 → 10).
        assert_eq!(effects.total_unread_changed, Some(10));

        // A subsequent increase fires.
        let effects = state.process_unread_update(inbox, 11);
        assert!(effects.new_mail);
    }

    #[test]
    fn reset_entry_preserves_total() {
        let mut slots = SlotMap::new();
        let inbox = key(&mut slots, "inbox");
        let mut state = State::new();
        state.process_unread_update(inbox, 5);

        // Reset doesn't change total — count is preserved.
        state.reset_entry(inbox);
        assert_eq!(state.total_unread(), 5);
    }

    #[test]
    fn reset_entry_preserves_entry() {
        let mut slots = SlotMap::new();
        let inbox = key(&mut slots, "inbox");
        let mut state = State::new();
        state.process_unread_update(inbox, 5);
        state.reset_entry(inbox);

        // Entry is still tracked.
        assert_eq!(state.entry_count(), 1);
        assert_eq!(state.unread_for(inbox), Some(5));
    }

    #[test]
    fn unread_for_returns_current_count() {
        let mut slots = SlotMap::new();
        let inbox = key(&mut slots, "inbox");
        let mut state = State::new();
        assert_eq!(state.unread_for(inbox), None);

        state.process_unread_update(inbox, 5);
        assert_eq!(state.unread_for(inbox), Some(5));
    }

    #[test]
    fn entry_count_tracks_entries() {
        let mut slots = SlotMap::new();
        let a = key(&mut slots, "a");
        let b = key(&mut slots, "b");
        let mut state = State::new();
        assert_eq!(state.entry_count(), 0);

        state.process_unread_update(a, 0);
        state.process_unread_update(b, 0);
        assert_eq!(state.entry_count(), 2);
    }

    #[test]
    fn increase_from_zero_fires() {
        let mut slots = SlotMap::new();
        let inbox = key(&mut slots, "inbox");
        let mut state = State::new();
        state.process_unread_update(inbox, 0);

        let effects = state.process_unread_update(inbox, 1);
        assert!(effects.new_mail);
        assert_eq!(effects.total_unread_changed, Some(1));
    }

    #[test]
    fn reconnect_baseline_does_not_fire_on_same_count() {
        let mut slots = SlotMap::new();
        let inbox = key(&mut slots, "inbox");
        let mut state = State::new();
        state.process_unread_update(inbox, 5);
        state.reset_entry(inbox);

        // Reconnect reports the same count — no new_mail, no total change.
        let effects = state.process_unread_update(inbox, 5);
        assert!(!effects.new_mail);
        assert_eq!(effects.total_unread_changed, None);
    }
}
