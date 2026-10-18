//! Coverage the post-repair sweep found missing: redescription conformance for
//! built-in controls and layouts, and adverse cancellation and error paths.
use pert_ui::*;

/// Checks that describing `old` again as `new` presents like a fresh `new`.
#[track_caller]
fn conforms(build: impl Fn(&Runtime, bool) -> Element) {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(600.0, 400.0, 1.0).unwrap())
        .unwrap();
    let (old, new, fresh) = (build(&ui, false), build(&ui, true), build(&ui, true));
    if let Err(error) = check_redescription(&mut ui, old, new, fresh) {
        panic!("{error}: {error:?}");
    }
}

fn leaf(text: &str) -> Element {
    Element::new("leaf", Label::new(text))
}

#[test]
fn actions_and_choices_present_a_new_description_like_a_fresh_mount() {
    conforms(|_, new| {
        Element::new(
            "c",
            Button::new(if new { "New" } else { "Old" }, |_| Ok(())),
        )
    });
    conforms(|_, new| {
        let link = Link::new(if new { "New" } else { "Old" }, |_| Ok(()));
        Element::new("c", if new { link.disabled() } else { link })
    });
    conforms(|_, new| Element::new("c", Chip::new(if new { "New" } else { "Old" })));
    let choices = |new: bool| {
        [
            Choice::new(1, "One"),
            Choice::new(2, if new { "Two!" } else { "Two" }),
        ]
    };
    conforms(|ui, new| {
        let value = ui.signal(if new { 2 } else { 1 });
        Element::new("c", RadioGroup::new(value, choices(new)).unwrap())
    });
    conforms(|ui, new| {
        let value = ui.signal(if new { 2 } else { 1 });
        Element::new("c", SegmentedControl::new(value, choices(new)).unwrap())
    });
    conforms(|ui, new| {
        let value = ui.signal(if new { 2 } else { 1 });
        Element::new("c", Select::new(ui, value, choices(new)).unwrap())
    });
    conforms(|ui, new| {
        let value = ui.signal(if new { vec![2] } else { vec![1] });
        Element::new("c", MultiSelect::new(ui, value, choices(new)).unwrap())
    });
    conforms(|ui, new| {
        let menu = Menu::new(
            ui,
            if new { "New" } else { "Old" },
            vec![MenuItem::new("a", "A", |_| Ok(()))],
        )
        .unwrap();
        Element::new("c", menu)
    });
}

#[test]
fn numeric_and_draft_controls_present_a_new_description_like_a_fresh_mount() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(600.0, 400.0, 1.0).unwrap())
        .unwrap();
    let mut parameter = |id: u128, base: f32| {
        ui.parameter(
            ParameterId::new(1, id, "amount").unwrap(),
            ParameterSpec::new("Amount", 0.0, 100.0, base).unwrap(),
        )
        .unwrap()
        .0
    };
    let (old, new) = (parameter(1, 20.0), parameter(2, 70.0));
    type Build = fn(Parameter<f32>) -> Element;
    let builds: [Build; 3] = [
        |p| Element::new("c", Slider::new(p)),
        |p| Element::new("c", InlineSlider::new(p)),
        |p| Element::new("c", Knob::new(p)),
    ];
    for build in builds {
        let checked = check_redescription(
            &mut ui,
            build(old.clone()),
            build(new.clone()),
            build(new.clone()),
        )
        .unwrap_or_else(|error| panic!("{error}: {error:?}"));
        ui.unmount(&checked.root).unwrap();
    }
    conforms(|_, new| {
        let editor = DraftEditor::new(DraftEditorController::<()>::new());
        Element::new(
            "c",
            if new {
                editor.style(TextStyle::Heading)
            } else {
                editor
            },
        )
    });
}

#[test]
fn layouts_present_a_new_description_like_a_fresh_mount() {
    let text = |new: bool| if new { "New" } else { "Old" };
    let gap = |new: bool| Length::from(if new { 8 } else { 2 });
    conforms(|_, new| Element::new("c", Container::new(leaf(text(new)))));
    conforms(|_, new| Element::new("c", Column::new([leaf(text(new))]).gap(gap(new))));
    conforms(|_, new| Element::new("c", Row::new([leaf(text(new))]).gap(gap(new))));
    conforms(|_, new| {
        let tracks = [Track::weighted(1.0).unwrap(), Track::weighted(2.0).unwrap()];
        Element::new("c", Grid::new(tracks, [leaf(text(new))]).unwrap())
    });
}

