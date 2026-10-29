//! Pointer press focus around admitted interactive surfaces, through the
//! public component contract and a built-in surface owner.
//!
//! A press focuses the nearest focusable component at or above its target.
//! When focus is already inside an admitted popover, context menu, submenu or
//! modal dialog, the dialog's own root included, and that walk reaches the
//! surface's root, focus stays where it is instead of moving to a focusable
//! ancestor outside: such a move dismissed a popup before its content saw the
//! press, and a window or application dialog refused it with
//! `ModalBlocked`. Focus outside the surface, explicit focus and focus that a
//! handler requests keep their old effect.
use crate::support;
use pert_ui::testing::HostIntegration;
use pert_ui::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use support::{frame, widget};

type Log = Rc<RefCell<Vec<String>>>;
type Slot = Rc<RefCell<Option<WidgetId>>>;

fn rect([x, y, width, height]: [f32; 4]) -> Rect {
    Rect::new(x, y, width, height).unwrap()
}

/// A fixed-size component whose children sit at fixed local rectangles. It
/// records its focus loss and focus-within changes; a leaf also takes and
/// records presses and, on one, asks for focus on the slot's component when
/// one is set.
struct Probe {
    name: &'static str,
    focusable: bool,
    size: [f32; 2],
    places: Vec<[f32; 4]>,
    children: Vec<Element>,
    log: Log,
    request: Option<Slot>,
}
impl Probe {
    fn new(name: &'static str, log: &Log, width: f32, height: f32) -> Self {
        Self {
            name,
            focusable: false,
            size: [width, height],
            places: Vec::new(),
            children: Vec::new(),
            log: log.clone(),
            request: None,
        }
    }
    fn focusable(mut self, focusable: bool) -> Self {
        self.focusable = focusable;
        self
    }
    fn child(mut self, at: [f32; 4], child: Element) -> Self {
        self.places.push(at);
        self.children.push(child);
        self
    }
    fn request(mut self, slot: &Slot) -> Self {
        self.request = Some(slot.clone());
        self
    }
    fn element(self) -> Element {
        Element::new(self.name, self)
    }
}
impl Component for Probe {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn focusable(&self) -> bool {
        self.focusable
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(self.size[0], self.size[1])?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, _: Size) -> Result<(), RuntimeError> {
        for (index, at) in self.places.iter().enumerate().take(cx.len()) {
            cx.place(index, rect(*at))?;
        }
        Ok(())
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        event: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        if !self.places.is_empty() || !matches!(event.kind(), PointerKind::Press(_)) {
            return Ok(InputOutcome::Ignored);
        }
        self.log.borrow_mut().push(format!("{} press", self.name));
        if let Some(target) = self.request.as_ref().and_then(|slot| slot.borrow().clone()) {
            cx.request_focus_for(&target)?;
        }
        Ok(InputOutcome::Handled)
    }
    fn focus_lost(&mut self, _: &mut EventContext) -> Result<(), RuntimeError> {
        self.log
            .borrow_mut()
            .push(format!("{} focus_lost", self.name));
        Ok(())
    }
    fn focus_within_changed(
        &mut self,
        _: &mut EventContext,
        within: bool,
    ) -> Result<(), RuntimeError> {
        self.log
            .borrow_mut()
            .push(format!("{} within {within}", self.name));
        Ok(())
    }
}

/// A custom surface owner: an opener on top and, while its open signal is
/// true, a promoted child of the current kind. A framework dismissal is
/// recorded with its reason and closes the surface.
struct Owner {
    name: &'static str,
    popup: &'static str,
    kind: Rc<Cell<SurfaceKind>>,
    open: Signal<bool>,
    shown: bool,
    size: [f32; 2],
    popup_at: [f32; 4],
    children: Vec<Element>,
    log: Log,
}
impl Component for Owner {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.shown = cx.read(&self.open)?;
        Ok(())
    }
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn child_visible(&self, key: &str) -> bool {
        key != self.popup || self.shown
    }
    fn child_surface(&self, key: &str) -> Option<SurfaceKind> {
        (key == self.popup).then(|| self.kind.get())
    }
    fn dismiss_surface(
        &mut self,
        _: &mut EventContext,
        reason: SurfaceDismissal,
    ) -> Result<(), RuntimeError> {
        self.log
            .borrow_mut()
            .push(format!("{} dismiss {reason:?}", self.name));
        self.shown = false;
        self.open.set(false)
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(self.size[0], self.size[1])?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, _: Size) -> Result<(), RuntimeError> {
        cx.place(0, rect([0.0, 0.0, 100.0, 30.0]))?;
        if cx.len() > 1 {
            cx.place(1, rect(self.popup_at))?;
        }
        Ok(())
    }
}

