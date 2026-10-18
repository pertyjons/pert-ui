//! Public font identity, rich-label layout, icon overrides and retained dependencies.
use pert_ui::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};
type Result = std::result::Result<(), Box<dyn std::error::Error>>;
fn ui() -> Runtime {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 480.0, 1.0).unwrap())
        .unwrap();
    ui
}
fn draw(ui: &mut Runtime) -> DisplayList {
    ui.flush(Duration::ZERO).unwrap();
    ui.paint().unwrap()
}
fn ink(scene: &DisplayList) -> Vec<Vec<u8>> {
    scene.masks().iter().map(|m| m.pixels().to_vec()).collect()
}
struct Probe {
    content: RichText,
    paints: Rc<Cell<usize>>,
    measures: Rc<Cell<usize>>,
    layout: RefCell<Option<TextLayout>>,
    fixed: bool,
}
impl Component for Probe {
    fn update(&mut self, _: &mut UpdateContext) -> std::result::Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(
        &self,
        cx: &MeasureContext<'_>,
        _: Constraints,
    ) -> std::result::Result<Size, RuntimeError> {
        self.measures.set(self.measures.get() + 1);
        if self.fixed {
            Ok(Size::new(200.0, 50.0)?)
        } else {
            Ok(cx.rich_text(&self.content, None, TextStyle::Body)?.size())
        }
    }
    fn arrange(
        &self,
        cx: &mut ArrangeContext<'_>,
        _: Size,
    ) -> std::result::Result<(), RuntimeError> {
        if !self.fixed {
            *self.layout.borrow_mut() = Some(cx.measurement().rich_text(
                &self.content,
                None,
                TextStyle::Body,
            )?);
        }
        Ok(())
    }
    fn paint(&self, p: &mut Painter) -> std::result::Result<(), DrawError> {
        self.paints.set(self.paints.get() + 1);
        let layout = if self.fixed {
            p.rich_text(&self.content, None, TextStyle::Body)?
        } else {
            self.layout.borrow().as_ref().unwrap().clone()
        };
        layout.paint(p, 0.0, 0.0, p.color(ColorRole::OnSurface))
    }
}
fn probe(content: RichText, fixed: bool) -> (Probe, Rc<Cell<usize>>, Rc<Cell<usize>>) {
    let paints = Rc::new(Cell::new(0));
    let measures = Rc::new(Cell::new(0));
    (
        Probe {
            content,
            paints: paints.clone(),
            measures: measures.clone(),
            layout: RefCell::new(None),
            fixed,
        },
        paints,
        measures,
    )
}
#[test]
fn registration_is_local_atomic_and_does_not_schedule_unused_fonts() -> Result {
    let mut ui = ui();
    let fonts = ui.fonts();
    ui.mount_tree(Element::new("label", Label::new("Ready")))?;
    draw(&mut ui);
    let body = fonts.default_font(FontRole::Body)?;
    assert_eq!(
        fonts.register(FontData::from_owned(vec![0; 30])),
        Err(RuntimeError::Font(FontError::InvalidData))
    );
    assert_eq!(
        fonts.register(FontData::from_owned(vec![0; FontData::MAX_BYTES + 1])),
        Err(RuntimeError::Font(FontError::TooLarge))
    );
    assert_eq!(
        fonts.register(FontData::from_owned(vec![0; FontData::MAX_BYTES])),
        Err(RuntimeError::Font(FontError::InvalidData)),
        "data at the limit passes the size check and is then parsed"
    );
    assert_eq!(fonts.default_font(FontRole::Body)?, body);
    let one = fonts.register(FontData::bundled(FontRole::Monospace))?;
    let two = fonts.register(FontData::bundled(FontRole::Monospace))?;
    assert_ne!(one, two);
    assert!(!ui.needs_paint());
    let other = Runtime::new().fonts();
    assert_eq!(
        other.default_font(FontRole::Body),
        Err(RuntimeError::Closed)
    );
    let other_ui = Runtime::new();
    let foreign = other_ui.fonts().default_font(FontRole::Body)?;
    assert_eq!(
        fonts.set_default(FontRole::Body, foreign),
        Err(RuntimeError::Font(FontError::ForeignFont))
    );
    assert_eq!(
        fonts.icon_glyph(one, '\u{10ffff}'),
        Err(RuntimeError::Font(FontError::MissingGlyph))
    );
    assert_eq!(
        fonts.material_icon("absent_name"),
        Err(RuntimeError::Font(FontError::UnknownIcon))
    );
    let _glyph = fonts.material_icon("folder_open")?;
    let icons = ui.icons();
    drop(ui);
    assert_eq!(
        fonts.default_font(FontRole::Body),
        Err(RuntimeError::Closed)
    );
    assert_eq!(icons.reset(Icon::Close), Err(RuntimeError::Closed));
    Ok(())
}
#[test]
fn defaults_invalidate_only_role_readers_and_heading_inherits_body() -> Result {
    let mut ui = ui();
    let fonts = ui.fonts();
    let original = fonts.default_font(FontRole::Body)?;
    let mono = fonts.register(FontData::bundled(FontRole::Monospace))?;
    let (body, bp, bm) = probe("iiii WWWW".into(), false);
    let (explicit, ep, em) = probe(
        RichText::new().span(TextSpan::new("iiii WWWW").font(original.clone())),
        false,
    );
    let (fixed, fp, fm) = probe("iiii WWWW".into(), true);
    ui.mount_tree(Element::new(
        "root",
        Column::new([
            Element::new("body", body),
            Element::new("explicit", explicit),
            Element::new("fixed", fixed),
        ]),
    ))?;
    draw(&mut ui);
    let before = (bm.get(), em.get(), fm.get());
    fonts.set_default(FontRole::Body, mono.clone())?;
    assert_eq!(fonts.default_font(FontRole::Heading)?, mono);
    draw(&mut ui);
    assert!(bm.get() > before.0);
    assert_eq!(em.get(), before.1);
    assert_eq!(fm.get(), before.2);
    assert_eq!((bp.get(), ep.get(), fp.get()), (2, 1, 2));
    fonts.set_default(FontRole::Heading, original.clone())?;
    draw(&mut ui);
    fonts.reset_default(FontRole::Body)?;
    draw(&mut ui);
    assert_eq!(fonts.default_font(FontRole::Heading)?, original);
    fonts.reset_default(FontRole::Heading)?;
    assert!(!ui.needs_paint());
    Ok(())
}
#[test]
fn hidden_readers_defer_font_work_and_unmount_removes_subscriptions() -> Result {
    let mut ui = ui();
    let fonts = ui.fonts();
    let (component, paints, measures) = probe("Hidden label".into(), false);
    let id = ui.mount_tree(Element::new("label", component))?;
    draw(&mut ui);
    ui.set_visible(&id, false)?;
    draw(&mut ui);
    let count = measures.get();
    let mono = fonts.default_font(FontRole::Monospace)?;
    fonts.set_default(FontRole::Body, mono)?;
    assert_eq!(ui.flush(Duration::ZERO)?.components_updated, 0);
    assert!(!ui.needs_paint());
    assert_eq!(measures.get(), count);
    ui.set_visible(&id, true)?;
    draw(&mut ui);
    assert!(measures.get() > count);
    assert_eq!(paints.get(), 2);
    ui.unmount(&id)?;
    draw(&mut ui);
    fonts.reset_default(FontRole::Body)?;
    assert!(!ui.needs_paint());
    assert!(ui.next_deadline().is_none());
    Ok(())
}
#[test]
fn colored_mixed_fonts_share_geometry_and_palette_changes_reuse_coverage() -> Result {
    let mut ui = ui();
    let fonts = ui.fonts();
    let mono = fonts.default_font(FontRole::Monospace)?;
    let content = RichText::new()
        .icon(Icon::Settings)
        .gap()
        .text("Settings ")
        .span(
            TextSpan::new("123")
                .font(mono)
                .color_role(ColorRole::Warning),
        )
        .text(" e\u{301} אבג");
    let (component, paints, measures) = probe(content, false);
    ui.mount_tree(Element::new("label", component))?;
    let first = draw(&mut ui);
    let measured = measures.get();
    assert!(first.masks().len() >= 2);
    assert!(
        first
            .masks()
            .iter()
            .all(|m| m.pixels().iter().any(|p| *p != 0))
    );
    ui.theme()
        .set_color(ThemeMode::Dark, ColorRole::Warning, Color::ACCENT)?;
    let second = draw(&mut ui);
    assert_eq!(measures.get(), measured);
    assert_eq!(paints.get(), 2);
    for (a, b) in first.masks().iter().zip(second.masks()) {
        assert_eq!(a.pixels().as_ptr(), b.pixels().as_ptr());
    }
    assert_ne!(first.commands(), second.commands());
    Ok(())
}
#[test]
fn icon_sets_are_atomic_and_selects_follow_shared_overrides() -> Result {
    let mut ui = ui();
    let fonts = ui.fonts();
    let icons = ui.icons();
    let select = Select::new(
        &ui,
        ui.signal(0),
        [Choice::new(0, "First"), Choice::new(1, "Second")],
    )?;
    ui.mount_tree(Element::new("select", select))?;
    let before = draw(&mut ui);
    let glyph = fonts.material_icon("close")?;
    icons.set(Icon::ChevronDown, glyph.clone())?;
    let changed = draw(&mut ui);
    assert_ne!(ink(&before), ink(&changed));
    assert_eq!(
        icons.set_overrides([(Icon::Close, glyph.clone()), (Icon::Close, glyph.clone())]),
        Err(RuntimeError::Font(FontError::DuplicateIcon))
    );
    assert!(!ui.needs_paint());
    let other = Runtime::new();
    let foreign = other.fonts().material_icon("add")?;
    assert_eq!(
        icons.set_overrides([(Icon::ChevronDown, glyph), (Icon::Close, foreign)]),
        Err(RuntimeError::Font(FontError::ForeignFont))
    );
    assert!(!ui.needs_paint());
    icons.set_overrides([])?;
    assert_eq!(ink(&before), ink(&draw(&mut ui)));
    Ok(())
}
#[test]
fn changing_rich_tab_metadata_keeps_identity_and_font_changes_refresh_titles() -> Result {
    let mut ui = ui();
    let fonts = ui.fonts();
    let tab = |icon| {
        Tab::new(
            "a",
            "Title WWW iii",
            Element::new("body", Label::new("Content")),
        )
        .leading_icon(icon)
        .into_element()
    };
    let tabs =
        ui.mount_tree(Element::new("tabs", Tabs::new([])).children([tab(Icon::Settings)]))?;
    let first = draw(&mut ui);
    let id = ui.children(&tabs)?[0].clone();
    ui.reconcile_children(&tabs, vec![tab(Icon::Save)])?;
    let second = draw(&mut ui);
    assert_eq!(ui.children(&tabs)?[0], id);
    assert_ne!(ink(&first), ink(&second));
    fonts.set_default(FontRole::Body, fonts.default_font(FontRole::Monospace)?)?;
    assert_ne!(ink(&second), ink(&draw(&mut ui)));
    Ok(())
}
#[test]
fn rich_binding_recollects_dependencies_and_bound_labels_keep_icons() -> Result {
    let mut ui = ui();
    let content = ui.signal(RichText::new().icon(Icon::Close));
    let original = ui.mount_tree(Element::new("label", Label::new(content.read_only())))?;
    draw(&mut ui);
    content.set(RichText::new().icon(Icon::Save))?;
    draw(&mut ui);
    ui.icons()
        .set(Icon::Close, ui.fonts().material_icon("add")?)?;
    assert!(!ui.needs_paint());
    ui.icons()
        .set(Icon::Save, ui.fonts().material_icon("add")?)?;
    assert!(ui.flush(Duration::ZERO)?.components_updated > 0);
    ui.paint()?;
    ui.unmount(&original)?;
    draw(&mut ui);
    let text = ui.signal("Before".to_owned());
    let id = ui.mount_tree(Element::new(
        "bound",
        Label::new(text.read_only()).leading_icon(Icon::Settings),
    ))?;
    draw(&mut ui);
    text.set("After".into())?;
    draw(&mut ui);
    ui.icons()
        .set(Icon::Settings, ui.fonts().material_icon("save")?)?;
    assert!(ui.flush(Duration::ZERO)?.components_updated > 0);
    ui.paint()?;
    ui.unmount(&id)?;
    Ok(())
}
#[test]
fn invalid_rich_text_is_rejected_without_panics() -> Result {
    for (content, error) in [
        (
            RichText::new().text("x".repeat(TextEditError::MAX_TEXT_BYTES + 1)),
            RuntimeError::TextEdit(TextEditError::TooLarge),
        ),
        (
            (0..=RichText::MAX_SEGMENTS).fold(RichText::new(), |t, _| t.text("")),
            RuntimeError::Text(TextError::TooLarge(TextLimit::Segments)),
        ),
        (
            RichText::new().span(TextSpan::new("x").size(Length::new(0.0)?)),
            RuntimeError::Text(TextError::Geometry),
        ),
        (
            RichText::new()
                .icon(Icon::Add)
                .span(TextSpan::new("").size(Length::new(0.0)?)),
            RuntimeError::Text(TextError::Geometry),
        ),
    ] {
        let mut ui = ui();
        ui.mount_tree(Element::new("bad", Label::new(content)))?;
        assert_eq!(ui.flush(Duration::ZERO), Err(error));
    }
    let mut most = ui();
    most.mount_tree(Element::new(
        "most",
        Label::new((0..RichText::MAX_SEGMENTS).fold(RichText::new(), |t, _| t.text(""))),
    ))?;
    most.flush(Duration::ZERO)?;
    let foreign_ui = Runtime::new();
    let foreign = foreign_ui.fonts().default_font(FontRole::Body)?;
    let mut ui = ui();
    ui.mount_tree(Element::new(
        "bad",
        Label::new(RichText::new().span(TextSpan::new("x").font(foreign))),
    ))?;
    assert_eq!(
        ui.flush(Duration::ZERO),
        Err(RuntimeError::Font(FontError::ForeignFont))
    );
    Ok(())
}

