//! Mail state machine.
//!
//! Tracks per-mailbox unread counts and deduces events such as
//! "new mail" based on count changes.

use slotmap::{Key, SlotMap};
use supervisor::SupervisorEvent;

pub use mail_state_machine_core as core;

/// The outcome of processing a workload update.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkloadEffects {
    /// Whether new mail was detected — the unread count increased
    /// compared to the previously known value.
    pub new_mail: bool,

    /// The new total unread count, if it changed.
    pub total_unread_changed: Option<u32>,
}

/// Read-only tracking state for a single mailbox.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tracked {
    /// Last known unread count, or [`None`] before the first update.
    pub unread: Option<u32>,

    /// Whether the mailbox monitor is currently active.
    pub active: bool,
}

/// Caller-supplied user data plus tracking state.
#[derive(Debug, Clone)]
pub struct Entry<UserData> {
    /// Caller-supplied user data.
    pub user_data: UserData,

    /// Tracking state.
    tracked: Tracked,
}

impl<UserData> Entry<UserData> {
    /// Returns the tracking state.
    pub fn tracked(&self) -> &Tracked {
        &self.tracked
    }
}

/// Tracks per-mailbox unread counts and deduces events.
#[derive(Debug)]
pub struct State<K: Key, UserData> {
    /// Tracked entries.
    entries: SlotMap<K, Entry<UserData>>,

    /// Aggregated counters.
    totals: Totals,
}

/// Aggregated counters across all tracked entries.
#[derive(Debug, Default)]
pub struct Totals {
    /// Total unread count.
    pub unread: u32,
}

impl<K: Key, UserData> Default for State<K, UserData> {
    fn default() -> Self {
        Self {
            entries: SlotMap::with_key(),
            totals: Totals::default(),
        }
    }
}

impl<K: Key, UserData> State<K, UserData> {
    /// Create a new, empty state machine.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new entry and return its key.
    pub fn insert(&mut self, user_data: UserData) -> K {
        self.entries.insert(Entry {
            user_data,
            tracked: Tracked {
                unread: None,
                active: false,
            },
        })
    }

    /// Begin processing an update for the given entry.
    ///
    /// Returns [`None`] when the key is not found.
    pub fn process_update(&mut self, key: K) -> Option<UpdateProcessor<'_, UserData>> {
        let entry = self.entries.get_mut(key)?;
        Some(UpdateProcessor {
            entry,
            totals: &mut self.totals,
        })
    }

    /// Aggregated counters.
    pub fn totals(&self) -> &Totals {
        &self.totals
    }

    /// Read-only view of the tracked entries.
    pub fn entries(&self) -> &SlotMap<K, Entry<UserData>> {
        &self.entries
    }

    /// Look up an entry by key, mutably.
    pub fn get_entry_mut(&mut self, key: K) -> Option<&mut Entry<UserData>> {
        self.entries.get_mut(key)
    }
}

/// Handle for applying updates to a single entry.
pub struct UpdateProcessor<'a, UserData> {
    /// The entry being updated.
    entry: &'a mut Entry<UserData>,

    /// Shared aggregated counters.
    totals: &'a mut Totals,
}

impl<UserData> UpdateProcessor<'_, UserData> {
    /// Apply a workload update, returning the resulting [`WorkloadEffects`].
    ///
    /// The very first update establishes a baseline and never
    /// sets [`new_mail`](WorkloadEffects::new_mail).
    pub fn workload(&mut self, payload: &impl core::WorkloadPayload) -> WorkloadEffects {
        let unread = payload.unread();
        let old_total = self.totals.unread;
        let prev_unread = self.entry.tracked.unread;

        self.entry.tracked.unread = Some(unread);
        self.totals.unread = old_total
            .saturating_sub(prev_unread.unwrap_or(0))
            .saturating_add(unread);

        let new_mail = prev_unread.is_some_and(|old| unread > old);
        let total_unread_changed = (self.totals.unread != old_total).then_some(self.totals.unread);

        WorkloadEffects {
            new_mail,
            total_unread_changed,
        }
    }

    /// Apply a [`SupervisorEvent`], updating the entry's active state.
    pub fn supervisor<T, E>(&mut self, event: &SupervisorEvent<T, E>) {
        self.entry.tracked.active = matches!(event, SupervisorEvent::Started);
    }
}

#[cfg(test)]
mod tests;
