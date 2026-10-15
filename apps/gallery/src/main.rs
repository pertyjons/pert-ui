//! The pert-ui component gallery and its checks, built on the public facade.
//!
//! Without arguments it opens the native gallery. One mode argument selects
//! something else instead; `--help` lists them. `--headless` exercises the
//! parameter, key and inspection contracts without opening a window and fails
//! when one of them does not hold, which is how CI runs it. A usage error exits
//! with status 2, a failed mode with status 1.
mod brand;
mod gallery;
mod history;
mod interaction_check;
mod language;
mod live;
mod multi_window;
mod pages;
mod samples;
mod screenshot;
mod search;
#[cfg(test)]
mod tests;
mod workspace_check;
use pert_ui::{
    ComponentDetails, DebugMode, EditOutcome, InspectValue, ParameterSnapshot, RuntimeError,
    prelude::*,
};
use std::{
    collections::BTreeMap, error::Error, ffi::OsString, io::Write, path::PathBuf,
    process::ExitCode, time::Duration,
};

const USAGE: &str = "\
usage: pert-ui-gallery [MODE]

Without a mode, opens the native component gallery. Modes:

  --cpu                            open the native gallery with software
                                   rendering: frames are drawn on the CPU and
                                   no GPU device is created; also combines
                                   with --interaction-check
  --headless                       check the parameter, key and inspection
                                   contracts without a window
  --multi-window                   open the three-window fixture
  --live-benchmark                 measure the synthetic static/live workload
                                   headlessly, then check 30 s of quiescence
  --live-smoke                     open the static/live workload for a five-second
                                   native run
  --zoom-benchmark                 measure headless zoom preparation
  --workspace-check-save PATH      save a changed workspace to a new file
  --workspace-check-restore PATH   restore that file and compare the result
  --interaction-check              drive named gallery targets with injected
                                   input in a native window and check each
  --screenshot [--scale N] [--palette violet] PAGE TARGET PATH
                                   render PAGE (a page key, or - for the first)
                                   headlessly on the CPU at scale N (default 1),
                                   optionally with the dark Violet palette,
                                   and write a PNG of TARGET: window,
                                   region:X,Y,W,H, path:A/B or key:KEY
  --help                           print this help";

/// What one run of the program does.
#[derive(Debug, PartialEq)]
enum Mode {
    Gallery,
    SoftwareGallery,
    Headless,
    MultiWindow,
    LiveBenchmark,
    LiveSmoke,
    ZoomBenchmark,
    WorkspaceSave(PathBuf),
    WorkspaceRestore(PathBuf),
    InteractionCheck,
    SoftwareInteractionCheck,
    Screenshot {
        page: String,
        target: String,
        path: PathBuf,
        scale: f64,
        palette: screenshot::ScreenshotPalette,
    },
    Help,
}

