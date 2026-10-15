//! Navigation history contracts through the public facade.
use pert_ui::*;
use std::{num::NonZeroUsize, rc::Rc, time::Duration};

fn labels(history: &NavigationHistory<u32>) -> Vec<String> {
    history
        .state()
        .get()
        .unwrap()
        .entries()
        .iter()
        .map(|entry| entry.label().to_owned())
        .collect()
}
fn current(history: &NavigationHistory<u32>) -> Option<u32> {
    history.current().get().unwrap().map(|entry| *entry.value())
}

#[test]
fn an_empty_history_has_nowhere_to_go() {
    let ui = Runtime::new();
    let history = NavigationHistory::<u32>::new(&ui).unwrap();
    assert_eq!(history.capacity().get(), 100);
    assert!(history.state().get().unwrap().entries().is_empty());
    assert_eq!(current(&history), None);
    assert!(!history.can_go_back().get().unwrap());
    assert!(!history.can_go_forward().get().unwrap());
    assert!(history.back().unwrap().is_none());
    assert!(history.forward().unwrap().is_none());
}

#[test]
fn visits_move_back_and_forward_like_a_browser() {
    let ui = Runtime::new();
    let history = NavigationHistory::new(&ui).unwrap();
    assert!(history.visit(1, "one").unwrap());
    assert!(history.visit(2, "two").unwrap());
    assert!(history.visit(3, "three").unwrap());
    assert_eq!(labels(&history), ["one", "two", "three"]);
    assert!(history.can_go_back().get().unwrap());
    assert!(!history.can_go_forward().get().unwrap());

    let back = history.back().unwrap().unwrap();
    assert_eq!((*back.value(), back.label()), (2, "two"));
    assert_eq!(current(&history), Some(2));
    assert!(history.can_go_forward().get().unwrap());
    let state = history.state().get().unwrap();
    assert_eq!(state.previous().map(|e| *e.value()), Some(1));
    assert_eq!(state.next().map(|e| *e.value()), Some(3));

    assert_eq!(*history.back().unwrap().unwrap().value(), 1);
    assert!(
        history.back().unwrap().is_none(),
        "no entry before the first"
    );
    assert_eq!(current(&history), Some(1));
    assert_eq!(*history.forward().unwrap().unwrap().value(), 2);
    assert_eq!(*history.forward().unwrap().unwrap().value(), 3);
    assert!(
        history.forward().unwrap().is_none(),
        "no entry after the last"
    );
    // Moving never adds or reorders entries.
    assert_eq!(labels(&history), ["one", "two", "three"]);
}

#[test]
fn a_visit_drops_the_forward_stack_and_equal_visits_record_nothing() {
    let ui = Runtime::new();
    let history = NavigationHistory::new(&ui).unwrap();
    for (value, label) in [(1, "one"), (2, "two"), (3, "three")] {
        history.visit(value, label).unwrap();
    }
    history.back().unwrap();
    history.back().unwrap();
    // Reporting the restored place again, as an application does after
    // restoring it, records nothing and keeps the forward stack.
    assert!(!history.visit(1, "renamed").unwrap());
    assert_eq!(labels(&history), ["one", "two", "three"]);
    assert!(history.can_go_forward().get().unwrap());
    // A new place drops everything after the current entry.
    assert!(history.visit(4, "four").unwrap());
    assert_eq!(labels(&history), ["one", "four"]);
    assert!(!history.can_go_forward().get().unwrap());
    // An equal value earlier in the history is a separate visit.
    assert!(history.visit(1, "one again").unwrap());
    assert_eq!(labels(&history), ["one", "four", "one again"]);
}

#[test]
fn entries_keep_their_identity_and_stale_identities_never_retarget() {
    let ui = Runtime::new();
    let history = NavigationHistory::new(&ui).unwrap();
    history.visit(1, "one").unwrap();
    history.visit(2, "two").unwrap();
    history.visit(3, "three").unwrap();
    let ids: Vec<_> = history
        .state()
        .get()
        .unwrap()
        .entries()
        .iter()
        .map(HistoryEntry::id)
        .collect();
    assert!(ids.windows(2).all(|pair| pair[0] < pair[1]));
    // Going to an entry moves the position without a new branch.
    let first = history.go_to(ids[0]).unwrap().unwrap();
    assert_eq!(*first.value(), 1);
    assert_eq!(labels(&history), ["one", "two", "three"]);
    assert_eq!(*history.go_to(ids[2]).unwrap().unwrap().value(), 3);
    assert_eq!(*history.go_to(ids[2]).unwrap().unwrap().value(), 3);
    // A dropped entry's identity stops resolving, even though a new entry
    // now stands at its position.
    history.back().unwrap();
    history.visit(9, "nine").unwrap();
    assert!(history.go_to(ids[2]).unwrap().is_none());
    assert_eq!(current(&history), Some(9));
    let new_id = history.current().get().unwrap().unwrap().id();
    assert!(!ids.contains(&new_id), "identities are never reused");
    // Clearing keeps identities unique too.
    history.clear().unwrap();
    assert_eq!(current(&history), None);
    history.visit(1, "one").unwrap();
    assert!(history.current().get().unwrap().unwrap().id() > new_id);
}

