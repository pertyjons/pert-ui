//! Breadcrumbs: keyed segments that report their key, a current location that
//! is not activatable, middle segments that collapse into a menu at narrow
//! widths, and identity by key when the path changes.
use pert_ui::*;
mod support;
use std::{cell::RefCell, rc::Rc};
use support::{find, press, settle, widget};

type Activated = Rc<RefCell<Vec<String>>>;

const PATH: [(&str, &str); 5] = [
    ("/", "Root"),
    ("/home", "home"),
    ("/home/user", "user"),
    ("/home/user/music", "music"),
    ("/home/user/music/project", "project"),
];

fn ui(width: f32) -> Runtime {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(width, 200.0, 1.0).unwrap())
        .unwrap();
    ui
}

fn breadcrumb(path: &[(&str, &str)], activated: &Activated) -> Breadcrumb {
    let activated = activated.clone();
    Breadcrumb::new(
        path.iter()
            .map(|(key, label)| BreadcrumbSegment::new(*key, *label)),
        move |_, key| {
            activated.borrow_mut().push(key.to_owned());
            Ok(())
        },
    )
    .unwrap()
}

fn mount(ui: &mut Runtime, activated: &Activated) -> (WidgetId, WidgetId) {
    let path = breadcrumb(&PATH, activated);
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([Element::new("path", path)]),
        ))
        .unwrap();
    settle(ui);
    let path = widget(ui, &root, "path");
    (root, path)
}

fn visible(ui: &Runtime, id: &WidgetId) -> bool {
    ui.bounds(id)
        .unwrap()
        .is_some_and(|rect| rect.width() > 0.0)
        && ui
            .inspect()
            .components
            .iter()
            .any(|component| component.key == ui.key(id).unwrap() && component.visible)
}

fn collapsed(ui: &Runtime) -> f64 {
    let component = ui
        .inspect()
        .components
        .into_iter()
        .find(|component| component.key == "path")
        .unwrap();
    match component.details.properties.get("collapsed") {
        Some(InspectValue::Number(count)) => *count,
        other => panic!("collapsed is {other:?}"),
    }
}

fn shown_segments(ui: &Runtime, path: &WidgetId) -> Vec<String> {
    ui.children(path)
        .unwrap()
        .iter()
        .filter(|id| ui.key(id).unwrap() != "overflow")
        .filter(|id| {
            ui.inspect()
                .components
                .iter()
                .any(|c| c.key == ui.key(id).unwrap() && c.visible)
        })
        .map(|id| ui.key(id).unwrap().to_owned())
        .collect()
}

#[test]
fn wide_breadcrumb_shows_every_segment_and_reports_activated_keys() {
    let mut ui = ui(900.0);
    let activated = Activated::default();
    let (root, path) = mount(&mut ui, &activated);
    assert_eq!(collapsed(&ui), 0.0);
    assert_eq!(shown_segments(&ui, &path).len(), PATH.len());
    assert!(!visible(&ui, &widget(&ui, &root, "overflow")));
    // Segments are in path order, left to right.
    let xs: Vec<f32> = PATH
        .iter()
        .map(|(key, _)| ui.bounds(&widget(&ui, &root, key)).unwrap().unwrap().x())
        .collect();
    assert!(xs.windows(2).all(|pair| pair[0] < pair[1]), "{xs:?}");

    ui.focus(&widget(&ui, &root, "/home")).unwrap();
    press(&mut ui, KeyCode::Enter);
    assert_eq!(*activated.borrow(), ["/home"]);

    // A pointer click on another segment reports that key.
    let [x, y, w, h] = ui
        .bounds(&widget(&ui, &root, "/home/user"))
        .unwrap()
        .unwrap()
        .values();
    let center = Point::new(x + w / 2.0, y + h / 2.0).unwrap();
    for kind in [
        PointerKind::Move,
        PointerKind::Press(PointerButton::Primary),
        PointerKind::Release(PointerButton::Primary),
    ] {
        ui.pointer_input(PointerEvent::new(center, kind)).unwrap();
    }
    assert_eq!(*activated.borrow(), ["/home", "/home/user"]);
}

