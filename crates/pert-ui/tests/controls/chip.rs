//! Chips: static styling without a Tab stop, removal requests reported to the
//! owner, identity by key and focus after the owner removes the focused chip.
use crate::support;
use pert_ui::*;
use std::{cell::RefCell, rc::Rc, time::Duration};
use support::{key, settle, widget};

type Removed = Rc<RefCell<Vec<String>>>;

fn ui() -> Runtime {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(600.0, 300.0, 1.0).unwrap())
        .unwrap();
    ui
}

/// Removable chips keyed by `keys`, each reporting its key to `removed`.
fn chips(keys: &[&str], removed: &Removed) -> Vec<Element> {
    keys.iter()
        .map(|key| {
            let (removed, name) = (removed.clone(), (*key).to_owned());
            Element::new(
                *key,
                Chip::removable(format!("Tag {key}"), move |_| {
                    removed.borrow_mut().push(name.clone());
                    Ok(())
                }),
            )
        })
        .collect()
}

fn mount(ui: &mut Runtime, keys: &[&str], removed: &Removed) -> WidgetId {
    let root = ui
        .mount_tree(Element::new("tags", Wrap::new(chips(keys, removed))))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    root
}

fn focused_key(ui: &Runtime) -> Option<String> {
    ui.focused().map(|id| ui.key(&id).unwrap().to_owned())
}

fn semantics(ui: &Runtime, key: &str) -> Semantics {
    ui.inspect()
        .components
        .into_iter()
        .find(|component| component.key == key)
        .and_then(|component| component.details.semantics)
        .unwrap_or_else(|| panic!("no semantics for {key}"))
}

#[test]
fn static_chip_takes_no_tab_stop_and_reports_a_label() {
    let mut ui = ui();
    let root = ui
        .mount_tree(Element::new(
            "row",
            Row::new([
                Element::new("tag", Chip::new("Draft").leading_icon(Icon::Info)),
                Element::new("button", Button::new("Go", |_| Ok(()))),
            ]),
        ))
        .unwrap();
    settle(&mut ui);
    key(&mut ui, KeyCode::Tab);
    settle(&mut ui);
    assert_eq!(focused_key(&ui).as_deref(), Some("button"));
    let tag = widget(&ui, &root, "tag");
    assert!(ui.bounds(&tag).unwrap().is_some());
    let semantics = semantics(&ui, "tag");
    assert_eq!(semantics.role, SemanticRole::Label);
    assert_eq!(semantics.name.as_deref(), Some("Draft"));
    assert!(semantics.actions.is_empty());
}

#[test]
fn delete_and_backspace_request_removal_once_per_press() {
    let mut ui = ui();
    let removed = Removed::default();
    let root = mount(&mut ui, &["a", "b"], &removed);
    ui.focus(&widget(&ui, &root, "a")).unwrap();
    settle(&mut ui);
    ui.key_input(KeyCode::Delete, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::Delete, KeyAction::Repeat).unwrap();
    ui.key_input(KeyCode::Delete, KeyAction::Release).unwrap();
    assert_eq!(*removed.borrow(), ["a"]);
    // The chip never removes itself: the owner has not described it away.
    settle(&mut ui);
    assert_eq!(focused_key(&ui).as_deref(), Some("a"));
    key(&mut ui, KeyCode::Backspace);
    assert_eq!(*removed.borrow(), ["a", "a"]);
    let semantics = semantics(&ui, "a");
    assert_eq!(semantics.actions, [SemanticAction::Dismiss]);
    assert_eq!(semantics.description.as_deref(), Some("Remove Tag a"));
}

#[test]
fn pointer_removal_owns_interleaved_delete_and_backspace() {
    for (code, native) in [KeyCode::Delete, KeyCode::Backspace]
        .into_iter()
        .flat_map(|code| [(code, false), (code, true)])
    {
        let remove_key = |ui: &mut Runtime| {
            if native {
                for action in [KeyAction::Press, KeyAction::Release] {
                    ui.command_key_input(CommandKeyInput::new(
                        code,
                        action,
                        CommandKey::Named(code),
                    ))
                    .unwrap();
                }
            } else {
                key(ui, code);
            }
        };
        let mut ui = ui();
        let removed = Removed::default();
        let root = mount(&mut ui, &["a"], &removed);
        let chip = widget(&ui, &root, "a");
        let [x, y, w, h] = ui.bounds(&chip).unwrap().unwrap().values();
        let point = Point::new(x + w - h / 2.0, y + h / 2.0).unwrap();
        ui.focus(&chip).unwrap();
        ui.pointer_input(PointerEvent::new(
            point,
            PointerKind::Press(PointerButton::Primary),
        ))
        .unwrap();
        remove_key(&mut ui);
        assert!(removed.borrow().is_empty());
        ui.pointer_input(PointerEvent::new(
            point,
            PointerKind::Release(PointerButton::Primary),
        ))
        .unwrap();
        assert_eq!(*removed.borrow(), ["a"]);
        remove_key(&mut ui);
        assert_eq!(*removed.borrow(), ["a", "a"]);
    }
}