#[test]
fn label_decorations_participate_in_preferred_size_and_scale() -> Result {
    for scale in [1.0, 1.25, 2.0] {
        let mut ui = ui();
        ui.set_viewport(Viewport::new(800.0, 480.0, scale)?)?;
        let root = ui.mount_tree(Element::new(
            "row",
            Row::new([
                Element::new("plain", Label::new("Text")),
                Element::new(
                    "decorated",
                    Label::new("Text")
                        .leading_icon(Icon::Settings)
                        .trailing_icon(Icon::Save),
                ),
                Element::new(
                    "rich",
                    Label::new(
                        RichText::from("Text")
                            .leading_icon(Icon::Settings)
                            .trailing_icon(Icon::Save),
                    ),
                ),
            ]),
        ))?;
        draw(&mut ui);
        let children = ui.children(&root)?;
        let plain = ui.bounds(&children[0])?.unwrap().values();
        let decorated = ui.bounds(&children[1])?.unwrap().values();
        let rich = ui.bounds(&children[2])?.unwrap().values();
        assert!(decorated[2] > plain[2] + 20.0);
        assert_eq!(&decorated[2..], &rich[2..]);
        assert!(ui.validation_report().issues.is_empty());
    }
    Ok(())
}

#[test]
fn icon_override_iteration_can_use_font_api_and_closed_runtime_stays_closed() -> Result {
    let mut ui = Some(ui());
    let icons = ui.as_ref().unwrap().icons();
    let fonts = ui.as_ref().unwrap().fonts();
    // Iterators are application code; evaluating one must not hold the text borrow.
    icons.set_overrides(std::iter::once_with(|| {
        (Icon::Close, fonts.material_icon("close").unwrap())
    }))?;
    let glyph = fonts.material_icon("save")?;
    let closing = std::iter::once_with(|| {
        drop(ui.take());
        (Icon::Save, glyph)
    });
    assert_eq!(icons.set_overrides(closing), Err(RuntimeError::Closed));
    Ok(())
}