/// Parses the arguments after the program name into exactly one mode.
fn mode(arguments: impl IntoIterator<Item = OsString>) -> Result<Mode, String> {
    let mut arguments = arguments.into_iter();
    let mut chosen: Option<Mode> = None;
    let mut software = false;
    while let Some(argument) = arguments.next() {
        let Some(flag) = argument.to_str() else {
            return Err(format!("unexpected argument {argument:?}"));
        };
        let next = match flag {
            "--help" | "-h" => return Ok(Mode::Help),
            "--headless" => Mode::Headless,
            "--multi-window" => Mode::MultiWindow,
            "--live-benchmark" => Mode::LiveBenchmark,
            "--live-smoke" => Mode::LiveSmoke,
            "--zoom-benchmark" => Mode::ZoomBenchmark,
            "--interaction-check" => Mode::InteractionCheck,
            "--cpu" => {
                if software {
                    return Err("--cpu given twice".into());
                }
                software = true;
                continue;
            }
            "--screenshot" => {
                let mut take = |what: &str| {
                    arguments
                        .next()
                        .ok_or_else(|| format!("--screenshot requires {what}"))
                };
                let mut page = take("a page")?;
                let mut scale = 1.0;
                let mut palette = screenshot::ScreenshotPalette::BuiltIn;
                let mut seen_scale = false;
                let mut seen_palette = false;
                loop {
                    if page == "--scale" && !seen_scale {
                        scale = take("a scale")?
                            .to_str()
                            .and_then(|text| text.parse::<f64>().ok())
                            .filter(|scale| scale.is_finite() && *scale > 0.0)
                            .ok_or("--scale needs a positive number")?;
                        seen_scale = true;
                    } else if page == "--palette" && !seen_palette {
                        if take("a palette")? != "violet" {
                            return Err("--palette accepts violet".into());
                        }
                        palette = screenshot::ScreenshotPalette::Violet;
                        seen_palette = true;
                    } else {
                        break;
                    }
                    page = take("a page")?;
                }
                if page.to_str().is_some_and(|page| page.starts_with("--")) {
                    return Err("unknown or repeated screenshot option".into());
                }
                let page = page
                    .into_string()
                    .map_err(|page| format!("unexpected page {page:?}"))?;
                let target = take("a target")?
                    .into_string()
                    .map_err(|target| format!("unexpected target {target:?}"))?;
                Mode::Screenshot {
                    page,
                    target,
                    path: PathBuf::from(take("a file path")?),
                    scale,
                    palette,
                }
            }
            "--workspace-check-save" | "--workspace-check-restore" => {
                // A path need not be valid Unicode.
                let path = PathBuf::from(
                    arguments
                        .next()
                        .ok_or_else(|| format!("{flag} requires a file path"))?,
                );
                if flag == "--workspace-check-save" {
                    Mode::WorkspaceSave(path)
                } else {
                    Mode::WorkspaceRestore(path)
                }
            }
            other => return Err(format!("unknown argument {other}")),
        };
        if chosen.replace(next).is_some() {
            return Err("choose at most one mode".into());
        }
    }
    match (chosen.unwrap_or(Mode::Gallery), software) {
        (mode, false) => Ok(mode),
        (Mode::Gallery, true) => Ok(Mode::SoftwareGallery),
        (Mode::InteractionCheck, true) => Ok(Mode::SoftwareInteractionCheck),
        (_, true) => Err("--cpu applies to the native gallery and --interaction-check".into()),
    }
}

struct Readout {
    binding: ReadSignal<ParameterSnapshot<f32>>,
    retained: Option<ParameterSnapshot<f32>>,
}

impl Component for Readout {
    fn update(&mut self, context: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.retained = Some(context.read_paint(&self.binding)?);
        Ok(())
    }
    fn inspect(&self) -> ComponentDetails {
        let mut details = ComponentDetails::default();
        if let Some(snapshot) = self.retained {
            details.properties.insert(
                "base".into(),
                InspectValue::Number(f64::from(snapshot.base)),
            );
            details.properties.insert(
                "effective".into(),
                InspectValue::Number(f64::from(snapshot.effective)),
            );
        }
        details
    }
}

/// Fails with `message` unless `holds`.
fn require(holds: bool, message: impl Into<String>) -> Result<(), Box<dyn Error>> {
    if holds {
        Ok(())
    } else {
        Err(format!("headless contract failed: {}", message.into()).into())
    }
}

/// Two readouts share one parameter; a preview refreshes both, a commit is
/// authoritative, simultaneous keys are tracked and cleared on focus loss, the
/// inspection records the events and nothing stays scheduled. Returns the
/// summary line, or an error naming the first contract that did not hold.
fn headless() -> Result<String, Box<dyn Error>> {
    let mut runtime = Runtime::new();
    runtime.set_debug_mode(DebugMode::Inspect);
    let (amount, _host) = runtime.parameter(
        ParameterId::new(1, 1, "amount")?,
        ParameterSpec::new("Amount", -100.0, 100.0, 0.0)?,
    )?;
    let first = runtime.mount(
        "first",
        Readout {
            binding: amount.snapshot(),
            retained: None,
        },
    )?;
    runtime.mount(
        "second",
        Readout {
            binding: amount.snapshot(),
            retained: None,
        },
    )?;
    runtime.flush(Duration::ZERO)?;

    let revision = amount.snapshot().get()?.revision;
    let edit = runtime.try_event(&first, |context| amount.begin_edit(context, revision))??;
    edit.update(25.0)?;
    edit.update(50.0)?;
    let preview = runtime.flush(Duration::ZERO)?;
    let outcome = edit.commit(50.0)?;
    runtime.flush(Duration::ZERO)?;
    drop(edit);
    require(
        preview.components_updated == 2,
        format!(
            "the preview refreshed {} readouts instead of both",
            preview.components_updated
        ),
    )?;
    require(
        matches!(outcome, EditOutcome::Committed { value, .. } if value == 50.0),
        format!("the commit ended as {outcome:?}"),
    )?;
    let inspection = runtime.inspect();
    for key in ["first", "second"] {
        let base = inspection
            .components
            .iter()
            .find(|component| component.key == key)
            .and_then(|component| component.details.properties.get("base").cloned());
        require(
            base == Some(InspectValue::Number(50.0)),
            format!("readout {key} shows {base:?} after the commit"),
        )?;
    }

    runtime.key_input(KeyCode::KeyA, KeyAction::Press)?;
    runtime.key_input(KeyCode::KeyS, KeyAction::Press)?;
    let keys = runtime.keyboard().pressed().count();
    runtime.clear_focus();
    runtime.flush(Duration::ZERO)?;
    require(keys == 2, format!("{keys} of two held keys were tracked"))?;
    let held = runtime.keyboard().pressed().count();
    require(held == 0, format!("{held} held keys survived focus loss"))?;

    let inspection = runtime.inspect_with(|| {
        BTreeMap::from([(
            "example".into(),
            InspectValue::Text("General parameter controls".into()),
        )])
    });
    require(
        !inspection.events.is_empty(),
        "the inspection recorded no diagnostic events",
    )?;
    let idle = runtime.next_deadline().is_none();
    require(idle, "work is still scheduled after the example")?;
    Ok(format!(
        "Headless gallery: {} controls refreshed for the preview; {outcome:?}; {keys} simultaneous keys; {} diagnostic events; idle = {idle}. Headless mode.",
        preview.components_updated,
        inspection.events.len(),
    ))
}