/// What the surface holds besides the nonfocusable `row`.
#[derive(Clone, Copy, Default)]
struct Content {
    /// A focusable `field`, which admission focuses.
    field: bool,
    /// The `panel` around the content is focusable.
    focusable_panel: bool,
    /// The surface's root `popup` is focusable.
    focusable_root: bool,
    /// A nested `sub` owner whose submenu holds only the nonfocusable
    /// `sub-row`.
    submenu: bool,
}

/// The application side of a stage: what the probes record and the signals
/// and slots the test drives.
struct Parts {
    log: Log,
    open: Signal<bool>,
    sub_open: Signal<bool>,
    kind: Rc<Cell<SurfaceKind>>,
    request: Slot,
}

/// One composed root: a focusable `page` holding the surface `owner`, a
/// focusable `other` and a nonfocusable `note`, none overlapping. The owner
/// is larger for the modal kinds, since a view dialog is laid out inside its
/// owner's allocation.
fn compose(ui: &Runtime, kind: SurfaceKind, content: Content) -> (Element, Parts) {
    let parts = Parts {
        log: Log::default(),
        open: ui.signal(false),
        sub_open: ui.signal(false),
        kind: Rc::new(Cell::new(kind)),
        request: Slot::default(),
    };
    let log = &parts.log;
    let mut panel = Probe::new("panel", log, 160.0, 110.0).focusable(content.focusable_panel);
    if content.field {
        let field = Probe::new("field", log, 150.0, 30.0).focusable(true);
        panel = panel.child([5.0, 5.0, 150.0, 30.0], field.element());
    }
    let row = Probe::new("row", log, 150.0, 30.0).request(&parts.request);
    panel = panel.child([5.0, 40.0, 150.0, 30.0], row.element());
    if content.submenu {
        let sub_popup = Probe::new("sub-popup", log, 120.0, 40.0).child(
            [5.0, 5.0, 110.0, 30.0],
            Probe::new("sub-row", log, 110.0, 30.0).element(),
        );
        let sub = Owner {
            name: "sub",
            popup: "sub-popup",
            kind: Rc::new(Cell::new(SurfaceKind::Submenu)),
            open: parts.sub_open.clone(),
            shown: false,
            size: [150.0, 30.0],
            popup_at: [150.0, 0.0, 120.0, 40.0],
            children: vec![
                Probe::new("sub-opener", log, 100.0, 30.0).element(),
                sub_popup.element(),
            ],
            log: log.clone(),
        };
        panel = panel.child([5.0, 75.0, 150.0, 30.0], Element::new("sub", sub));
    }
    let popup = Probe::new("popup", log, 160.0, 110.0)
        .focusable(content.focusable_root)
        .child([0.0, 0.0, 160.0, 110.0], panel.element());
    let size = if matches!(
        kind,
        SurfaceKind::Dialog | SurfaceKind::ApplicationDialog | SurfaceKind::ViewDialog
    ) {
        [380.0, 200.0]
    } else {
        [120.0, 30.0]
    };
    let owner = Owner {
        name: "owner",
        popup: "popup",
        kind: parts.kind.clone(),
        open: parts.open.clone(),
        shown: false,
        size,
        popup_at: [0.0, 34.0, 160.0, 110.0],
        children: vec![
            Probe::new("opener", log, 100.0, 30.0)
                .focusable(true)
                .element(),
            popup.element(),
        ],
        log: log.clone(),
    };
    let page = Probe::new("page", log, 400.0, 300.0)
        .focusable(true)
        .child([10.0, 10.0, size[0], size[1]], Element::new("owner", owner))
        .child(
            [10.0, 260.0, 100.0, 30.0],
            Probe::new("other", log, 100.0, 30.0)
                .focusable(true)
                .element(),
        )
        .child(
            [200.0, 260.0, 100.0, 30.0],
            Probe::new("note", log, 100.0, 30.0).element(),
        );
    (page.element(), parts)
}