#[test]
fn icon_only_tabs_preserve_names_and_unnamed_tabs_are_rejected() -> Result {
    let content = || Element::new("body", Label::new("Content"));
    let tab = Tab::new("close", "Close", content())
        .rich_title(RichText::new().icon(Icon::Close))
        .into_element();
    assert_eq!(tab.label(), Some("Close"));
    let mut runtime = ui();
    runtime.mount_tree(Element::new("tabs", Tabs::new([])).children([tab]))?;
    draw(&mut runtime);
    for name in ["", "  "] {
        let mut runtime = ui();
        // Described with its tabs, the strip is checked before it mounts.
        assert_eq!(
            runtime
                .mount_tree(Element::new(
                    "tabs",
                    Tabs::new([Tab::new("unnamed", name, content())
                        .rich_title(RichText::new().icon(Icon::Close))]),
                ))
                .err(),
            Some(RuntimeError::Layout(LayoutError::BlankLabel))
        );
        // A raw child is only seen by the update that reads it.
        let tab = Tab::new("unnamed", name, content())
            .rich_title(RichText::new().icon(Icon::Close))
            .into_element();
        runtime.mount_tree(Element::new("tabs", Tabs::new([])).children([tab]))?;
        assert_eq!(
            runtime.flush(Duration::ZERO),
            Err(RuntimeError::Layout(LayoutError::BlankLabel))
        );
    }
    Ok(())
}