#[test]
fn the_current_segment_is_neither_a_tab_stop_nor_activatable() {
    let mut ui = ui(900.0);
    let activated = Activated::default();
    let (root, _) = mount(&mut ui, &activated);
    let mut stops = Vec::new();
    for _ in 0..PATH.len() {
        press(&mut ui, KeyCode::Tab);
        stops.push(ui.key(&ui.focused().unwrap()).unwrap().to_owned());
    }
    let current = PATH[PATH.len() - 1].0;
    assert!(!stops.iter().any(|key| key == current), "{stops:?}");
    let last = widget(&ui, &root, current);
    let semantics = ui
        .inspect()
        .components
        .into_iter()
        .find(|c| c.key == current)
        .and_then(|c| c.details.semantics)
        .unwrap();
    assert_eq!(semantics.role, SemanticRole::Label);
    assert!(semantics.actions.is_empty());
    // Every other segment is a link to its location.
    let first = ui
        .inspect()
        .components
        .into_iter()
        .find(|c| c.key == PATH[0].0)
        .and_then(|c| c.details.semantics)
        .unwrap();
    assert_eq!(first.role, SemanticRole::Link);
    assert!(first.actions.contains(&SemanticAction::Activate));
    let [x, y, w, h] = ui.bounds(&last).unwrap().unwrap().values();
    let center = Point::new(x + w / 2.0, y + h / 2.0).unwrap();
    for kind in [
        PointerKind::Press(PointerButton::Primary),
        PointerKind::Release(PointerButton::Primary),
    ] {
        ui.pointer_input(PointerEvent::new(center, kind)).unwrap();
    }
    assert!(activated.borrow().is_empty());
}

#[test]
fn narrow_breadcrumb_collapses_the_middle_into_a_menu_that_reports_keys() {
    let mut ui = ui(900.0);
    let activated = Activated::default();
    let (root, path) = mount(&mut ui, &activated);
    let full = ui.bounds(&path).unwrap().unwrap().width();

    // Narrower than the full row: the first and last stay, the middle goes.
    ui.set_viewport(Viewport::new(full * 0.7, 200.0, 1.0).unwrap())
        .unwrap();
    settle(&mut ui);
    let count = collapsed(&ui);
    assert!(count >= 1.0, "collapsed {count}");
    let shown = shown_segments(&ui, &path);
    assert_eq!(shown.first().map(String::as_str), Some("/"));
    assert_eq!(
        shown.last().map(String::as_str),
        Some("/home/user/music/project")
    );
    assert!(!shown.iter().any(|key| key == "/home"), "{shown:?}");
    let overflow = widget(&ui, &root, "overflow");
    assert!(visible(&ui, &overflow));
    let row = ui.bounds(&path).unwrap().unwrap();
    for key in &shown {
        let rect = ui.bounds(&widget(&ui, &root, key)).unwrap().unwrap();
        assert!(
            rect.x() + rect.width() <= row.x() + row.width() + 0.5,
            "{key} overflows"
        );
    }

    // The menu lists the collapsed segments, outermost first, and choosing one
    // reports its key.
    let trigger = find(&ui, &overflow, "button").unwrap();
    ui.focus(&trigger).unwrap();
    press(&mut ui, KeyCode::Enter);
    let rows = ui
        .inspect()
        .components
        .into_iter()
        .find(|c| c.key == "items" && c.visible)
        .and_then(|c| c.details.semantics)
        .unwrap();
    let names: Vec<_> = rows
        .items
        .iter()
        .map(|item| item.name.clone().unwrap())
        .collect();
    assert_eq!(names.first().map(String::as_str), Some("home"));
    assert_eq!(names.len(), count as usize);
    press(&mut ui, KeyCode::Enter);
    assert_eq!(*activated.borrow(), ["/home"]);

    // At the narrowest, every middle segment collapses.
    ui.set_viewport(Viewport::new(40.0, 200.0, 1.0).unwrap())
        .unwrap();
    settle(&mut ui);
    assert_eq!(collapsed(&ui), (PATH.len() - 2) as f64);

    // Widening brings every segment back and hides the button.
    ui.set_viewport(Viewport::new(900.0, 200.0, 1.0).unwrap())
        .unwrap();
    settle(&mut ui);
    assert_eq!(collapsed(&ui), 0.0);
    assert_eq!(shown_segments(&ui, &path).len(), PATH.len());
    assert!(!visible(&ui, &overflow));
}

#[test]
fn segments_keep_identity_and_focus_by_key_when_the_path_changes() {
    let mut ui = ui(900.0);
    let activated = Activated::default();
    let (root, path) = mount(&mut ui, &activated);
    let home = widget(&ui, &root, "/home");
    ui.focus(&home).unwrap();
    settle(&mut ui);
    // Navigate up: the path loses its tail, and a renamed label keeps the key.
    let shorter = [
        ("/", "Root"),
        ("/home", "Home folder"),
        ("/home/user", "user"),
    ];
    ui.reconcile_children(
        &root,
        vec![Element::new("path", breadcrumb(&shorter, &activated))],
    )
    .unwrap();
    settle(&mut ui);
    assert_eq!(widget(&ui, &root, "/home"), home);
    assert_eq!(ui.focused(), Some(home.clone()));
    assert!(
        ui.find_descendant(&path, "/home/user/music")
            .unwrap()
            .is_none()
    );
    // The new last segment is now the current location.
    let user = ui
        .inspect()
        .components
        .into_iter()
        .find(|c| c.key == "/home/user")
        .unwrap();
    assert_eq!(
        user.details.properties.get("current"),
        Some(&InspectValue::Bool(true))
    );
    // The new description's callback is the one that runs.
    let replaced = Activated::default();
    ui.reconcile_children(
        &root,
        vec![Element::new("path", breadcrumb(&shorter, &replaced))],
    )
    .unwrap();
    settle(&mut ui);
    press(&mut ui, KeyCode::Enter);
    assert!(activated.borrow().is_empty());
    assert_eq!(*replaced.borrow(), ["/home"]);
}