struct Stage {
    ui: Runtime,
    root: WidgetId,
    p: Parts,
}
impl Stage {
    fn new(kind: SurfaceKind, content: Content) -> Self {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(400.0, 300.0, 1.0).unwrap())
            .unwrap();
        let (page, p) = compose(&ui, kind, content);
        let root = ui.mount_tree(page).unwrap();
        frame(&mut ui);
        Self { ui, root, p }
    }
    fn id(&self, key: &str) -> WidgetId {
        widget(&self.ui, &self.root, key)
    }
    /// Focuses the opener, then opens the surface.
    fn open(&mut self) {
        let opener = self.id("opener");
        self.ui.focus(&opener).unwrap();
        self.p.open.set(true).unwrap();
        frame(&mut self.ui);
        assert!(self.is_open(), "the surface opened");
    }
    fn is_open(&self) -> bool {
        self.p.open.get().unwrap()
    }
    fn focus(&self) -> Option<String> {
        focus_key(&self.ui)
    }
    fn center(&self, key: &str) -> Point {
        center(&self.ui, &self.id(key))
    }
    fn press(&mut self, at: Point) -> Result<InputOutcome, RuntimeError> {
        self.ui.pointer_input(PointerEvent::new(
            at,
            PointerKind::Press(PointerButton::Primary),
        ))
    }
    fn release(&mut self, at: Point) -> Result<InputOutcome, RuntimeError> {
        self.ui.pointer_input(PointerEvent::new(
            at,
            PointerKind::Release(PointerButton::Primary),
        ))
    }
    /// Clears the log, so a test can assert everything one gesture records.
    fn mark(&self) {
        self.p.log.borrow_mut().clear();
    }
    fn log(&self) -> Vec<String> {
        self.p.log.borrow().clone()
    }
}

fn focus_key(ui: &Runtime) -> Option<String> {
    ui.focused().map(|id| ui.key(&id).unwrap().to_owned())
}

fn center(ui: &Runtime, id: &WidgetId) -> Point {
    let b = ui.bounds(id).unwrap().expect("laid out");
    Point::new(b.x() + b.width() / 2.0, b.y() + b.height() / 2.0).unwrap()
}

const NONMODAL: [SurfaceKind; 3] = [
    SurfaceKind::Popover,
    SurfaceKind::ContextMenu,
    SurfaceKind::Submenu,
];
const MODAL: [SurfaceKind; 3] = [
    SurfaceKind::Dialog,
    SurfaceKind::ApplicationDialog,
    SurfaceKind::ViewDialog,
];
const WITH_FIELD: Content = Content {
    field: true,
    focusable_panel: false,
    focusable_root: false,
    submenu: false,
};

/// What one press and release on the `row` did.
#[derive(Debug, PartialEq)]
struct Pressed {
    kind: SurfaceKind,
    outcome: Result<InputOutcome, RuntimeError>,
    /// Focus right after the press.
    pressed: Option<String>,
    /// Focus after the release and a frame.
    focus: Option<String>,
    open: bool,
    /// Everything the probes and owners recorded, from the press on.
    log: Vec<String>,
}
impl Pressed {
    /// Focus that stayed on `focus` and a surface that stayed open, with
    /// nothing recorded but the row's own press.
    fn kept(kind: SurfaceKind, focus: &str) -> Self {
        Self {
            kind,
            outcome: Ok(InputOutcome::Handled),
            pressed: Some(focus.to_owned()),
            focus: Some(focus.to_owned()),
            open: true,
            log: vec!["row press".to_owned()],
        }
    }
}

/// Opens a stage of `kind`, checks where admission put focus, then presses
/// and releases the row. Every kind is observed before a test compares them,
/// so a failure shows each kind's actual result.
fn press_row(kind: SurfaceKind, content: Content, admitted: &str) -> (Stage, Pressed) {
    let mut s = Stage::new(kind, content);
    s.open();
    assert_eq!(s.focus().as_deref(), Some(admitted), "{kind:?}");
    let row = s.center("row");
    s.mark();
    let outcome = s.press(row);
    let pressed = s.focus();
    s.release(row).unwrap();
    frame(&mut s.ui);
    let observed = Pressed {
        kind,
        outcome,
        pressed,
        focus: s.focus(),
        open: s.is_open(),
        log: s.log(),
    };
    (s, observed)
}

// ---- Focus inside an admitted surface stays there ----

