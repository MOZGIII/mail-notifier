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
    let effects = state.process_update(inbox).unwrap().workload(&Payload(5));
    assert!(!effects.new_mail);
    assert_eq!(effects.total_unread_changed, Some(5));
}

#[test]
fn baseline_zero_does_not_change_total() {
    let mut state = State::<slotmap::DefaultKey, ()>::new();
    let inbox = state.insert(());
    let effects = state.process_update(inbox).unwrap().workload(&Payload(0));
    assert!(!effects.new_mail);
    assert_eq!(effects.total_unread_changed, None);
}

#[test]
fn increase_fires_new_mail() {
    let mut state = State::<slotmap::DefaultKey, ()>::new();
    let inbox = state.insert(());
    state.process_update(inbox).unwrap().workload(&Payload(3));

    let effects = state.process_update(inbox).unwrap().workload(&Payload(4));
    assert!(effects.new_mail);
    assert_eq!(effects.total_unread_changed, Some(4));
}

#[test]
fn decrease_does_not_fire() {
    let mut state = State::<slotmap::DefaultKey, ()>::new();
    let inbox = state.insert(());
    state.process_update(inbox).unwrap().workload(&Payload(5));

    let effects = state.process_update(inbox).unwrap().workload(&Payload(3));
    assert!(!effects.new_mail);
    assert_eq!(effects.total_unread_changed, Some(3));
}

#[test]
fn same_count_does_not_fire() {
    let mut state = State::<slotmap::DefaultKey, ()>::new();
    let inbox = state.insert(());
    state.process_update(inbox).unwrap().workload(&Payload(5));

    let effects = state.process_update(inbox).unwrap().workload(&Payload(5));
    assert!(!effects.new_mail);
    assert_eq!(effects.total_unread_changed, None);
}

#[test]
fn independent_mailboxes_fire_independently() {
    let mut state = State::<slotmap::DefaultKey, ()>::new();
    let a = state.insert(());
    let b = state.insert(());
    // Baselines.
    state.process_update(a).unwrap().workload(&Payload(3));
    state.process_update(b).unwrap().workload(&Payload(7));

    // +1 on a, -1 on b → should still fire new_mail for a.
    let effects_a = state.process_update(a).unwrap().workload(&Payload(4));
    assert!(effects_a.new_mail);
    assert_eq!(effects_a.total_unread_changed, Some(11));

    let effects_b = state.process_update(b).unwrap().workload(&Payload(6));
    assert!(!effects_b.new_mail);
    assert_eq!(effects_b.total_unread_changed, Some(10));
}

#[test]
fn plus_one_minus_one_total_unchanged() {
    let mut state = State::<slotmap::DefaultKey, ()>::new();
    let a = state.insert(());
    let b = state.insert(());
    state.process_update(a).unwrap().workload(&Payload(3));
    state.process_update(b).unwrap().workload(&Payload(7));
    // total = 10

    // +1 on a → total 11
    state.process_update(a).unwrap().workload(&Payload(4));

    // -1 on b → total back to 10
    let effects = state.process_update(b).unwrap().workload(&Payload(6));
    assert_eq!(effects.total_unread_changed, Some(10));

    // Now set both to same → no total change.
    let effects = state.process_update(a).unwrap().workload(&Payload(4));
    assert_eq!(effects.total_unread_changed, None);
}

#[test]
fn total_unread_sums_all() {
    let mut state = State::<slotmap::DefaultKey, ()>::new();
    let a = state.insert(());
    let b = state.insert(());
    state.process_update(a).unwrap().workload(&Payload(3));
    state.process_update(b).unwrap().workload(&Payload(7));

    assert_eq!(state.total_unread(), 10);
}

#[test]
fn unread_for_returns_current_count() {
    let mut state = State::<slotmap::DefaultKey, ()>::new();
    let inbox = state.insert(());
    // Freshly inserted — no updates yet.
    assert_eq!(
        state.entries().get(inbox).map(|e| e.tracked().unread),
        Some(None)
    );

    state.process_update(inbox).unwrap().workload(&Payload(5));
    assert_eq!(
        state.entries().get(inbox).map(|e| e.tracked().unread),
        Some(Some(5))
    );
}

#[test]
fn entry_count_tracks_entries() {
    let mut state = State::<slotmap::DefaultKey, ()>::new();
    assert_eq!(state.entries().len(), 0);

    state.insert(());
    state.insert(());
    assert_eq!(state.entries().len(), 2);
}

#[test]
fn increase_from_zero_fires() {
    let mut state = State::<slotmap::DefaultKey, ()>::new();
    let inbox = state.insert(());
    state.process_update(inbox).unwrap().workload(&Payload(0));

    let effects = state.process_update(inbox).unwrap().workload(&Payload(1));
    assert!(effects.new_mail);
    assert_eq!(effects.total_unread_changed, Some(1));
}

#[test]
fn reconnect_fires_on_higher_count() {
    let mut state = State::<slotmap::DefaultKey, ()>::new();
    let inbox = state.insert(());
    state.process_update(inbox).unwrap().workload(&Payload(5));

    // Reconnect reports a higher count — new_mail fires.
    let effects = state.process_update(inbox).unwrap().workload(&Payload(7));
    assert!(effects.new_mail);
    assert_eq!(effects.total_unread_changed, Some(7));
}