#[test]
fn tab_and_section_icon_slots_replace_and_survive_rich_titles() {
    let content = || Element::new("body", Label::new("Content"));
    let tab = Tab::new("mix", "Mix", content())
        .leading_icon(Icon::Settings)
        .leading_icon(Icon::Folder)
        .into_element();
    assert_eq!(tab.label(), Some("Mix"));
    assert_eq!(
        tab.rich_label(),
        Some(&RichText::from("Mix").leading_icon(Icon::Folder))
    );
    let styled = Tab::new("mix", "Mix", content())
        .leading_icon(Icon::Folder)
        .rich_title(RichText::new().text("Mixer").icon(Icon::Close))
        .into_element();
    assert_eq!(styled.label(), Some("Mixer"));
    assert_eq!(
        styled.rich_label(),
        Some(
            &RichText::new()
                .text("Mixer")
                .icon(Icon::Close)
                .leading_icon(Icon::Folder)
        )
    );
    let section = Section::new("general", "General", content())
        .rich_title(RichText::from("General settings"))
        .leading_icon(Icon::Settings)
        .leading_icon(Icon::Folder)
        .into_element();
    assert_eq!(section.label(), Some("General settings"));
    assert_eq!(
        section.rich_label(),
        Some(&RichText::from("General settings").leading_icon(Icon::Folder))
    );
}