#[test]
fn a_press_on_nonfocusable_popup_content_keeps_focus_and_the_popup() {
    // Formerly the walk focused `page`, which dismissed the popup with
    // `FocusLeft` before the row saw its press. Now there is no focus loss,
    // no focus-within loss on the panel or the root and no dismissal.
    let (stages, observed): (Vec<_>, Vec<_>) = NONMODAL
        .into_iter()
        .map(|kind| press_row(kind, WITH_FIELD, "field"))
        .unzip();
    let expected: Vec<_> = NONMODAL
        .into_iter()
        .map(|kind| Pressed::kept(kind, "field"))
        .collect();
    assert_eq!(observed, expected);
    for mut s in stages {
        let kind = s.p.kind.get();
        // An outside press still dismisses, is consumed and restores the
        // opener.
        let note = s.center("note");
        s.mark();
        assert_eq!(s.press(note), Ok(InputOutcome::Handled), "{kind:?}");
        assert_eq!(s.release(note), Ok(InputOutcome::Handled), "{kind:?}");
        frame(&mut s.ui);
        assert!(!s.is_open(), "{kind:?}");
        assert_eq!(s.focus().as_deref(), Some("opener"), "{kind:?}");
        assert!(
            s.log().contains(&"owner dismiss OutsidePress".to_owned()),
            "{kind:?}: {:?}",
            s.log()
        );
        assert!(!s.log().contains(&"note press".to_owned()), "{kind:?}");
    }
}

#[test]
fn a_press_inside_a_dialog_keeps_focus_and_returns_its_outcome() {
    // Formerly a window or application dialog refused the walk's `page` with
    // `ModalBlocked`, returned after the row's press, and a view dialog let
    // focus leave for `page`.
    let observed: Vec<_> = MODAL
        .into_iter()
        .map(|kind| press_row(kind, WITH_FIELD, "field").1)
        .collect();
    let expected: Vec<_> = MODAL
        .into_iter()
        .map(|kind| Pressed::kept(kind, "field"))
        .collect();
    assert_eq!(observed, expected);
}

#[test]
fn a_dialog_without_focusable_content_keeps_focus_on_its_root() {
    // Admission focuses the root itself when nothing inside can be.
    let observed: Vec<_> = MODAL
        .into_iter()
        .map(|kind| press_row(kind, Content::default(), "popup").1)
        .collect();
    let expected: Vec<_> = MODAL
        .into_iter()
        .map(|kind| Pressed::kept(kind, "popup"))
        .collect();
    assert_eq!(observed, expected);
}

#[test]
fn a_press_on_a_submenu_keeps_focus_in_the_enclosing_menu() {
    for kind in [SurfaceKind::ContextMenu, SurfaceKind::Popover] {
        let content = Content {
            submenu: true,
            ..WITH_FIELD
        };
        let mut s = Stage::new(kind, content);
        s.open();
        s.p.sub_open.set(true).unwrap();
        frame(&mut s.ui);
        // The submenu has nothing focusable, so focus stays in the menu.
        assert_eq!(s.focus().as_deref(), Some("field"), "{kind:?}");
        let sub_row = s.center("sub-row");
        s.mark();
        // The submenu's root holds no focus and is passed; the enclosing
        // menu's root holds it and stops the walk.
        assert_eq!(s.press(sub_row), Ok(InputOutcome::Handled), "{kind:?}");
        s.release(sub_row).unwrap();
        frame(&mut s.ui);
        assert_eq!(s.focus().as_deref(), Some("field"), "{kind:?}");
        assert!(s.is_open() && s.p.sub_open.get().unwrap(), "{kind:?}");
        assert_eq!(s.log(), ["sub-row press"], "{kind:?}");
        // A press on the menu's own content closes the submenu, as an
        // outside press of it, and keeps focus in the menu.
        let row = s.center("row");
        s.mark();
        assert_eq!(s.press(row), Ok(InputOutcome::Handled), "{kind:?}");
        s.release(row).unwrap();
        frame(&mut s.ui);
        assert_eq!(s.focus().as_deref(), Some("field"), "{kind:?}");
        assert!(s.is_open() && !s.p.sub_open.get().unwrap(), "{kind:?}");
        assert_eq!(
            s.log(),
            ["sub dismiss OutsidePress", "row press"],
            "{kind:?}"
        );
    }
}

// ---- The walk is otherwise unchanged ----

