//! Scrollbar thumbs are translucent at rest and more opaque while the pointer
//! is over their bar or dragging it, through the facade, for every component
//! that draws the shared overlay scrollbar.
use pert_ui::*;
use std::time::Duration;

/// Thumb rectangles in the current scene, as `(resting, emphasized)`.
fn thumbs(ui: &mut Runtime) -> (Vec<Rect>, Vec<Rect>) {
    let palette = Palette::dark();
    let (rest, hover) = (
        palette.color(ColorRole::ScrollbarThumb),
        palette.color(ColorRole::ScrollbarThumbHover),
    );
    let scene = ui.paint().unwrap();
    let mut resting = Vec::new();
    let mut emphasized = Vec::new();
    for command in scene.commands() {
        if let DrawCommand::FillRect { rect, color } = command {
            if *color == rest {
                resting.push(*rect);
            } else if *color == hover {
                emphasized.push(*rect);
            }
        }
    }
    (resting, emphasized)
}

fn centre(rect: Rect) -> Point {
    Point::new(
        rect.x() + rect.width() / 2.0,
        rect.y() + rect.height() / 2.0,
    )
    .unwrap()
}

fn pointer(ui: &mut Runtime, at: Point, kind: PointerKind) {
    ui.pointer_input(PointerEvent::new(at, kind)).unwrap();
    ui.flush(ui.now()).unwrap();
}

/// Hovering the one resting thumb emphasizes it, and moving away rests it.
fn hover_emphasizes(ui: &mut Runtime, away: Point) {
    let (resting, emphasized) = thumbs(ui);
    assert_eq!(resting.len(), 1, "one resting thumb: {resting:?}");
    assert!(emphasized.is_empty());
    let thumb = resting[0];
    pointer(ui, centre(thumb), PointerKind::Move);
    assert_eq!(thumbs(ui), (vec![], vec![thumb]), "hovered");
    pointer(ui, away, PointerKind::Move);
    assert_eq!(thumbs(ui), (vec![thumb], vec![]), "rested again");
}

#[test]
fn the_built_in_palettes_make_the_thumb_translucent_and_more_opaque_on_hover() {
    for palette in [Palette::dark(), Palette::light()] {
        let rest = palette.color(ColorRole::ScrollbarThumb).alpha();
        let hover = palette.color(ColorRole::ScrollbarThumbHover).alpha();
        assert!(rest > 0.0 && rest < hover && hover < 1.0, "{rest} {hover}");
    }
}