#[test]
fn reconnect_same_count_does_not_fire() {
    let mut state = State::<slotmap::DefaultKey, ()>::new();
    let inbox = state.insert(());
    state.process_update(inbox).unwrap().workload(&Payload(5));

    // Reconnect reports the same count — no new_mail, no total change.
    let effects = state.process_update(inbox).unwrap().workload(&Payload(5));
    assert!(!effects.new_mail);
    assert_eq!(effects.total_unread_changed, None);
}

#[test]
fn get_returns_stored_value() {
    let mut state = State::<slotmap::DefaultKey, &str>::new();
    let inbox = state.insert("inbox");
    assert_eq!(
        state.entries().get(inbox).map(|e| &e.user_data),
        Some(&"inbox")
    );
}

#[test]
fn get_mut_modifies_stored_value() {
    let mut state = State::<slotmap::DefaultKey, String>::new();
    let inbox = state.insert("inbox".to_owned());
    if let Some(entry) = state.get_entry_mut(inbox) {
        entry.user_data.push_str("_modified");
    }
    assert_eq!(
        state.entries().get(inbox).map(|e| e.user_data.as_str()),
        Some("inbox_modified"),
    );
}

#[test]
fn iter_yields_all_entries() {
    let mut state = State::<slotmap::DefaultKey, &str>::new();
    let a = state.insert("a");
    let b = state.insert("b");
    state.process_update(a).unwrap().workload(&Payload(3));
    state.process_update(b).unwrap().workload(&Payload(7));

    let items: Vec<(_, Option<u32>, &&str)> = state
        .entries()
        .iter()
        .map(|(k, e)| (k, e.tracked().unread, &e.user_data))
        .collect();
    assert_eq!(items.len(), 2);
    assert!(items.contains(&(a, Some(3), &"a")));
    assert!(items.contains(&(b, Some(7), &"b")));
}

#[test]
fn process_supervisor_started_sets_active() {
    let mut state = State::<slotmap::DefaultKey, ()>::new();
    let inbox = state.insert(());
    assert!(!state.entries().get(inbox).unwrap().tracked().active);

    state
        .process_update(inbox)
        .unwrap()
        .supervisor(&SupervisorEvent::<core::convert::Infallible, ()>::Started);
    assert!(state.entries().get(inbox).unwrap().tracked().active);
}

#[test]
fn process_supervisor_error_clears_active() {
    let mut state = State::<slotmap::DefaultKey, ()>::new();
    let inbox = state.insert(());

    state
        .process_update(inbox)
        .unwrap()
        .supervisor(&SupervisorEvent::<core::convert::Infallible, ()>::Started);
    assert!(state.entries().get(inbox).unwrap().tracked().active);

    state
        .process_update(inbox)
        .unwrap()
        .supervisor(&SupervisorEvent::<core::convert::Infallible, ()>::Error {
            error: (),
            next_retry_in: std::time::Duration::ZERO,
        });
    assert!(!state.entries().get(inbox).unwrap().tracked().active);
}

#[test]
fn process_supervisor_done_clears_active() {
    let mut state = State::<slotmap::DefaultKey, ()>::new();
    let inbox = state.insert(());

    state
        .process_update(inbox)
        .unwrap()
        .supervisor(&SupervisorEvent::<(), ()>::Started);
    assert!(state.entries().get(inbox).unwrap().tracked().active);

    state
        .process_update(inbox)
        .unwrap()
        .supervisor(&SupervisorEvent::<(), ()>::Done { value: () });
    assert!(!state.entries().get(inbox).unwrap().tracked().active);
}

#[test]
fn process_supervisor_panicked_clears_active() {
    let mut state = State::<slotmap::DefaultKey, ()>::new();
    let inbox = state.insert(());

    state
        .process_update(inbox)
        .unwrap()
        .supervisor(&SupervisorEvent::<(), ()>::Started);
    assert!(state.entries().get(inbox).unwrap().tracked().active);

    state
        .process_update(inbox)
        .unwrap()
        .supervisor(&SupervisorEvent::<(), ()>::Panicked {
            panic_payload: Box::new("oops"),
            next_retry_in: std::time::Duration::ZERO,
        });
    assert!(!state.entries().get(inbox).unwrap().tracked().active);
}

#[test]
fn process_update_unknown_key_returns_none() {
    let mut state = State::<slotmap::DefaultKey, ()>::new();
    assert!(
        state
            .process_update(slotmap::DefaultKey::default())
            .is_none()
    );
}

#[test]
fn entries_unknown_key_returns_none() {
    let state = State::<slotmap::DefaultKey, ()>::new();
    assert!(
        state
            .entries()
            .get(slotmap::DefaultKey::default())
            .is_none()
    );
}

#[test]
fn get_entry_mut_unknown_key_returns_none() {
    let mut state = State::<slotmap::DefaultKey, ()>::new();
    assert!(
        state
            .get_entry_mut(slotmap::DefaultKey::default())
            .is_none()
    );
}

#[test]
fn empty_state_defaults() {
    let state = State::<slotmap::DefaultKey, ()>::new();
    assert_eq!(state.total_unread(), 0);
    assert_eq!(state.entries().len(), 0);
    assert_eq!(state.entries().iter().count(), 0);
}