#[test]
fn a_focusable_component_inside_the_surface_still_takes_the_press() {
    for kind in [SurfaceKind::Popover, SurfaceKind::Dialog] {
        for (content, taker, left) in [
            (
                Content {
                    focusable_panel: true,
                    ..WITH_FIELD
                },
                "panel",
                None,
            ),
            (
                Content {
                    focusable_root: true,
                    ..WITH_FIELD
                },
                "popup",
                Some("panel within false"),
            ),
        ] {
            let mut s = Stage::new(kind, content);
            s.open();
            let field = s.id("field");
            s.ui.focus(&field).unwrap();
            frame(&mut s.ui);
            let row = s.center("row");
            s.mark();
            assert_eq!(s.press(row), Ok(InputOutcome::Handled), "{kind:?} {taker}");
            s.release(row).unwrap();
            frame(&mut s.ui);
            assert_eq!(s.focus().as_deref(), Some(taker), "{kind:?}");
            assert!(s.is_open(), "{kind:?} {taker}");
            // Focus moving up to the root leaves the panel.
            let expected: Vec<_> = ["field focus_lost"]
                .into_iter()
                .chain(left)
                .chain(["row press"])
                .collect();
            assert_eq!(s.log(), expected, "{kind:?} {taker}");
        }
    }
}

#[test]
fn a_press_on_content_holding_focus_keeps_it_inside_and_outside_surfaces() {
    let mut s = Stage::new(SurfaceKind::Popover, WITH_FIELD);
    // The owner holds the focused opener; its right edge is the owner itself.
    let opener = s.id("opener");
    s.ui.focus(&opener).unwrap();
    frame(&mut s.ui);
    let owner = s.ui.bounds(&s.id("owner")).unwrap().unwrap();
    let edge = Point::new(owner.x() + owner.width() - 5.0, owner.y() + 15.0).unwrap();
    s.mark();
    assert_eq!(s.press(edge), Ok(InputOutcome::Ignored));
    s.release(edge).unwrap();
    frame(&mut s.ui);
    assert_eq!(s.focus().as_deref(), Some("opener"));
    assert!(s.log().is_empty(), "{:?}", s.log());
    // Inside the surface, the panel holding the focused field, below the rows.
    s.open();
    let panel = s.ui.bounds(&s.id("panel")).unwrap().unwrap();
    let below = Point::new(panel.x() + 80.0, panel.y() + panel.height() - 5.0).unwrap();
    s.mark();
    assert_eq!(s.press(below), Ok(InputOutcome::Ignored));
    s.release(below).unwrap();
    frame(&mut s.ui);
    assert_eq!(s.focus().as_deref(), Some("field"));
    assert!(s.is_open());
    assert!(s.log().is_empty(), "{:?}", s.log());
}

#[test]
fn without_a_surface_a_press_still_focuses_the_ancestor() {
    let mut s = Stage::new(SurfaceKind::Popover, WITH_FIELD);
    let opener = s.id("opener");
    s.ui.focus(&opener).unwrap();
    frame(&mut s.ui);
    let note = s.center("note");
    s.mark();
    assert_eq!(s.press(note), Ok(InputOutcome::Handled));
    s.release(note).unwrap();
    frame(&mut s.ui);
    assert_eq!(s.focus().as_deref(), Some("page"));
    assert_eq!(s.log(), ["opener focus_lost", "note press"]);
}

/// The documented limit: an admitted popup with nothing focusable leaves
/// focus on its opener, outside the surface, so a press on its content still
/// takes the old walk out of it and dismisses it.
#[test]
fn focus_already_outside_the_surface_takes_the_old_walk() {
    for kind in NONMODAL {
        let mut s = Stage::new(kind, Content::default());
        s.open();
        assert_eq!(s.focus().as_deref(), Some("opener"), "{kind:?}");
        let row = s.center("row");
        s.mark();
        s.press(row).unwrap();
        s.release(row).unwrap();
        frame(&mut s.ui);
        assert_eq!(s.focus().as_deref(), Some("page"), "{kind:?}");
        assert!(!s.is_open(), "{kind:?}");
        let log = s.log();
        assert!(
            log.contains(&"opener focus_lost".to_owned())
                && log.contains(&"owner dismiss FocusLeft".to_owned()),
            "{kind:?}: {log:?}"
        );
    }
}

#[test]
fn explicit_and_requested_focus_still_leave_and_dismiss_the_popup() {
    // A handler's focus request made during the press wins over the kept focus.
    let mut s = Stage::new(SurfaceKind::Popover, WITH_FIELD);
    s.open();
    *s.p.request.borrow_mut() = Some(s.id("other"));
    let row = s.center("row");
    s.mark();
    assert_eq!(s.press(row), Ok(InputOutcome::Handled));
    s.release(row).unwrap();
    frame(&mut s.ui);
    assert_eq!(s.focus().as_deref(), Some("other"));
    assert!(!s.is_open());
    let log = s.log();
    assert_eq!(
        log.first().map(String::as_str),
        Some("row press"),
        "{log:?}"
    );
    assert!(log.contains(&"field focus_lost".to_owned()), "{log:?}");
    assert!(
        log.contains(&"owner dismiss FocusLeft".to_owned()),
        "{log:?}"
    );
    // So does explicit focus from the application.
    let mut s = Stage::new(SurfaceKind::Popover, WITH_FIELD);
    s.open();
    let other = s.id("other");
    s.mark();
    s.ui.focus(&other).unwrap();
    frame(&mut s.ui);
    assert_eq!(s.focus().as_deref(), Some("other"));
    assert!(!s.is_open());
    assert!(s.log().contains(&"owner dismiss FocusLeft".to_owned()));
}