#[test]
fn the_remove_button_requests_removal_on_a_matched_click_only() {
    let mut ui = ui();
    let removed = Removed::default();
    let root = mount(&mut ui, &["a"], &removed);
    let [x, y, w, h] = ui
        .bounds(&widget(&ui, &root, "a"))
        .unwrap()
        .unwrap()
        .values();
    let point = |x: f32, y: f32| Point::new(x, y).unwrap();
    let button = point(x + w - h / 2.0, y + h / 2.0);
    let body = point(x + 4.0, y + h / 2.0);
    let outside = point(x + w + 50.0, y + h / 2.0);
    let primary = PointerButton::Primary;
    let send = |ui: &mut Runtime, at, kind| {
        ui.pointer_input(PointerEvent::new(at, kind)).unwrap();
    };
    // A click on the label body does not remove.
    send(&mut ui, body, PointerKind::Press(primary));
    send(&mut ui, body, PointerKind::Release(primary));
    assert!(removed.borrow().is_empty());
    // Releasing outside the button cancels.
    send(&mut ui, button, PointerKind::Press(primary));
    send(&mut ui, outside, PointerKind::Release(primary));
    assert!(removed.borrow().is_empty());
    // Escape cancels an armed press.
    send(&mut ui, button, PointerKind::Press(primary));
    ui.focus(&widget(&ui, &root, "a")).unwrap();
    key(&mut ui, KeyCode::Escape);
    send(&mut ui, button, PointerKind::Release(primary));
    assert!(removed.borrow().is_empty());
    send(&mut ui, button, PointerKind::Press(primary));
    send(&mut ui, button, PointerKind::Release(primary));
    assert_eq!(*removed.borrow(), ["a"]);
}

#[test]
fn chips_keep_identity_and_focus_by_key_when_reordered_or_relabelled() {
    let mut ui = ui();
    let removed = Removed::default();
    let root = mount(&mut ui, &["a", "b", "c"], &removed);
    let before: Vec<_> = ["a", "b", "c"]
        .iter()
        .map(|key| widget(&ui, &root, key))
        .collect();
    ui.focus(&before[1]).unwrap();
    settle(&mut ui);
    let mut reordered = chips(&["c", "a"], &removed);
    reordered.insert(1, Element::new("b", Chip::removable("Renamed", |_| Ok(()))));
    ui.reconcile_children(&root, reordered).unwrap();
    settle(&mut ui);
    assert_eq!(widget(&ui, &root, "a"), before[0]);
    assert_eq!(widget(&ui, &root, "b"), before[1]);
    assert_eq!(widget(&ui, &root, "c"), before[2]);
    assert_eq!(ui.focused(), Some(before[1].clone()));
    assert_eq!(semantics(&ui, "b").name.as_deref(), Some("Renamed"));
}

#[test]
fn removing_the_focused_chip_moves_focus_to_the_next_then_the_previous_chip() {
    let mut ui = ui();
    let removed = Removed::default();
    let root = mount(&mut ui, &["a", "b", "c"], &removed);
    ui.focus(&widget(&ui, &root, "b")).unwrap();
    settle(&mut ui);
    key(&mut ui, KeyCode::Delete);
    assert_eq!(*removed.borrow(), ["b"]);
    ui.reconcile_children(&root, chips(&["a", "c"], &removed))
        .unwrap();
    settle(&mut ui);
    assert_eq!(focused_key(&ui).as_deref(), Some("c"), "next chip");
    // The last chip has no next one, so focus goes back to the previous chip.
    ui.reconcile_children(&root, chips(&["a"], &removed))
        .unwrap();
    settle(&mut ui);
    assert_eq!(focused_key(&ui).as_deref(), Some("a"), "previous chip");
}