/// One root `a` with one child `a/x`.
struct Pair;
impl TreeItems for Pair {
    fn roots(&self) -> Vec<String> {
        vec!["a".into()]
    }
    fn children(&self, key: &str) -> Vec<String> {
        if key == "a" {
            vec!["a/x".into()]
        } else {
            Vec::new()
        }
    }
    fn can_expand(&self, key: &str) -> bool {
        key == "a"
    }
    fn build(&self, key: &str) -> Option<Element> {
        Some(Element::new(key, Label::new(key)))
    }
}

#[test]
fn a_tree_controller_reports_closed_after_shutdown() {
    // COLL-03 documents `Closed` for construction once the runtime shut down.
    let mut ui = Runtime::new();
    let live = TreeController::new(
        &ui,
        TreeSource::new(Pair),
        ListSelection::new(&ui, SelectionMode::Single),
    )
    .unwrap();
    let selection = ListSelection::new(&ui, SelectionMode::Single);
    ui.shutdown();
    assert!(matches!(
        TreeController::new(&ui, TreeSource::new(Pair), selection),
        Err(RuntimeError::Closed)
    ));
    assert!(matches!(
        live.set_expanded("a", true),
        Err(RuntimeError::Closed)
    ));
    assert!(matches!(live.is_expanded("a"), Err(RuntimeError::Closed)));
}

/// Whether the painted scene uses the theme's Error role.
fn paints_error(ui: &mut Runtime) -> bool {
    let error = {
        let theme = ui.theme();
        theme
            .palette(theme.mode().unwrap())
            .unwrap()
            .color(ColorRole::Error)
    };
    let scene = ui.paint().unwrap();
    format!("{:?}", scene.commands()).contains(&format!("{error:?}"))
}

#[test]
fn a_shutdown_during_the_check_returns_the_original_cause() {
    // repairs-conformance noted the cleanup paths as unexecuted. A task that
    // shuts the runtime down fails the helper's first flush. Shutdown retires
    // the helper's root, so cleanup has nothing to release and reports the
    // task's own error rather than `Cleanup`. `Runtime::unmount` fails only
    // for a closed runtime, a phase or a retired identity, and the first two
    // cannot arise while a helper holds `&mut Runtime` outside a phase, so the
    // `Cleanup` variant has no public trigger today.
    let mut ui = Runtime::new();
    let _task = ui
        .defer(|ui| {
            ui.shutdown();
            Err(ApplicationError::from("task failed").into())
        })
        .unwrap();
    let label = || Element::new("c", Label::new("Caption"));
    let error = redescribe(&mut ui, label(), label()).unwrap_err();
    assert!(
        matches!(&error, RedescriptionError::Runtime(RuntimeError::Application(app)) if app.to_string() == "task failed"),
        "the original cause is kept: {error:?}"
    );
}

/// The semantics of the component keyed `slider`.
fn slider_semantics(ui: &Runtime) -> Semantics {
    ui.inspect()
        .components
        .into_iter()
        .find(|c| c.key == "slider")
        .and_then(|c| c.details.semantics)
        .unwrap()
}

#[test]
fn a_rejected_slider_commit_shows_its_error() {
    // repairs-presentation left the rejected-outcome paint branch
    // source-reviewed only. A host write during a held key makes the
    // release's commit rejected.
    for inline in [false, true] {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(400.0, 200.0, 1.0).unwrap())
            .unwrap();
        let (parameter, host) = ui
            .parameter(
                ParameterId::new(1, 1, "amount").unwrap(),
                ParameterSpec::new("Amount", 0.0, 1.0, 0.25).unwrap(),
            )
            .unwrap();
        let control = if inline {
            Element::new("slider", InlineSlider::new(parameter.clone()))
        } else {
            Element::new("slider", Slider::new(parameter.clone()))
        };
        let id = ui.mount_tree(control).unwrap();
        ui.flush(std::time::Duration::ZERO).unwrap();
        ui.focus(&id).unwrap();
        ui.key_input(KeyCode::ArrowRight, KeyAction::Press).unwrap();
        ui.flush(ui.now()).unwrap();
        assert!(!paints_error(&mut ui), "{inline}: a held step is no error");
        host.write_base(0.9).unwrap();
        ui.key_input(KeyCode::ArrowRight, KeyAction::Release)
            .unwrap();
        ui.flush(ui.now()).unwrap();
        assert_eq!(parameter.snapshot().get().unwrap().base, 0.9, "{inline}");
        let rejected = slider_semantics(&ui).invalid;
        assert_eq!(
            rejected,
            paints_error(&mut ui),
            "{inline}: semantics and painting agree"
        );
        assert!(rejected, "{inline}: the refused commit is shown");
    }
}