// ---- Only a current admission counts ----

#[test]
fn a_hidden_or_closed_surface_keeps_nothing_for_a_later_press() {
    for hide_owner in [false, true] {
        let mut s = Stage::new(SurfaceKind::Popover, WITH_FIELD);
        s.open();
        let row = s.center("row");
        // Closed or hidden before the press, with no flush in between: the
        // press settles that first, then hits the page under the old row.
        if hide_owner {
            let owner = s.id("owner");
            s.ui.set_visible(&owner, false).unwrap();
        } else {
            s.p.open.set(false).unwrap();
        }
        s.mark();
        assert_eq!(s.press(row), Ok(InputOutcome::Ignored), "{hide_owner}");
        s.release(row).unwrap();
        frame(&mut s.ui);
        assert!(!s.is_open(), "{hide_owner}");
        assert_eq!(s.focus().as_deref(), Some("page"), "{hide_owner}");
        assert!(!s.log().contains(&"row press".to_owned()), "{hide_owner}");
    }
}

#[test]
fn a_surface_changed_to_a_notification_no_longer_keeps_focus() {
    let mut s = Stage::new(SurfaceKind::Popover, WITH_FIELD);
    s.open();
    let owner = s.id("owner");
    s.p.kind.set(SurfaceKind::Notification);
    s.ui.invalidate(&owner, Invalidation::Layout).unwrap();
    frame(&mut s.ui);
    // Readmitted as a notification, which takes no focus: the field keeps it.
    assert_eq!(s.focus().as_deref(), Some("field"));
    let row = s.center("row");
    s.mark();
    assert_eq!(s.press(row), Ok(InputOutcome::Handled));
    s.release(row).unwrap();
    frame(&mut s.ui);
    assert_eq!(s.focus().as_deref(), Some("page"));
    assert_eq!(
        s.log(),
        [
            "field focus_lost",
            "panel within false",
            "popup within false",
            "row press"
        ]
    );
    // Changed again to a dialog, it is admitted afresh and keeps focus.
    s.p.kind.set(SurfaceKind::Dialog);
    s.ui.invalidate(&owner, Invalidation::Layout).unwrap();
    frame(&mut s.ui);
    assert_eq!(s.focus().as_deref(), Some("field"));
    let row = s.center("row");
    s.mark();
    assert_eq!(s.press(row), Ok(InputOutcome::Handled));
    s.release(row).unwrap();
    frame(&mut s.ui);
    assert_eq!(s.focus().as_deref(), Some("field"));
    assert_eq!(s.log(), ["row press"]);
}

#[test]
fn a_detached_window_keeps_focus_inside_its_surface() {
    let kinds = [SurfaceKind::Popover, SurfaceKind::Dialog];
    let observed: Vec<_> = kinds
        .into_iter()
        .map(|kind| {
            let mut ui = Runtime::new();
            ui.set_viewport(Viewport::new(400.0, 300.0, 1.0).unwrap())
                .unwrap();
            ui.mount_tree(Element::new("main", Label::new("Main")))
                .unwrap();
            let (page, p) = compose(&ui, kind, WITH_FIELD);
            let root = ui.mount_tree(page).unwrap();
            let prepared = ui
                .prepare_detach(&root, NativeWindowOptions::new("Detached").unwrap())
                .unwrap();
            let window = ui.commit_detach(prepared).unwrap();
            ui.select_window(&window).unwrap();
            ui.set_viewport(Viewport::new(400.0, 300.0, 1.0).unwrap())
                .unwrap();
            frame(&mut ui);
            let opener = widget(&ui, &root, "opener");
            ui.focus(&opener).unwrap();
            p.open.set(true).unwrap();
            frame(&mut ui);
            assert_eq!(focus_key(&ui).as_deref(), Some("field"), "{kind:?}");
            let row = center(&ui, &widget(&ui, &root, "row"));
            p.log.borrow_mut().clear();
            let press = PointerKind::Press(PointerButton::Primary);
            let outcome = ui.pointer_input(PointerEvent::new(row, press));
            let pressed = focus_key(&ui);
            let release = PointerKind::Release(PointerButton::Primary);
            ui.pointer_input(PointerEvent::new(row, release)).unwrap();
            frame(&mut ui);
            Pressed {
                kind,
                outcome,
                pressed,
                focus: focus_key(&ui),
                open: p.open.get().unwrap(),
                log: p.log.borrow().clone(),
            }
        })
        .collect();
    let expected: Vec<_> = kinds
        .into_iter()
        .map(|kind| Pressed::kept(kind, "field"))
        .collect();
    assert_eq!(observed, expected);
}