#[test]
fn the_capacity_drops_the_oldest_entries() {
    let ui = Runtime::new();
    let history = NavigationHistory::with_capacity(&ui, NonZeroUsize::new(3).unwrap()).unwrap();
    for value in 1..=5 {
        history.visit(value, value.to_string()).unwrap();
    }
    assert_eq!(labels(&history), ["3", "4", "5"]);
    assert_eq!(current(&history), Some(5));
    history.back().unwrap();
    history.back().unwrap();
    assert!(history.back().unwrap().is_none());
    // Visiting from the middle drops the forward stack before the capacity
    // is applied, so nothing older is lost.
    history.visit(6, "6").unwrap();
    assert_eq!(labels(&history), ["3", "6"]);
    let default = NavigationHistory::<u32>::new(&ui).unwrap();
    for value in 0..150 {
        default.visit(value, value.to_string()).unwrap();
    }
    let state = default.state().get().unwrap();
    assert_eq!(state.entries().len(), 100);
    assert_eq!(state.entries()[0].value(), &50);
}

#[test]
fn a_refused_write_leaves_the_history_unchanged() {
    struct Writer(
        NavigationHistory<u32>,
        Rc<std::cell::RefCell<Option<RuntimeError>>>,
    );
    impl Component for Writer {
        fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
            // Writing from update is refused like any signal write.
            let error = self.0.visit(2, "two").unwrap_err();
            *self.1.borrow_mut() = Some(error);
            Ok(())
        }
    }
    let mut ui = Runtime::new();
    let history = NavigationHistory::new(&ui).unwrap();
    history.visit(1, "one").unwrap();
    let seen = Rc::new(std::cell::RefCell::new(None));
    ui.set_viewport(Viewport::new(100.0, 100.0, 1.0).unwrap())
        .unwrap();
    ui.mount_tree(Element::new(
        "writer",
        Writer(history.clone(), seen.clone()),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(*seen.borrow(), Some(RuntimeError::WriteDuringUpdate));
    assert_eq!(labels(&history), ["one"]);
    // The refused visit consumed no identity.
    history.visit(3, "three").unwrap();
    let ids: Vec<_> = history
        .state()
        .get()
        .unwrap()
        .entries()
        .iter()
        .map(|entry| entry.id().get())
        .collect();
    assert_eq!(ids, [1, 2]);
}

fn tab(key: &str) -> Tab {
    Tab::new(key, key, Element::new("body", Label::new(key)))
}

#[test]
fn recording_user_selections_skips_the_history_s_own_navigation() {
    let mut ui = Runtime::new();
    let history = NavigationHistory::<String>::new(&ui).unwrap();
    history.visit("a".to_owned(), "a").unwrap();
    let recorder = history.clone();
    let strip = Tabs::new([tab("a"), tab("b"), tab("c")]).on_select(move |_, change| {
        if change.origin == ChangeOrigin::User
            && let Some(key) = &change.selected
        {
            recorder.visit(key.clone(), key.clone())?;
        }
        Ok(())
    });
    ui.set_viewport(Viewport::new(400.0, 200.0, 1.0).unwrap())
        .unwrap();
    let tabs = ui.mount_tree(Element::new("tabs", strip)).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.focus(&tabs).unwrap();
    for _ in 0..2 {
        ui.key_input(KeyCode::ArrowRight, KeyAction::Press).unwrap();
        ui.key_input(KeyCode::ArrowRight, KeyAction::Release)
            .unwrap();
    }
    let values = |history: &NavigationHistory<String>| -> Vec<String> {
        history
            .state()
            .get()
            .unwrap()
            .entries()
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    };
    assert_eq!(values(&history), ["a", "b", "c"]);
    // Going back two places and restoring each through a reveal records
    // nothing, so the forward stack survives.
    for expected in ["b", "a"] {
        let entry = history.back().unwrap().unwrap();
        assert_eq!(entry.value(), expected);
        ui.reveal_child(&tabs, entry.value()).unwrap();
    }
    assert_eq!(values(&history), ["a", "b", "c"]);
    assert!(history.can_go_forward().get().unwrap());
    // A new user selection then drops the forward stack.
    ui.focus(&tabs).unwrap();
    ui.key_input(KeyCode::ArrowRight, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::ArrowRight, KeyAction::Release)
        .unwrap();
    assert_eq!(values(&history), ["a", "b"]);
    assert!(!history.can_go_forward().get().unwrap());
}

#[test]
fn bound_controls_follow_the_history_without_polling() {
    let mut ui = Runtime::new();
    let history = NavigationHistory::<u32>::new(&ui).unwrap();
    let caption = history
        .state()
        .map(|state| {
            state
                .current()
                .map_or_else(|| "nowhere".to_owned(), |entry| entry.label().to_owned())
        })
        .unwrap();
    ui.set_viewport(Viewport::new(200.0, 100.0, 1.0).unwrap())
        .unwrap();
    let label = ui
        .mount_tree(Element::new("where", Label::new(caption)))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let text = |ui: &Runtime| -> String {
        ui.inspect()
            .components
            .into_iter()
            .find(|item| item.key == "where")
            .and_then(|item| item.details.semantics)
            .and_then(|semantics| semantics.name)
            .unwrap()
    };
    assert_eq!(text(&ui), "nowhere");
    history.visit(1, "Mixer").unwrap();
    history.visit(2, "Editor").unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(text(&ui), "Editor");
    history.back().unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(text(&ui), "Mixer");
    assert!(ui.next_deadline().is_none(), "nothing polls the history");
    ui.unmount(&label).unwrap();
}