#[test]
fn a_scroll_area_thumb_is_emphasized_over_its_bar_and_while_dragged() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 100.0, 1.0).unwrap())
        .unwrap();
    ui.mount_tree(Element::new(
        "scroll",
        ScrollArea::new(Element::new(
            "content",
            Container::new(Element::new("button", Button::new("Action", |_| Ok(()))))
                .size_limits(SizeLimits::at_least(Size::from([180, 400]))),
        )),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    // The button sits at the top left, and takes the pointer itself.
    let button = Point::new(20.0, 15.0).unwrap();
    hover_emphasizes(&mut ui, button);

    let (resting, _) = thumbs(&mut ui);
    let thumb = resting[0];
    pointer(&mut ui, centre(thumb), PointerKind::Move);
    // The wheel scrolls, and the pointer is still over the bar.
    pointer(
        &mut ui,
        centre(thumb),
        PointerKind::Scroll(Point::new(0.0, 20.0).unwrap()),
    );
    assert_eq!(thumbs(&mut ui).1.len(), 1, "the wheel keeps the emphasis");

    // Dragging keeps it wherever the pointer goes, until the release.
    let (_, emphasized) = thumbs(&mut ui);
    pointer(
        &mut ui,
        centre(emphasized[0]),
        PointerKind::Press(PointerButton::Primary),
    );
    pointer(&mut ui, Point::new(20.0, 60.0).unwrap(), PointerKind::Move);
    assert_eq!(thumbs(&mut ui).1.len(), 1, "a drag keeps the emphasis");
    pointer(
        &mut ui,
        Point::new(20.0, 60.0).unwrap(),
        PointerKind::Release(PointerButton::Primary),
    );
    assert_eq!(thumbs(&mut ui).0.len(), 1, "released away from the bar");

    // Leaving the window rests it too.
    let (resting, _) = thumbs(&mut ui);
    pointer(&mut ui, centre(resting[0]), PointerKind::Move);
    assert_eq!(thumbs(&mut ui).1.len(), 1);
    ui.pointer_input(
        PointerEvent::new(Point::new(250.0, 50.0).unwrap(), PointerKind::Move).outside(),
    )
    .unwrap();
    ui.flush(ui.now()).unwrap();
    assert_eq!(thumbs(&mut ui).1.len(), 0);
}

struct Rows(usize);
impl ListItems for Rows {
    fn len(&self) -> usize {
        self.0
    }
    fn key(&self, index: usize) -> Option<String> {
        (index < self.0).then(|| format!("row-{index}"))
    }
    fn build(&self, index: usize) -> Option<Element> {
        Some(Element::new(
            self.key(index)?,
            Label::new(format!("Row {index}")),
        ))
    }
}

#[test]
fn a_list_thumb_is_emphasized_over_its_bar() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 100.0, 1.0).unwrap())
        .unwrap();
    let rows = ui.signal(ListSource::new(Rows(50)));
    ui.mount_tree(Element::new(
        "list",
        VirtualList::new(rows.read_only(), 24.0).unwrap(),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    hover_emphasizes(&mut ui, Point::new(20.0, 10.0).unwrap());
}

struct Numbers;
impl TableItems for Numbers {
    fn len(&self) -> usize {
        50
    }
    fn key(&self, index: usize) -> Option<String> {
        (index < 50).then(|| index.to_string())
    }
    fn cell(&self, index: usize, _: &str) -> Option<Element> {
        Some(Element::new("text", Label::new(index.to_string())))
    }
}

#[test]
fn a_table_thumb_is_emphasized_over_its_bar() {
    let mut ui = Runtime::new();
    // Narrower than its columns, so the table scrolls horizontally, and too
    // short for its rows, so its body list scrolls vertically.
    ui.set_viewport(Viewport::new(150.0, 150.0, 1.0).unwrap())
        .unwrap();
    let state = TableController::new(
        &ui,
        ListSelection::new(&ui, SelectionMode::Single),
        [
            TableColumn::new("a", "A").width(Length::new(120.0).unwrap()),
            TableColumn::new("b", "B").width(Length::new(120.0).unwrap()),
        ],
        TableSource::new(Numbers),
    )
    .unwrap();
    ui.mount_tree(Element::new("table", Table::new(&state, 24.0).unwrap()))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let (resting, emphasized) = thumbs(&mut ui);
    assert_eq!(resting.len(), 2, "both bars rest: {resting:?}");
    assert!(emphasized.is_empty());
    // Each bar is emphasized alone while the pointer is over it.
    for thumb in resting.clone() {
        pointer(&mut ui, centre(thumb), PointerKind::Move);
        assert_eq!(thumbs(&mut ui).1, vec![thumb]);
    }
    pointer(&mut ui, Point::new(20.0, 60.0).unwrap(), PointerKind::Move);
    assert_eq!(thumbs(&mut ui).1, vec![]);
}

#[test]
fn a_text_area_thumb_is_emphasized_over_its_bar() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 100.0, 1.0).unwrap())
        .unwrap();
    let text = ui.signal(
        (0..40)
            .map(|line| format!("Line {line}\n"))
            .collect::<String>(),
    );
    ui.mount_tree(Element::new("field", TextArea::new(text)))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    hover_emphasizes(&mut ui, Point::new(20.0, 50.0).unwrap());
}

fn escape(ui: &mut Runtime) {
    ui.key_input(KeyCode::Escape, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::Escape, KeyAction::Release).unwrap();
    ui.flush(ui.now()).unwrap();
}