// ---- A short MultiSelect inside a scrolling page ----

/// One composed root: an overflowing scroll area, which is focusable, whose
/// form holds a six-option chooser and filler lines.
struct Chooser {
    ui: Runtime,
    root: WidgetId,
    value: Signal<Vec<String>>,
    reports: Rc<RefCell<Vec<Vec<String>>>>,
}
impl Chooser {
    fn new(commit: MultiSelectCommit) -> Self {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(480.0, 560.0, 1.0).unwrap())
            .unwrap();
        let value = ui.signal(Vec::new());
        let reports = Rc::new(RefCell::new(Vec::new()));
        let recorded = reports.clone();
        let options: Vec<_> = (1..=6)
            .map(|n| Choice::new(format!("o{n:02}"), format!("O{n:02}")))
            .collect();
        let chooser = MultiSelect::new(&ui, value.clone(), options)
            .unwrap()
            .commit(commit)
            .on_change(move |_, values| {
                recorded.borrow_mut().push(values.to_vec());
                Ok(())
            });
        let filler =
            Column::new((0..40).map(|n| Element::new(format!("line{n}"), Label::new("Filler"))));
        let form = Column::new([
            Element::new("tags", chooser),
            Element::new("filler", filler),
        ]);
        let root = ui
            .mount_tree(Element::new(
                "page",
                ScrollArea::new(Element::new("form", form)),
            ))
            .unwrap();
        frame(&mut ui);
        Self {
            ui,
            root,
            value,
            reports,
        }
    }
    fn id(&self, key: &str) -> WidgetId {
        widget(&self.ui, &self.root, key)
    }
    fn property(&self, id: &WidgetId, name: &str) -> Option<InspectValue> {
        let id = self.ui.inspection_id(id).unwrap();
        self.ui
            .inspect()
            .components
            .into_iter()
            .find(|c| c.id == id)
            .and_then(|c| c.details.properties.get(name).cloned())
    }
    fn is_open(&self) -> bool {
        self.property(&self.id("tags"), "open") == Some(InspectValue::Bool(true))
    }
    /// The search field's text, as its semantics report it.
    fn query(&self) -> Option<SemanticValue> {
        let id = self.ui.inspection_id(&self.id("search")).unwrap();
        self.ui
            .inspect()
            .components
            .into_iter()
            .find(|c| c.id == id)
            .and_then(|c| c.details.semantics)
            .and_then(|semantics| semantics.value)
    }
    /// The visible row labelled `label`.
    fn row(&self, label: &str) -> WidgetId {
        let key = self
            .ui
            .inspect()
            .components
            .into_iter()
            .find(|c| {
                c.visible
                    && c.details.properties.contains_key("checked")
                    && c.details.properties.get("text")
                        == Some(&InspectValue::Text(label.to_owned()))
            })
            .map(|c| c.key)
            .unwrap_or_else(|| panic!("no row {label}"));
        self.ui
            .find_descendant(&self.id("list"), &key)
            .unwrap()
            .unwrap()
    }
    fn highlighted(&self) -> Option<String> {
        self.ui
            .inspect()
            .components
            .into_iter()
            .filter(|c| c.visible && c.details.properties.contains_key("checked"))
            .find(|c| c.details.properties["highlighted"] == InspectValue::Bool(true))
            .and_then(|c| match c.details.properties.get("text") {
                Some(InspectValue::Text(text)) => Some(text.clone()),
                _ => None,
            })
    }
    fn pointer(&mut self, at: Point, kind: PointerKind) {
        self.ui.pointer_input(PointerEvent::new(at, kind)).unwrap();
        frame(&mut self.ui);
    }
    fn key(&mut self, code: KeyCode) {
        support::key(&mut self.ui, code);
        frame(&mut self.ui);
    }
    fn value(&self) -> Vec<String> {
        self.value.get().unwrap()
    }
    fn reports(&self) -> Vec<Vec<String>> {
        self.reports.borrow().clone()
    }
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