#[test]
fn removal_skips_neighbors_removed_by_the_same_description() {
    let mut ui = ui();
    let removed = Removed::default();
    let root = mount(&mut ui, &["a", "b", "c", "d"], &removed);
    ui.focus(&widget(&ui, &root, "b")).unwrap();
    settle(&mut ui);
    ui.reconcile_children(&root, chips(&["a", "d"], &removed))
        .unwrap();
    settle(&mut ui);
    assert_eq!(focused_key(&ui).as_deref(), Some("d"));
}

/// A focusable owner around the chips, standing in for a control such as a
/// multi-select trigger.
struct Owner(Vec<Element>);
impl Component for Owner {
    fn focusable(&self) -> bool {
        true
    }
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.0)
    }
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
}

#[test]
fn removing_the_last_focused_chip_moves_focus_to_the_owner() {
    let mut ui = ui();
    let removed = Removed::default();
    let owner = ui
        .mount_tree(Element::new(
            "owner",
            Owner(vec![Element::new(
                "tags",
                Wrap::new(chips(&["a"], &removed)),
            )]),
        ))
        .unwrap();
    settle(&mut ui);
    let tags = widget(&ui, &owner, "tags");
    ui.focus(&widget(&ui, &owner, "a")).unwrap();
    settle(&mut ui);
    ui.reconcile_children(&tags, Vec::new()).unwrap();
    settle(&mut ui);
    assert_eq!(ui.focused(), Some(owner));
}

#[test]
fn other_siblings_are_not_candidates() {
    let mut ui = ui();
    let removed = Removed::default();
    let mut children = chips(&["a"], &removed);
    children.push(Element::new("button", Button::new("Add", |_| Ok(()))));
    let root = ui
        .mount_tree(Element::new(
            "owner",
            Owner(vec![Element::new("tags", Wrap::new(children))]),
        ))
        .unwrap();
    settle(&mut ui);
    let tags = widget(&ui, &root, "tags");
    ui.focus(&widget(&ui, &root, "a")).unwrap();
    settle(&mut ui);
    ui.reconcile_children(
        &tags,
        vec![Element::new("button", Button::new("Add", |_| Ok(())))],
    )
    .unwrap();
    settle(&mut ui);
    assert_eq!(ui.focused(), Some(root), "the button is not a chip");
}

#[test]
fn a_disabled_removable_chip_offers_no_removal() {
    let mut ui = ui();
    let root = ui
        .mount_tree(Element::new(
            "row",
            Row::new([
                Element::new("tag", Chip::removable("Locked", |_| Ok(())).disabled()),
                Element::new("button", Button::new("Go", |_| Ok(()))),
            ]),
        ))
        .unwrap();
    settle(&mut ui);
    key(&mut ui, KeyCode::Tab);
    settle(&mut ui);
    assert_eq!(focused_key(&ui).as_deref(), Some("button"));
    let semantics = semantics(&ui, "tag");
    assert!(semantics.disabled);
    assert!(semantics.actions.is_empty());
    assert!(widget(&ui, &root, "tag") != widget(&ui, &root, "button"));
}

/// An owner that holds the data and describes its chips again from it, the
/// way a multi-select chooser does: removal writes the data, and the owner's
/// next update drops the chip.
struct TagEditor {
    tags: Signal<Vec<String>>,
    shown: Option<Vec<String>>,
}
impl TagEditor {
    fn describe(&self, tags: &[String]) -> Vec<Element> {
        let chips = tags
            .iter()
            .map(|tag| {
                let (data, removed) = (self.tags.clone(), tag.clone());
                Element::new(
                    tag.clone(),
                    Chip::removable(tag.clone(), move |_| {
                        let mut list = data.get()?;
                        list.retain(|tag| *tag != removed);
                        data.set(list).map(|_| ())
                    }),
                )
            })
            .collect::<Vec<_>>();
        vec![Element::new("chips", Wrap::new(chips))]
    }
}
impl Component for TagEditor {
    fn focusable(&self) -> bool {
        true
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let tags = cx.read(&self.tags.read_only())?;
        if self.shown.as_ref() != Some(&tags) {
            cx.request_children(self.describe(&tags));
            self.shown = Some(tags);
        }
        Ok(())
    }
}

