//! The asynchronous source contract: tickets per key, superseded, settled and
//! foreign results, owner removal, repeated and failed delivery.
use pert_ui::*;

fn assert_send<T: Send>(_: &T) {}

#[test]
fn only_the_latest_ticket_per_key_is_current() {
    let tracker = RequestTracker::new();
    let a1 = tracker.issue("a").unwrap();
    let b1 = tracker.issue("b").unwrap();
    let a2 = tracker.issue("a").unwrap();
    assert!(a2.generation() > b1.generation() && b1.generation() > a1.generation());
    assert_eq!(tracker.status(&a1), TicketStatus::Superseded);
    assert_eq!(tracker.status(&a2), TicketStatus::Current);
    // Another key is untouched by a newer ticket for "a".
    assert_eq!(tracker.status(&b1), TicketStatus::Current);
    assert!(tracker.is_pending(&"a") && tracker.is_pending(&"b"));
    // A superseded ticket cannot complete its key's newer request.
    assert!(!tracker.complete(&a1));
    assert_eq!(tracker.status(&a2), TicketStatus::Current);
}

#[test]
fn a_repeated_delivery_is_settled_and_changes_nothing() {
    let tracker = RequestTracker::new();
    let ticket = tracker.issue(7_u32).unwrap();
    assert!(tracker.complete(&ticket));
    assert_eq!(tracker.status(&ticket), TicketStatus::Settled);
    assert!(!tracker.complete(&ticket));
    assert!(!tracker.is_pending(&7));
}

#[test]
fn a_failed_delivery_leaves_the_ticket_current_for_a_retry() {
    let tracker = RequestTracker::new();
    let ticket = tracker.issue("x").unwrap();
    // The component checked the status, but applying failed: it does not
    // complete, so the same result can be delivered again.
    assert_eq!(tracker.status(&ticket), TicketStatus::Current);
    let retry = ticket.clone();
    assert_eq!(tracker.status(&retry), TicketStatus::Current);
    assert!(tracker.complete(&retry));
    assert_eq!(tracker.status(&ticket), TicketStatus::Settled);
}

#[test]
fn cancelling_or_retaining_settles_unwanted_keys() {
    let tracker = RequestTracker::new();
    let a = tracker.issue(1).unwrap();
    let b = tracker.issue(2).unwrap();
    let c = tracker.issue(3).unwrap();
    assert!(tracker.cancel(&1));
    assert!(!tracker.cancel(&1));
    tracker.retain(|key| *key != 2);
    assert_eq!(tracker.status(&a), TicketStatus::Settled);
    assert_eq!(tracker.status(&b), TicketStatus::Settled);
    assert_eq!(tracker.status(&c), TicketStatus::Current);
}

#[test]
fn a_ticket_from_another_or_a_removed_tracker_is_foreign() {
    let first = RequestTracker::new();
    let ticket = first.issue("k").unwrap();
    let second = RequestTracker::new();
    let own = second.issue("k").unwrap();
    // Same key and generation, different owner.
    assert_eq!(ticket.generation(), own.generation());
    assert_ne!(ticket, own);
    assert_eq!(second.status(&ticket), TicketStatus::Foreign);
    assert!(!second.complete(&ticket));
    assert_eq!(second.status(&own), TicketStatus::Current);
    // The owner is removed; a tracker created afterwards still refuses the
    // ticket, which keeps the removed owner's identity alive.
    drop(first);
    let third = RequestTracker::new();
    let _ = third.issue("k").unwrap();
    assert_eq!(third.status(&ticket), TicketStatus::Foreign);
}

#[test]
fn tickets_travel_to_other_threads_and_compare_by_owner_key_and_generation() {
    let tracker = RequestTracker::new();
    let ticket = tracker.issue(String::from("listing")).unwrap();
    assert_send(&ticket);
    let sent = ticket.clone();
    let back = std::thread::spawn(move || sent).join().unwrap();
    assert_eq!(back, ticket);
    assert_eq!(back.key(), "listing");
    assert_eq!(tracker.status(&back), TicketStatus::Current);
}