/// Joins the live producer after the event loop returned, reporting the event
/// loop's error first.
fn joined(
    result: Result<(), Box<dyn Error>>,
    producer: &live::ProducerHandle,
) -> Result<(), Box<dyn Error>> {
    let joined = producer.join();
    result?;
    joined?;
    Ok(())
}

fn live_smoke() -> Result<(), Box<dyn Error>> {
    let producer = live::ProducerHandle::default();
    let thread = producer.clone();
    let result = pert_ui::App::new(
        "pert-ui — Static/live smoke (5 seconds, then idle)",
        move |ui| {
            let (workload, source) = live::Workload::with_thread(ui, true, thread)?;
            ui.mount_tree(workload.element()?)?;
            // The producer thread stops after five seconds and drops its
            // endpoints; the sources observe that and schedule nothing more.
            workload
                .spawn(source, Some(Duration::from_secs(5)))
                .map_err(pert_ui::ApplicationError::new)?;
            Ok(())
        },
    )
    .run();
    joined(result.map_err(Into::into), &producer)
}

fn native_gallery(software: bool) -> Result<(), Box<dyn Error>> {
    let gallery = gallery::Gallery::new()?;
    let producer = gallery.producer();
    let title = if software {
        "pert-ui — Component gallery (CPU rendering)"
    } else {
        "pert-ui — Component gallery"
    };
    let app = pert_ui::App::new(title, move |ui| gallery.mount(ui).map(|_| ()));
    let app = if software {
        app.software_rendering()
    } else {
        app
    };
    let result = app.run();
    joined(result.map_err(Into::into), &producer)
}

fn run(mode: Mode) -> Result<(), Box<dyn Error>> {
    match mode {
        Mode::Gallery => native_gallery(false),
        Mode::SoftwareGallery => native_gallery(true),
        Mode::Headless => {
            let summary = headless()?;
            writeln!(std::io::stdout().lock(), "{summary}")?;
            Ok(())
        }
        Mode::MultiWindow => multi_window::run().map_err(Into::into),
        Mode::LiveBenchmark => live::benchmark(),
        Mode::LiveSmoke => live_smoke(),
        Mode::ZoomBenchmark => pages::layout::zoom_benchmark(),
        Mode::WorkspaceSave(path) => workspace_check::run(true, &path),
        Mode::WorkspaceRestore(path) => workspace_check::run(false, &path),
        Mode::InteractionCheck => interaction_check::run(false),
        Mode::SoftwareInteractionCheck => interaction_check::run(true),
        Mode::Screenshot {
            page,
            target,
            path,
            scale,
            palette,
        } => {
            let written = screenshot::run(&page, &target, &path, scale, palette)?;
            writeln!(std::io::stdout().lock(), "{written}")?;
            Ok(())
        }
        Mode::Help => {
            writeln!(std::io::stdout().lock(), "{USAGE}")?;
            Ok(())
        }
    }
}