#[test]
fn an_owner_rebuilding_from_its_data_keeps_focus_moving_along_the_row() {
    let mut ui = ui();
    let tags = ui.signal(vec!["a".to_owned(), "b".to_owned(), "c".to_owned()]);
    let editor = ui
        .mount_tree(Element::new(
            "editor",
            TagEditor {
                tags: tags.clone(),
                shown: None,
            },
        ))
        .unwrap();
    settle(&mut ui);
    ui.focus(&widget(&ui, &editor, "a")).unwrap();
    settle(&mut ui);
    for (expected_focus, expected_tags) in [("b", vec!["b", "c"]), ("c", vec!["c"])] {
        key(&mut ui, KeyCode::Delete);
        settle(&mut ui);
        assert_eq!(tags.get().unwrap(), expected_tags);
        assert_eq!(focused_key(&ui).as_deref(), Some(expected_focus));
    }
    key(&mut ui, KeyCode::Delete);
    settle(&mut ui);
    assert!(tags.get().unwrap().is_empty());
    assert_eq!(ui.focused(), Some(editor), "the owner takes focus last");
}

struct Allocation {
    size: Size,
    child: Option<Element>,
}
impl Component for Allocation {
    fn take_children(&mut self) -> Vec<Element> {
        self.child.take().into_iter().collect()
    }
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(self.size)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, _: Size) -> Result<(), RuntimeError> {
        let _preferred = cx.measure(0, Constraints::default())?;
        cx.place(0, Rect::from_size(self.size))
    }
}
#[test]
fn constrained_chips_bound_label_and_remove_ink_and_keep_complete_semantics() {
    let label = "A long localized selection – 演奏者 🎛️";
    for width in [1.0, 4.0, 12.0, 70.0] {
        for height in [1.0, 8.0, 30.0] {
            for icon in [false, true] {
                let mut ui = ui();
                let removed = Removed::default();
                let requests = removed.clone();
                let mut chip = Chip::removable(label, move |_| {
                    requests.borrow_mut().push("removed".into());
                    Ok(())
                });
                if icon {
                    chip = chip.leading_icon(Icon::Info);
                }
                let root = ui
                    .mount_tree(Element::new(
                        "allocation",
                        Allocation {
                            size: Size::new(width, height).unwrap(),
                            child: Some(Element::new("chip", chip)),
                        },
                    ))
                    .unwrap();
                ui.flush(ui.now()).unwrap();
                let id = widget(&ui, &root, "chip");
                let scene = ui.paint().unwrap();
                let mut ink = 0;
                let mut ink_clips = Vec::new();
                let theme = ui.theme();
                let foreground = theme
                    .palette(theme.mode().unwrap())
                    .unwrap()
                    .color(ColorRole::OnSecondaryContainer);
                for command in scene.commands() {
                    if let DrawCommand::DrawMask { rect, clip, .. }
                    | DrawCommand::DrawImage { rect, clip, .. } = command
                    {
                        let clip = clip.expect("label and removal glyphs need bounded clips");
                        assert!(clip.x() >= 0.0 && clip.y() >= 0.0);
                        assert!(clip.x() + clip.width() <= width);
                        assert!(clip.y() + clip.height() <= height);
                        let visible = rect.intersection(clip);
                        assert!(visible.x() + visible.width() <= width);
                        // Rounded background corners are masks too. Only text
                        // foreground and color glyph images participate in the
                        // label/remove separation assertion.
                        if !matches!(command, DrawCommand::DrawMask { color, .. }
                            if *color != foreground)
                        {
                            ink_clips.push(clip);
                            ink += 1;
                        }
                    }
                }
                if width == 70.0 && height == 30.0 {
                    assert!(ink > 0);
                    let trailing = ink_clips
                        .iter()
                        .max_by(|a, b| a.x().partial_cmp(&b.x()).unwrap())
                        .unwrap();
                    let labels: Vec<_> =
                        ink_clips.iter().filter(|clip| *clip != trailing).collect();
                    assert!(
                        !labels.is_empty(),
                        "both label and remove ink must be exercised"
                    );
                    assert!(
                        labels
                            .iter()
                            .all(|clip| clip.x() + clip.width() <= trailing.x()),
                        "label clipping reserves the remove target: {ink_clips:?}, icon={icon}"
                    );
                }
                let semantics = semantics(&ui, "chip");
                assert_eq!(semantics.name.as_deref(), Some(label));
                assert_eq!(
                    semantics.description.as_deref(),
                    Some(format!("Remove {label}").as_str())
                );
                let bounds = ui.bounds(&id).unwrap().unwrap();
                let metrics = ui.theme().metrics(ui.theme().density().unwrap()).unwrap();
                let target = metrics.get(MetricRole::IconSize).min(width).min(height);
                let right = (width - metrics.get(MetricRole::ControlPaddingX) / 2.0).max(target);
                let point =
                    Point::new(bounds.x() + right - target / 2.0, bounds.y() + height / 2.0)
                        .unwrap();
                for kind in [
                    PointerKind::Press(PointerButton::Primary),
                    PointerKind::Release(PointerButton::Primary),
                ] {
                    ui.pointer_input(PointerEvent::new(point, kind)).unwrap();
                }
                assert_eq!(removed.borrow().len(), 1);
                ui.focus(&id).unwrap();
                for action in [KeyAction::Press, KeyAction::Release] {
                    ui.command_key_input(CommandKeyInput::new(
                        KeyCode::Delete,
                        action,
                        CommandKey::Named(KeyCode::Delete),
                    ))
                    .unwrap();
                }
                assert_eq!(removed.borrow().len(), 2);
            }
        }
    }
}