/// What the row click did in one commit mode.
#[derive(Debug, PartialEq)]
struct Clicked {
    commit: MultiSelectCommit,
    /// Focus, open state and bound value right after the press.
    pressed: (Option<String>, bool, Vec<String>),
    /// The same after the release.
    released: (Option<String>, bool, Vec<String>),
    reports: Vec<Vec<String>>,
}

/// The verified case: an overflowing page is a focusable ancestor of the
/// popup, and a short list is not focusable. A row press formerly focused
/// the page, closed the popup and lost the click.
#[test]
fn a_short_multi_select_in_a_scrolling_page_keeps_its_row_click() {
    use MultiSelectCommit::{EachToggle, OnClose};
    let commits = [EachToggle, OnClose];
    let bound = |commit, chosen: &[&str]| match commit {
        EachToggle => strings(chosen),
        _ => Vec::new(),
    };
    let mut choosers = Vec::new();
    let mut observed = Vec::new();
    for commit in commits {
        let mut c = Chooser::new(commit);
        let anchor = c.id("anchor");
        c.ui.focus(&anchor).unwrap();
        frame(&mut c.ui);
        c.key(KeyCode::Enter);
        assert!(c.is_open(), "{commit:?}");
        assert_eq!(focus_key(&c.ui).as_deref(), Some("search"), "{commit:?}");
        // A keyboard toggle first leaves an OnClose draft that a departure of
        // focus would commit at the press.
        c.key(KeyCode::Enter);
        assert_eq!(c.value(), bound(commit, &["o01"]), "{commit:?}");
        // The pointer owns the highlight before the press, so a lost click
        // cannot come from a stale keyboard highlight.
        let at = center(&c.ui, &c.row("O03"));
        c.pointer(at, PointerKind::Move);
        assert_eq!(c.highlighted().as_deref(), Some("O03"), "{commit:?}");
        c.pointer(at, PointerKind::Press(PointerButton::Primary));
        let pressed = (focus_key(&c.ui), c.is_open(), c.value());
        c.pointer(at, PointerKind::Release(PointerButton::Primary));
        let released = (focus_key(&c.ui), c.is_open(), c.value());
        observed.push(Clicked {
            commit,
            pressed,
            released,
            reports: c.reports(),
        });
        choosers.push(c);
    }
    let search = || Some("search".to_owned());
    let expected: Vec<_> = commits
        .into_iter()
        .map(|commit| Clicked {
            commit,
            pressed: (search(), true, bound(commit, &["o01"])),
            released: (search(), true, bound(commit, &["o01", "o03"])),
            reports: match commit {
                EachToggle => vec![strings(&["o01"]), strings(&["o01", "o03"])],
                _ => Vec::new(),
            },
        })
        .collect();
    assert_eq!(observed, expected);
    for (commit, mut c) in commits.into_iter().zip(choosers) {
        // The keyboard carries on from the clicked row in the search field.
        c.key(KeyCode::ArrowDown);
        assert_eq!(c.highlighted().as_deref(), Some("O04"), "{commit:?}");
        c.key(KeyCode::Enter);
        assert_eq!(
            c.value(),
            bound(commit, &["o01", "o03", "o04"]),
            "{commit:?}"
        );
        c.ui.text_input("O0").unwrap();
        frame(&mut c.ui);
        assert_eq!(
            c.query(),
            Some(SemanticValue::Text("O0".into())),
            "{commit:?}"
        );
        let typed = match commit {
            EachToggle => 3,
            _ => 0,
        };
        assert_eq!(c.reports().len(), typed, "{commit:?}");
        // An outside press closes it; OnClose writes its draft once, there.
        let outside = Point::new(470.0, 550.0).unwrap();
        c.pointer(outside, PointerKind::Press(PointerButton::Primary));
        c.pointer(outside, PointerKind::Release(PointerButton::Primary));
        assert!(!c.is_open(), "{commit:?}");
        let chosen = strings(&["o01", "o03", "o04"]);
        assert_eq!(c.value(), chosen, "{commit:?}");
        let reports = c.reports();
        assert_eq!(reports.last(), Some(&chosen), "{commit:?}");
        let expected = if commit == EachToggle { 3 } else { 1 };
        assert_eq!(reports.len(), expected, "{commit:?}");
    }
}