fn main() -> ExitCode {
    let mode = match mode(std::env::args_os().skip(1)) {
        Ok(mode) => mode,
        Err(message) => {
            eprintln!("pert-ui-gallery: {message}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    match run(mode) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("pert-ui-gallery: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod cli {
    use super::*;

    fn parse(arguments: &[&str]) -> Result<Mode, String> {
        mode(arguments.iter().map(OsString::from))
    }

    #[test]
    fn each_mode_parses_and_none_opens_the_gallery() {
        assert_eq!(parse(&[]), Ok(Mode::Gallery));
        assert_eq!(parse(&["--headless"]), Ok(Mode::Headless));
        assert_eq!(parse(&["--multi-window"]), Ok(Mode::MultiWindow));
        assert_eq!(parse(&["--live-benchmark"]), Ok(Mode::LiveBenchmark));
        assert_eq!(parse(&["--live-smoke"]), Ok(Mode::LiveSmoke));
        assert_eq!(parse(&["--zoom-benchmark"]), Ok(Mode::ZoomBenchmark));
        assert_eq!(parse(&["--interaction-check"]), Ok(Mode::InteractionCheck));
        assert_eq!(
            parse(&["--workspace-check-save", "a.txt"]),
            Ok(Mode::WorkspaceSave("a.txt".into()))
        );
        assert_eq!(
            parse(&["--workspace-check-restore", "a.txt"]),
            Ok(Mode::WorkspaceRestore("a.txt".into()))
        );
        assert_eq!(parse(&["--help"]), Ok(Mode::Help));
        assert_eq!(parse(&["--headless", "--help"]), Ok(Mode::Help));
    }

    #[test]
    fn unknown_conflicting_and_incomplete_arguments_are_usage_errors() {
        for arguments in [
            &["--headless", "--bogus"][..],
            &["--headlesss"],
            &["headless"],
            &["--multi-window", "--headless"],
            &["--headless", "--headless"],
            &["--headless", "--workspace-check-save", "a.txt"],
            &["--workspace-check-save"],
            &["--workspace-check-save", "a.txt", "b.txt"],
            &["--cpu", "--headless"],
            &["--cpu", "--cpu"],
        ] {
            assert!(parse(arguments).is_err(), "{arguments:?} was accepted");
        }
    }

    #[test]
    fn cpu_selects_software_rendering_for_the_native_windows() {
        assert_eq!(parse(&["--cpu"]), Ok(Mode::SoftwareGallery));
        assert_eq!(
            parse(&["--interaction-check", "--cpu"]),
            Ok(Mode::SoftwareInteractionCheck)
        );
        assert_eq!(
            parse(&["--cpu", "--interaction-check"]),
            Ok(Mode::SoftwareInteractionCheck)
        );
    }

    #[test]
    fn screenshot_palette_and_scale_options_parse_in_either_order() {
        for options in [
            ["--palette", "violet", "--scale", "2"],
            ["--scale", "2", "--palette", "violet"],
        ] {
            assert_eq!(
                parse(&[
                    "--screenshot",
                    options[0],
                    options[1],
                    options[2],
                    options[3],
                    "toolbar",
                    "window",
                    "toolbar.png",
                ]),
                Ok(Mode::Screenshot {
                    page: "toolbar".to_owned(),
                    target: "window".to_owned(),
                    path: "toolbar.png".into(),
                    scale: 2.0,
                    palette: screenshot::ScreenshotPalette::Violet,
                })
            );
        }
        for options in [
            &["--palette", "red"][..],
            &["--palette", "violet", "--palette", "violet"],
            &["--palette"],
        ] {
            let mut args = vec!["--screenshot"];
            args.extend_from_slice(options);
            args.extend(["toolbar", "window", "toolbar.png"]);
            assert!(parse(&args).is_err(), "{args:?} was accepted");
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_path_need_not_be_unicode_but_a_flag_must_be() {
        use std::os::unix::ffi::OsStringExt;
        let path = OsString::from_vec(vec![b'a', 0xff]);
        assert_eq!(
            mode([OsString::from("--workspace-check-save"), path.clone()]),
            Ok(Mode::WorkspaceSave(path.clone().into()))
        );
        assert!(mode([path]).is_err());
    }

    /// The headless half of the native interaction check: the same steps, in
    /// the same order, pass on a gallery without a window.
    #[test]
    fn the_interaction_steps_pass_headlessly() {
        let passed = interaction_check::run_headless().unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(passed.len(), interaction_check::steps().len());
    }

    #[test]
    fn the_headless_contracts_hold() {
        let summary = headless().expect("every headless contract holds");
        assert!(summary.contains("2 controls refreshed"), "{summary}");
        assert!(summary.contains("idle = true"), "{summary}");
    }
}