#[test]
fn constrained_static_and_disabled_chips_clip_labels_without_removal() {
    for disabled in [false, true] {
        let mut ui = ui();
        let removed = Removed::default();
        let requests = removed.clone();
        let chip = if disabled {
            Chip::removable("A long label that cannot fit", move |_| {
                requests.borrow_mut().push("removed".into());
                Ok(())
            })
            .disabled()
        } else {
            Chip::new("A long label that cannot fit")
        };
        let root = ui
            .mount_tree(Element::new(
                "allocation",
                Allocation {
                    size: Size::new(20.0, 8.0).unwrap(),
                    child: Some(Element::new("chip", chip.leading_icon(Icon::Info))),
                },
            ))
            .unwrap();
        ui.flush(ui.now()).unwrap();
        let theme = ui.theme();
        let foreground = theme
            .palette(theme.mode().unwrap())
            .unwrap()
            .color(if disabled {
                ColorRole::Muted
            } else {
                ColorRole::OnSecondaryContainer
            });
        for command in ui.paint().unwrap().commands() {
            assert!(
                !matches!(command, DrawCommand::DrawMask { color, .. } if *color == foreground),
                "overlapping horizontal padding leaves no label space"
            );
            assert!(!matches!(command, DrawCommand::DrawImage { .. }));
            if let DrawCommand::DrawMask { clip, .. } = command {
                let clip = clip.expect("a constrained label is clipped");
                assert!(clip.x() + clip.width() <= 20.0);
                assert!(clip.y() + clip.height() <= 8.0);
            }
        }
        let id = widget(&ui, &root, "chip");
        assert!(semantics(&ui, "chip").actions.is_empty());
        ui.focus(&id).unwrap();
        for action in [KeyAction::Press, KeyAction::Release] {
            ui.command_key_input(CommandKeyInput::new(
                KeyCode::Delete,
                action,
                CommandKey::Named(KeyCode::Delete),
            ))
            .unwrap();
        }
        assert!(removed.borrow().is_empty());
    }
}

#[test]
fn naturally_sized_chip_labels_keep_their_raster_ink_at_fractional_scale() {
    for label in ["j", "f", "Åfj\u{301}", "演奏者"] {
        for removable in [false, true] {
            let mut ui = ui();
            ui.set_viewport(Viewport::new(600.0, 300.0, 1.25).unwrap())
                .unwrap();
            let chip = if removable {
                Chip::removable(label, |_| Ok(()))
            } else {
                Chip::new(label)
            };
            let root = ui
                .mount_tree(Element::new(
                    "row",
                    Wrap::new([Element::new("chip", chip.leading_icon(Icon::Info))]),
                ))
                .unwrap();
            ui.flush(ui.now()).unwrap();
            let id = widget(&ui, &root, "chip");
            let bounds = ui.bounds(&id).unwrap().unwrap();
            let theme = ui.theme();
            let foreground = theme
                .palette(theme.mode().unwrap())
                .unwrap()
                .color(ColorRole::OnSecondaryContainer);
            let mut label_masks = 0;
            for command in ui.paint().unwrap().commands() {
                if let DrawCommand::DrawMask {
                    rect,
                    clip: Some(clip),
                    color,
                    ..
                } = command
                    && *color == foreground
                    && clip.height() == bounds.height()
                {
                    let visible = rect.intersection(*clip);
                    assert!(
                        (visible.width() - rect.width()).abs() < 0.001,
                        "{label}: {rect:?} under {clip:?}"
                    );
                    assert!(
                        (visible.height() - rect.height()).abs() < 0.001,
                        "{label}: {rect:?} under {clip:?}"
                    );
                    label_masks += 1;
                }
            }
            assert!(
                label_masks > 0,
                "the label/leading icon must actually render"
            );
        }
    }
}