struct SizedIcon {
    side: Rc<Cell<f32>>,
}
impl Component for SizedIcon {
    fn update(&mut self, _: &mut UpdateContext) -> std::result::Result<(), RuntimeError> {
        Ok(())
    }
    fn paint(&self, painter: &mut Painter) -> std::result::Result<(), DrawError> {
        painter.icon(
            Icon::Settings,
            Rect::new(0.0, 0.0, self.side.get(), self.side.get())?,
            Color::ACCENT,
        )
    }
}
#[test]
fn standalone_icons_share_resize_buckets_and_paint_above_text_size_limit() -> Result {
    let mut runtime = ui();
    runtime.set_viewport(Viewport::new(800.0, 480.0, 2.0)?)?;
    let side = Rc::new(Cell::new(17.0));
    let id = runtime.mount_tree(Element::new("icon", SizedIcon { side: side.clone() }))?;
    let small = draw(&mut runtime);
    assert!(!small.masks().is_empty());
    for size in [18.0, 25.0, 31.0] {
        side.set(size);
        runtime.invalidate(&id, Invalidation::Paint)?;
        let resized = draw(&mut runtime);
        assert_eq!(
            small.masks()[0].pixels().as_ptr(),
            resized.masks()[0].pixels().as_ptr()
        );
        assert_ne!(small.commands(), resized.commands());
    }
    side.set(160.0);
    runtime.invalidate(&id, Invalidation::Paint)?;
    let large = draw(&mut runtime);
    assert!(!large.masks().is_empty());
    assert!(large.masks()[0].width() <= 512);
    assert!(runtime.flush(Duration::ZERO)?.components_updated == 0);
    assert!(!runtime.needs_paint());
    Ok(())
}

#[test]
fn default_button_and_tab_heights_do_not_depend_on_icons_across_densities() -> Result {
    let mut ui = ui();
    let root = ui.mount_tree(Element::new(
        "controls",
        Column::new([
            Element::new(
                "buttons",
                Row::new([
                    Element::new("text", Button::new("Action", |_| Ok(()))),
                    Element::new("icon", Button::icon(Icon::Add, "Add", |_| Ok(()))),
                    Element::new(
                        "leading",
                        Button::new("Action", |_| Ok(())).leading_icon(Icon::Settings),
                    ),
                    Element::new(
                        "trailing",
                        Button::new("Action", |_| Ok(())).trailing_icon(Icon::Settings),
                    ),
                ]),
            ),
            Element::new(
                "plain-tabs",
                Tabs::new([Tab::new(
                    "plain",
                    "Title",
                    Element::new("body", Label::new("Body")),
                )]),
            ),
            Element::new(
                "icon-tabs",
                Tabs::new([
                    Tab::new("icon", "Title", Element::new("body", Label::new("Body")))
                        .leading_icon(Icon::Settings),
                ]),
            ),
        ]),
    ))?;
    for scale in [1.0, 1.25, 2.0] {
        ui.set_viewport(Viewport::new(800.0, 480.0, scale)?)?;
        for density in [Density::Compact, Density::Standard, Density::Comfortable] {
            ui.theme().set_density(density)?;
            ui.flush(Duration::ZERO)?;
            let rows = ui.children(&root)?;
            let heights = ui
                .children(&rows[0])?
                .iter()
                .map(|id| ui.bounds(id).unwrap().unwrap().height())
                .collect::<Vec<_>>();
            assert!(
                heights.iter().all(|h| *h == heights[0]),
                "{density:?}: {heights:?}"
            );
            assert_eq!(
                ui.bounds(&rows[1])?.unwrap().height(),
                ui.bounds(&rows[2])?.unwrap().height(),
                "{density:?}"
            );
        }
    }
    Ok(())
}