#[test]
fn invalid_segment_keys_are_rejected() {
    for (keys, expected) in [
        (vec![" "], LayoutError::BlankKey),
        (vec!["overflow"], LayoutError::ReservedKey),
        (vec!["a", "a"], LayoutError::DuplicateKey),
    ] {
        let segments = keys.iter().map(|key| BreadcrumbSegment::new(*key, "label"));
        assert_eq!(
            Breadcrumb::new(segments, |_, _| Ok(())).err(),
            Some(expected)
        );
    }
    assert!(Breadcrumb::new(Vec::new(), |_, _| Ok(())).is_ok());
    assert!(Breadcrumb::new([BreadcrumbSegment::new("a", "A")], |_, _| Ok(())).is_ok());
}

#[test]
fn a_focused_segment_that_collapses_loses_focus_under_the_hiding_rules() {
    let mut ui = ui(900.0);
    let activated = Activated::default();
    let (root, path) = mount(&mut ui, &activated);
    let home = widget(&ui, &root, "/home");
    ui.focus(&home).unwrap();
    settle(&mut ui);
    let full = ui.bounds(&path).unwrap().unwrap().width();
    ui.set_viewport(Viewport::new(full * 0.5, 200.0, 1.0).unwrap())
        .unwrap();
    settle(&mut ui);
    assert!(!shown_segments(&ui, &path).iter().any(|key| key == "/home"));
    assert_ne!(ui.focused(), Some(home));
}

/// An owner that keeps the current location and describes its breadcrumb
/// again from its own update, as a file browser does after navigating.
struct Location {
    path: Signal<String>,
    shown: Option<String>,
}
impl Component for Location {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let path = cx.read(&self.path.read_only())?;
        if self.shown.as_ref() == Some(&path) {
            return Ok(());
        }
        let segments = PATH
            .iter()
            .filter(|(key, _)| path.starts_with(key))
            .map(|(key, label)| BreadcrumbSegment::new(*key, *label));
        let target = self.path.clone();
        let breadcrumb = Breadcrumb::new(segments, move |_, key| {
            target.set(key.to_owned()).map(|_| ())
        })?;
        cx.request_children(vec![Element::new("path", breadcrumb)]);
        self.shown = Some(path);
        Ok(())
    }
}

#[test]
fn an_owner_navigates_by_describing_the_breadcrumb_again_from_its_update() {
    let mut ui = ui(900.0);
    let path = ui.signal(PATH[PATH.len() - 1].0.to_owned());
    let root = ui
        .mount_tree(Element::new(
            "location",
            Location {
                path: path.clone(),
                shown: None,
            },
        ))
        .unwrap();
    settle(&mut ui);
    let crumbs = widget(&ui, &root, "path");
    assert_eq!(shown_segments(&ui, &crumbs).len(), PATH.len());
    let user = widget(&ui, &root, "/home/user");
    ui.focus(&user).unwrap();
    press(&mut ui, KeyCode::Enter);
    assert_eq!(path.get().unwrap(), "/home/user");
    assert_eq!(
        widget(&ui, &root, "path"),
        crumbs,
        "the breadcrumb is retained"
    );
    assert_eq!(
        shown_segments(&ui, &crumbs),
        ["/", "/home", "/home/user"],
        "the path ends at the chosen segment"
    );
    assert_eq!(widget(&ui, &root, "/home/user"), user);
}

#[test]
fn unnamed_segments_are_rejected_even_when_they_have_an_icon() {
    for label in ["", " ", "\t\n", "\u{2003}"] {
        for with_icon in [false, true] {
            let segment = BreadcrumbSegment::new("root", label);
            let segment = if with_icon {
                segment.leading_icon(Icon::Folder)
            } else {
                segment
            };
            assert_eq!(
                Breadcrumb::new([segment], |_, _| Ok(())).err(),
                Some(LayoutError::BlankLabel)
            );
        }
    }
    assert!(
        Breadcrumb::new(
            vec![BreadcrumbSegment::new("root", " Home ").leading_icon(Icon::Folder)],
            |_, _| Ok(())
        )
        .is_ok()
    );
}

#[test]
fn a_segment_icon_is_a_replaceable_slot_independent_of_the_label() {
    let segment = BreadcrumbSegment::new("/home", "Home")
        .leading_icon(Icon::Settings)
        .leading_icon(Icon::Folder);
    assert_eq!(
        segment.content(),
        RichText::from("Home").leading_icon(Icon::Folder)
    );
    assert_eq!(
        BreadcrumbSegment::new("/home", "Home").content(),
        RichText::from("Home")
    );
}