/// Escape cancels a thumb drag. The thumb then shows whether the pointer is
/// over its bar now, as a release does, without waiting for the next move:
/// still emphasized over the bar, resting when the drag took the pointer away.
fn escape_rechecks_the_hover(ui: &mut Runtime, away: Point) {
    let (resting, emphasized) = thumbs(ui);
    assert_eq!(resting.len(), 1, "one resting thumb: {resting:?}");
    assert!(emphasized.is_empty());
    let thumb = resting[0];
    pointer(ui, centre(thumb), PointerKind::Move);
    pointer(
        ui,
        centre(thumb),
        PointerKind::Press(PointerButton::Primary),
    );
    escape(ui);
    assert_eq!(thumbs(ui).1.len(), 1, "cancelled over the bar");
    pointer(
        ui,
        centre(thumb),
        PointerKind::Release(PointerButton::Primary),
    );

    let (_, emphasized) = thumbs(ui);
    let thumb = emphasized[0];
    pointer(
        ui,
        centre(thumb),
        PointerKind::Press(PointerButton::Primary),
    );
    pointer(ui, away, PointerKind::Move);
    assert_eq!(thumbs(ui).1.len(), 1, "a drag keeps the emphasis");
    escape(ui);
    let (resting, emphasized) = thumbs(ui);
    assert_eq!((resting.len(), emphasized), (1, vec![]), "cancelled away");
    // The release that follows neither drags nor emphasizes.
    pointer(ui, away, PointerKind::Release(PointerButton::Primary));
    assert_eq!(thumbs(ui).1, vec![]);
}

#[test]
fn escape_rechecks_a_scroll_area_thumb_hover() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 100.0, 1.0).unwrap())
        .unwrap();
    ui.mount_tree(Element::new(
        "scroll",
        ScrollArea::new(Element::new(
            "content",
            Container::new(Element::new("label", Label::new("Content")))
                .size_limits(SizeLimits::at_least(Size::from([180, 400]))),
        )),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    escape_rechecks_the_hover(&mut ui, Point::new(20.0, 60.0).unwrap());
}

#[test]
fn escape_rechecks_a_list_thumb_hover() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 100.0, 1.0).unwrap())
        .unwrap();
    let rows = ui.signal(ListSource::new(Rows(50)));
    ui.mount_tree(Element::new(
        "list",
        VirtualList::new(rows.read_only(), 24.0).unwrap(),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    escape_rechecks_the_hover(&mut ui, Point::new(20.0, 60.0).unwrap());
}

#[test]
fn escape_rechecks_a_table_thumb_hover() {
    let mut ui = Runtime::new();
    // Narrower than its columns, so the table scrolls horizontally; tall
    // enough for its rows, so its body shows no bar of its own.
    ui.set_viewport(Viewport::new(150.0, 200.0, 1.0).unwrap())
        .unwrap();
    let state = TableController::new(
        &ui,
        ListSelection::new(&ui, SelectionMode::Single),
        [
            TableColumn::new("a", "A").width(Length::new(120.0).unwrap()),
            TableColumn::new("b", "B").width(Length::new(120.0).unwrap()),
        ],
        TableSource::new(Rows4),
    )
    .unwrap();
    let table = ui
        .mount_tree(Element::new("table", Table::new(&state, 24.0).unwrap()))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    // A header press moves the focus to the body at the next flush; this
    // helper sends Escape without one, so the body is focused beforehand.
    let body = ui.children(&table).unwrap()[0].clone();
    ui.focus(&body).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    escape_rechecks_the_hover(&mut ui, Point::new(20.0, 60.0).unwrap());
    assert_eq!(ui.focused(), Some(body));
}

/// A few rows, which fit a table's body without scrolling it.
struct Rows4;
impl TableItems for Rows4 {
    fn len(&self) -> usize {
        4
    }
    fn key(&self, index: usize) -> Option<String> {
        (index < 4).then(|| index.to_string())
    }
    fn cell(&self, index: usize, _: &str) -> Option<Element> {
        Some(Element::new("text", Label::new(index.to_string())))
    }
}

#[test]
fn escape_rechecks_a_text_area_thumb_hover() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 100.0, 1.0).unwrap())
        .unwrap();
    let text = ui.signal(
        (0..40)
            .map(|line| format!("Line {line}\n"))
            .collect::<String>(),
    );
    ui.mount_tree(Element::new("field", TextArea::new(text)))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    escape_rechecks_the_hover(&mut ui, Point::new(20.0, 60.0).unwrap());
}
