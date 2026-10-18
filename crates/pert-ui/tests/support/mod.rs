//! Helpers shared by the facade's integration tests, using the public API only.
//!
//! A test binary's `main.rs` declares it with `#[path = "../support/mod.rs"]
//! mod support;` and its modules opt in with `use crate::support;`. Each binary
//! compiles its own copy and uses only some helpers, so unused ones are allowed
//! rather than warned about per binary.
//!
//! The helpers panic instead of returning errors: they drive a runtime the test
//! itself built, so a failure here is a failed test, and `#[track_caller]`
//! reports the calling line.
#![allow(dead_code)]

use pert_ui::{KeyAction, KeyCode, Runtime, WidgetId};

/// The widget keyed `key` in the subtree rooted at `root`, `root` itself
/// included, or `None`.
///
/// Keys are unique only among siblings, so the lookup never picks one of
/// several matches: it panics when more than one widget in the subtree carries
/// `key`, so a repeated key fails the test instead of silently retargeting it.
/// Search below a uniquely keyed ancestor, or give the widgets distinct keys.
/// Below `root` it is [`Runtime::find_descendant`]. This is the one key lookup
/// the tests share, so every test resolves a key the same way.
///
/// # Panics
/// When several widgets carry `key`, or when `root` is not mounted in `ui`:
/// looking up a key in a tree that is gone is a test failure, not an absent
/// key.
#[track_caller]
pub fn find(ui: &Runtime, root: &WidgetId, key: &str) -> Option<WidgetId> {
    let own = match ui.key(root) {
        Ok(own) => own == key,
        Err(error) => panic!("looking up {key:?}: reading the key of {root:?}: {error}"),
    };
    let below = match ui.find_descendant(root, key) {
        Ok(below) => below,
        Err(error) => panic!("looking up {key:?} below {root:?}: {error}"),
    };
    match (own, below) {
        (true, Some(_)) => panic!("looking up {key:?}: both {root:?} and a descendant carry it"),
        (true, None) => Some(root.clone()),
        (false, below) => below,
    }
}

/// The widget keyed `key` under `root`, as [`find`] looks it up.
///
/// # Panics
/// When no widget has the key, or as [`find`] does.
#[track_caller]
pub fn widget(ui: &Runtime, root: &WidgetId, key: &str) -> WidgetId {
    match find(ui, root, key) {
        Some(id) => id,
        None => panic!("no widget keyed {key:?} under {root:?}"),
    }
}

/// Presses and releases `code`, without flushing.
///
/// # Panics
/// When the runtime rejects either key event.
#[track_caller]
pub fn key(ui: &mut Runtime, code: KeyCode) {
    for action in [KeyAction::Press, KeyAction::Release] {
        if let Err(error) = ui.key_input(code, action) {
            panic!("{code:?} {action:?}: {error}");
        }
    }
}

/// Flushes at the runtime's current time.
///
/// `Runtime::flush` rejects time moving backwards, so a helper that flushed at
/// a fixed time would fail once a test had advanced the clock; this one never
/// advances or rewinds it.
///
/// # Panics
/// When the flush fails.
#[track_caller]
pub fn settle(ui: &mut Runtime) {
    if let Err(error) = ui.flush(ui.now()) {
        panic!("flush: {error}");
    }
}

/// [`settle`], then paints, as a host frame does.
///
/// # Panics
/// When the flush or the paint fails.
#[track_caller]
pub fn frame(ui: &mut Runtime) {
    settle(ui);
    if let Err(error) = ui.paint() {
        panic!("paint: {error}");
    }
}

/// [`key`], then [`settle`], so the effects of the key are visible.
///
/// # Panics
/// As [`key`] and [`settle`] do.
#[track_caller]
pub fn press(ui: &mut Runtime, code: KeyCode) {
    key(ui, code);
    settle(ui);
}
