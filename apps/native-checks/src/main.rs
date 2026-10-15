//! Runs the opt-in native checks, each in a process of its own.
//!
//! winit allows one event loop per process, so `cargo test -- --ignored` runs the
//! first window test and fails every later one. This runner builds every test
//! target (the library's unit tests and each integration test) of the packages
//! that hold native checks, asks each test binary for its ignored tests, runs
//! each with `--exact` in a process of its own, then runs the gallery's
//! two-process workspace restart check and its interaction check, the latter
//! also with software rendering (`--cpu`), and reports
//! every outcome together with the platform it ran on. It needs a real desktop session and a hardware GPU
//! adapter; it is evidence for that platform and nothing else.
//!
//! ```sh
//! cargo run --locked -p pert-ui-native-checks            # everything
//! cargo run --locked -p pert-ui-native-checks -- --list  # what would run
//! cargo run --locked -p pert-ui-native-checks -- resize  # names containing "resize"
//! ```
//!
//! Everything is compiled first; the runner then starts each test binary
//! directly rather than through `cargo test`, so the process it times and kills
//! is the check itself and no orphaned test process keeps a window, GPU device
//! or the clipboard while the next check starts. `--timeout SECONDS` bounds each
//! process (120 by default); a check that exceeds it is killed and reported as
//! timed out, never as passed. A test check passes only when its binary exits
//! successfully *and* libtest reports that exactly one test passed, so a check
//! renamed or compiled away between listing and running fails instead of passing
//! without having run.
//! A gallery probe that reports its window was hidden is stopped immediately
//! and fails with that cause rather than waiting for the process timeout.
//!
//! The clipboard check takes over the session clipboard; `--skip-clipboard`
//! leaves it out. Checks left out by name filters or `--skip-clipboard` appear
//! in the summary as `SKIPPED` with the reason, so a partial run cannot pass for
//! a complete one. On macOS, AppKit accepts an event loop only on the process
//! main thread, which libtest does not provide: the window checks report that
//! and are listed as `UNSUPPORTED` rather than failed. The clipboard check and
//! the gallery's restart and interaction checks, which run the gallery program
//! on its own main thread, run there as everywhere else.
//!
//! The summary records the operating system and version, the Linux session type
//! and desktop (with a warning when logind reports the session locked, which
//! withholds frames and fails presentation checks), the GPU adapter and driver
//! as a check reports them on a line starting with `Hardware: ` (text after a
//! `;` on it is left out), the rustc version and the winit and wgpu versions the
//! build resolved. The exit status is 0 when every selected check passed or
//! is unsupported on this platform, 1 when any failed, timed out or could not be
//! started, when the build failed, or when no check matched, and 2 for a usage
//! error.
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    io::{self, BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Command, ExitCode, ExitStatus, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};

/// Packages whose ignored tests are native checks, with the features their
/// tests are built with. An ignored test in any test target of one of these
/// is, by convention, a check that needs a desktop session. The facade adds
/// `cpu` so its software presentation checks are built too.
const PACKAGES: [(&str, &str); 2] = [("pert-ui", "cpu"), ("pert-ui-wgpu", "")];
/// Name fragment of the check that replaces the session clipboard.
const CLIPBOARD: &str = "clipboard";
/// Name under which the gallery's two-process restart check is listed.
const WORKSPACE_RESTART: &str = "pert-ui-gallery::workspace_restart";
/// Name under which the gallery's interaction check is listed. The gallery
/// drives named targets on its pages with injected input in a native window;
/// see `apps/gallery/src/interaction_check.rs` for exactly what it covers.
const INTERACTIONS: &str = "pert-ui-gallery::interaction_check";
/// Name under which the same interaction check with software rendering
/// (`--interaction-check --cpu`) is listed.
const SOFTWARE_INTERACTIONS: &str = "pert-ui-gallery::interaction_check_cpu";
/// The error text of the libtest window checks when they are denied the main
/// thread, which happens on macOS only. The checks print it from
/// `pert-ui/src/platform/smoke.rs` and `pert-ui-wgpu/src/native_smoke.rs`;
/// `markers_match_the_texts_the_checks_print` pins it to both.
const MAIN_THREAD_REQUIRED: &str = "macOS native tests need the process main thread";
/// Prefix of the line on which a check reports the GPU adapter it used, as
/// `Hardware: ADAPTER` optionally followed by `; details`. The pixel readback
/// check in `pert-ui-wgpu/src/native_smoke.rs` prints it;
/// `markers_match_the_texts_the_checks_print` pins it there.
const HARDWARE: &str = "Hardware: ";
/// Gallery probes print one of these lines when native visibility interrupts them.
const INTERRUPTED: [&str; 2] = ["Interaction interrupted:", "Native restart interrupted:"];
/// Dependencies whose resolved versions the summary records.
const RECORDED_DEPENDENCIES: [&str; 2] = ["winit", "wgpu"];
/// How long output still arriving from a finished or killed process is awaited.
const DRAIN_GRACE: Duration = Duration::from_secs(5);
/// Bound on each process unless `--timeout` sets another.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(120);
/// Exit status of a usage error.
const USAGE_ERROR: u8 = 2;
const USAGE: &str = "\
usage: pert-ui-native-checks [OPTIONS] [NAME_FRAGMENT...]

Runs each opt-in native check in a process of its own and prints a summary.

  --list             print the checks that would run and exit
  --skip-clipboard   leave out the check that takes over the session clipboard
  --timeout SECONDS  bound each process (default 120)
  --help             print this help and exit

A name fragment selects the checks whose label contains it.";

/// One test binary built from a native-check package.
#[derive(Clone, Debug, PartialEq)]
struct TestBinary {
    package: String,
    /// Cargo's target kind, such as `lib` or `test`.
    kind: String,
    /// The target's name; the file name of an integration test.
    target: String,
    executable: PathBuf,
    /// The package directory, the working directory `cargo test` would use.
    directory: PathBuf,
}

enum Check {
    /// One ignored libtest test, by binary and full path.
    Test { binary: TestBinary, name: String },
    /// The gallery saves its workspace in one process and restores it in the next.
    WorkspaceRestart,
    /// The gallery drives named page targets with injected input and checks each.
    GalleryInteractions,
    /// The same, with the window presented by the CPU renderer.
    SoftwareInteractions,
}
impl Check {
    fn label(&self) -> String {
        match self {
            Self::Test { binary, name } => match binary.kind.as_str() {
                "lib" => format!("{}::{name}", binary.package),
                "test" => format!("{}/tests/{}::{name}", binary.package, binary.target),
                kind => format!("{}/{kind}/{}::{name}", binary.package, binary.target),
            },
            Self::WorkspaceRestart => WORKSPACE_RESTART.into(),
            Self::GalleryInteractions => INTERACTIONS.into(),
            Self::SoftwareInteractions => SOFTWARE_INTERACTIONS.into(),
        }
    }
}

#[derive(Debug, PartialEq)]
enum Outcome {
    Passed(Duration),
    Failed(String),
    TimedOut(Duration),
    /// The platform cannot run this check; not a failure, not a pass.
    Unsupported(String),
}

#[derive(Debug, PartialEq)]
struct Options {
    help: bool,
    list: bool,
    skip_clipboard: bool,
    timeout: Duration,
    filters: Vec<String>,
}

/// Parses the arguments after the program name.
fn options(arguments: impl IntoIterator<Item = OsString>) -> Result<Options, String> {
    let mut options = Options {
        help: false,
        list: false,
        skip_clipboard: false,
        timeout: DEFAULT_TIMEOUT,
        filters: Vec::new(),
    };
    let mut arguments = arguments.into_iter().map(|argument| {
        argument
            .into_string()
            .map_err(|argument| format!("argument {argument:?} is not valid Unicode"))
    });
    while let Some(argument) = arguments.next() {
        match argument?.as_str() {
            "--help" | "-h" => options.help = true,
            "--list" => options.list = true,
            "--skip-clipboard" => options.skip_clipboard = true,
            "--timeout" => {
                let seconds = arguments
                    .next()
                    .transpose()?
                    .and_then(|value| value.parse::<u64>().ok())
                    .filter(|seconds| *seconds > 0)
                    .ok_or("--timeout takes a positive number of seconds")?;
                options.timeout = Duration::from_secs(seconds);
            }
            flag if flag.starts_with('-') => return Err(format!("unknown option {flag}")),
            filter => options.filters.push(filter.to_owned()),
        }
    }
    Ok(options)
}

fn cargo() -> Command {
    // Cargo sets CARGO for processes it runs, so nested invocations use the same
    // toolchain as the outer one.
    Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
}

/// What one `cargo … --message-format=json` build produced.
#[derive(Debug, Default, PartialEq)]
struct Build {
    /// Built test binaries, or the requested executables of a plain build.
    executables: Vec<TestBinary>,
    /// Resolved versions of the [`RECORDED_DEPENDENCIES`] that were compiled.
    versions: BTreeMap<String, BTreeSet<String>>,
}

impl Build {
    /// Records one JSON message of Cargo's build output. Messages other than
    /// compiler artifacts are ignored.
    fn record(&mut self, message: &Value) {
        if message["reason"] != "compiler-artifact" {
            return;
        }
        let target = &message["target"];
        let name = target["name"].as_str().unwrap_or_default();
        if RECORDED_DEPENDENCIES.contains(&name) {
            // A package ID ends in `#name@version`, or in `#version` when the
            // name is the last path segment of its source.
            let version = message["package_id"]
                .as_str()
                .and_then(|id| id.rsplit(['@', '#']).next())
                .unwrap_or("unknown");
            self.versions
                .entry(name.to_owned())
                .or_default()
                .insert(version.to_owned());
        }
        let (Some(executable), Some(manifest)) = (
            message["executable"].as_str(),
            message["manifest_path"].as_str(),
        ) else {
            return;
        };
        let package = message["package_id"]
            .as_str()
            .and_then(package_name)
            .unwrap_or_default();
        self.executables.push(TestBinary {
            package,
            kind: target["kind"][0].as_str().unwrap_or("unknown").to_owned(),
            target: name.to_owned(),
            executable: executable.into(),
            directory: Path::new(manifest)
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_default(),
        });
    }
}

/// The package name in a Cargo package ID such as
/// `path+file:///work/crates/pert-ui#0.2.22` or
/// `registry+https://…/crates.io-index#winit@0.30.13`.
fn package_name(id: &str) -> Option<String> {
    let (source, fragment) = id.rsplit_once('#')?;
    Some(match fragment.split_once('@') {
        Some((name, _)) => name.to_owned(),
        None => source.rsplit('/').next()?.to_owned(),
    })
}

/// Runs a Cargo build with JSON output and collects what it produced.
fn build(arguments: &[&str]) -> Result<Build, String> {
    let output = cargo()
        .args(arguments)
        .args(["--locked", "--message-format=json-render-diagnostics"])
        .stderr(Stdio::inherit())
        .output()
        .map_err(|error| format!("could not run cargo {}: {error}", arguments.join(" ")))?;
    if !output.status.success() {
        return Err(format!(
            "cargo {} failed with {}",
            arguments.join(" "),
            output.status
        ));
    }
    let mut build = Build::default();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let message: Value = serde_json::from_str(line)
            .map_err(|error| format!("unreadable cargo message {line:?}: {error}"))?;
        build.record(&message);
    }
    Ok(build)
}

/// The ignored tests one test binary reports.
fn ignored_tests(binary: &TestBinary) -> Result<Vec<Check>, String> {
    let output = test_command(binary)
        .args(["--list", "--ignored", "--format", "terse"])
        .stderr(Stdio::inherit())
        .output()
        .map_err(|error| format!("could not run {}: {error}", binary.executable.display()))?;
    if !output.status.success() {
        return Err(format!(
            "listing {} failed with {}",
            binary.executable.display(),
            output.status
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.strip_suffix(": test"))
        .map(|name| Check::Test {
            binary: binary.clone(),
            name: name.to_owned(),
        })
        .collect())
}

/// A command for a test binary with the environment `cargo test` would give it.
fn test_command(binary: &TestBinary) -> Command {
    let mut command = Command::new(&binary.executable);
    command
        .current_dir(&binary.directory)
        .env("CARGO_MANIFEST_DIR", &binary.directory);
    command
}

/// How a process ended.
#[derive(Debug)]
enum Finish {
    Exited(ExitStatus),
    /// Killed once the timeout passed.
    TimedOut(Duration),
    /// Killed after a gallery probe reported that its window was hidden.
    Interrupted(String),
}

/// A process run to its end, with the lines it wrote to stdout and stderr.
#[derive(Debug)]
struct Execution {
    finish: Finish,
    elapsed: Duration,
    output: Vec<String>,
}

/// Copies `stream` line by line to `echo` and to `lines` until it closes.
fn forward(
    stream: impl Read + Send + 'static,
    mut echo: impl Write + Send + 'static,
    lines: mpsc::Sender<String>,
) {
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stream);
        let mut line = Vec::new();
        loop {
            line.clear();
            match reader.read_until(b'\n', &mut line) {
                Ok(0) | Err(_) => return,
                Ok(_) => {
                    // The echo is for the person watching; a closed terminal
                    // must not stop the collection the outcome depends on.
                    let _ = echo.write_all(&line);
                    let text = String::from_utf8_lossy(&line).trim_end().to_owned();
                    if lines.send(text).is_err() {
                        return;
                    }
                }
            }
        }
    });
}

/// Runs `command`, echoing its output, and kills it once `timeout` has passed.
///
/// The error is a failure to start or wait for the process; a timeout is an
/// [`Execution`] whose [`Finish`] says so. Output still arriving after the
/// process ended is awaited for [`DRAIN_GRACE`], so a descendant that keeps the
/// pipes open cannot hold the runner.
fn execute(command: &mut Command, timeout: Duration) -> io::Result<Execution> {
    let start = Instant::now();
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let (sender, receiver) = mpsc::channel();
    if let Some(stdout) = child.stdout.take() {
        forward(stdout, io::stdout(), sender.clone());
    }
    if let Some(stderr) = child.stderr.take() {
        forward(stderr, io::stderr(), sender);
    }
    let mut output = Vec::new();
    let finish = 'running: loop {
        for _ in 0..64 {
            let Ok(line) = receiver.try_recv() else {
                break;
            };
            let interrupted = INTERRUPTED.iter().any(|prefix| line.starts_with(prefix));
            output.push(line.clone());
            if interrupted {
                child.kill()?;
                child.wait()?;
                break 'running Finish::Interrupted(line);
            }
        }
        if let Some(status) = child.try_wait()? {
            break Finish::Exited(status);
        }
        if start.elapsed() >= timeout {
            child.kill()?;
            child.wait()?;
            break Finish::TimedOut(timeout);
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let elapsed = start.elapsed();
    let deadline = Instant::now() + DRAIN_GRACE;
    while let Some(left) = deadline.checked_duration_since(Instant::now()) {
        match receiver.recv_timeout(left) {
            Ok(line) => output.push(line),
            Err(_) => break,
        }
    }
    Ok(Execution {
        finish,
        elapsed,
        output,
    })
}

/// The number of passed tests in libtest's last `test result:` line.
fn passed_count(output: &[String]) -> Option<u64> {
    let summary = output
        .iter()
        .rev()
        .find(|line| line.starts_with("test result:"))?;
    summary.split(';').find_map(|part| {
        let mut words = part.split_whitespace().rev();
        (words.next() == Some("passed"))
            .then(|| words.next()?.parse().ok())
            .flatten()
    })
}

/// Judges a run of one libtest test: it passed only when the binary succeeded
/// and exactly one test passed. `macos` enables the main-thread limitation.
fn test_outcome(execution: &Execution, macos: bool) -> Outcome {
    let status = match execution.finish {
        Finish::TimedOut(limit) => return Outcome::TimedOut(limit),
        Finish::Interrupted(ref reason) => return Outcome::Failed(reason.clone()),
        Finish::Exited(status) => status,
    };
    if macos
        && !status.success()
        && execution
            .output
            .iter()
            .any(|line| line.contains(MAIN_THREAD_REQUIRED))
    {
        return Outcome::Unsupported(
            "a macOS event loop needs the process main thread, which libtest does not provide"
                .into(),
        );
    }
    if !status.success() {
        return Outcome::Failed(status.to_string());
    }
    match passed_count(&execution.output) {
        Some(1) => Outcome::Passed(execution.elapsed),
        Some(count) => Outcome::Failed(format!(
            "{count} tests passed where exactly one was selected; the check did not run as listed"
        )),
        None => Outcome::Failed("libtest printed no result line".into()),
    }
}

/// Judges a run of an ordinary program by its exit status.
fn process_outcome(execution: &Execution) -> Outcome {
    if let Some(reason) = execution
        .output
        .iter()
        .find(|line| INTERRUPTED.iter().any(|prefix| line.starts_with(prefix)))
    {
        return Outcome::Failed(reason.clone());
    }
    match execution.finish {
        Finish::TimedOut(limit) => Outcome::TimedOut(limit),
        Finish::Interrupted(ref reason) => Outcome::Failed(reason.clone()),
        Finish::Exited(status) if status.success() => Outcome::Passed(execution.elapsed),
        Finish::Exited(status) => Outcome::Failed(status.to_string()),
    }
}

/// Runs `command` and judges it; a process that cannot start has failed.
fn run(command: &mut Command, timeout: Duration, judge: impl Fn(&Execution) -> Outcome) -> Outcome {
    match execute(command, timeout) {
        Ok(execution) => judge(&execution),
        Err(error) => Outcome::Failed(format!("could not run: {error}")),
    }
}

fn workspace_restart(gallery: &Path, timeout: Duration) -> Outcome {
    let path: PathBuf = std::env::temp_dir().join(format!(
        "pert-ui-native-checks-{}.workspace",
        std::process::id()
    ));
    let start = Instant::now();
    let mut outcome = Outcome::Passed(Duration::ZERO);
    for mode in ["--workspace-check-save", "--workspace-check-restore"] {
        outcome = run(
            Command::new(gallery).arg(mode).arg(&path),
            timeout,
            process_outcome,
        );
        if !matches!(outcome, Outcome::Passed(_)) {
            break;
        }
    }
    // The file is this run's own; a failed save may not have created it.
    match std::fs::remove_file(&path) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Outcome::Failed(format!("could not remove {path:?}: {error}")),
    }
    match outcome {
        Outcome::Passed(_) => Outcome::Passed(start.elapsed()),
        other => other,
    }
}

/// The first line a program prints, or `None` when it cannot run or fails.
fn first_line(program: impl AsRef<std::ffi::OsStr>, arguments: &[&str]) -> Option<String> {
    let output = Command::new(program)
        .args(arguments)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()
        .filter(|output| output.status.success())?;
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_owned)
}

/// The operating system name and version, as precisely as the platform tells.
fn os_version() -> String {
    let unknown = || "version not detected".to_owned();
    match std::env::consts::OS {
        "linux" => {
            let release = std::fs::read_to_string("/etc/os-release")
                .ok()
                .and_then(|text| {
                    text.lines()
                        .find_map(|line| line.strip_prefix("PRETTY_NAME="))
                        .map(|name| name.trim_matches('"').to_owned())
                })
                .unwrap_or_else(unknown);
            let kernel = std::fs::read_to_string("/proc/sys/kernel/osrelease")
                .map(|kernel| kernel.trim().to_owned())
                .unwrap_or_else(|_| unknown());
            format!("{release}, kernel {kernel}")
        }
        "macos" => match (
            first_line("sw_vers", &["-productVersion"]),
            first_line("sw_vers", &["-buildVersion"]),
        ) {
            (Some(version), Some(build)) => format!("macOS {version} ({build})"),
            (Some(version), None) => format!("macOS {version}"),
            _ => unknown(),
        },
        "windows" => first_line("cmd", &["/C", "ver"]).unwrap_or_else(unknown),
        _ => unknown(),
    }
}

/// Whether logind reports the current Linux session locked, or `None` when
/// that cannot be told (no `loginctl`, no session, another OS). Without
/// `XDG_SESSION_ID`, logind resolves the caller's session as `auto`. A
/// locked session withholds frames, which fails every presentation check.
fn session_locked() -> Option<bool> {
    if !cfg!(target_os = "linux") {
        return None;
    }
    let id = std::env::var("XDG_SESSION_ID").unwrap_or_else(|_| "auto".into());
    let hint = first_line(
        "loginctl",
        &["show-session", &id, "-p", "LockedHint", "--value"],
    )?;
    Some(hint == "yes")
}

/// Warning printed when the session is locked: the compositor withholds frames.
const LOCKED: &str = "logind reports the session locked; the compositor withholds \
                      frames, so presentation checks fail";

/// The Linux session type and desktop from the session's environment.
fn session() -> String {
    let variable = |name: &str| std::env::var(name).ok().filter(|value| !value.is_empty());
    let kind = variable("XDG_SESSION_TYPE").unwrap_or_else(|| "unknown".into());
    let wayland = variable("WAYLAND_DISPLAY").map_or_else(
        || "no WAYLAND_DISPLAY".into(),
        |display| format!("WAYLAND_DISPLAY={display}"),
    );
    let desktop = variable("XDG_CURRENT_DESKTOP").unwrap_or_else(|| "unknown".into());
    format!("{kind} ({wayland}), desktop {desktop}; record the compositor version by hand")
}

fn describe_versions(versions: &BTreeMap<String, BTreeSet<String>>) -> String {
    RECORDED_DEPENDENCIES
        .iter()
        .map(|name| {
            let found = versions.get(*name).map_or_else(
                || "not built".to_owned(),
                |set| set.iter().cloned().collect::<Vec<_>>().join(", "),
            );
            format!("{name} {found}")
        })
        .collect::<Vec<_>>()
        .join("; ")
}

/// Splits `checks` into those to run and the summary lines of those left out.
///
/// A check runs when it matches a name filter (or no filter is given) and is not
/// the clipboard check under `--skip-clipboard`; every other check becomes a
/// `SKIPPED` line naming why. Returns `None` when no check is left to run, which
/// the runner reports as a failure so a mistyped filter cannot pass.
fn select(checks: Vec<Check>, options: &Options) -> Option<(Vec<Check>, Vec<String>)> {
    let mut skipped = Vec::new();
    let selected: Vec<_> = checks
        .into_iter()
        .filter(|check| {
            let label = check.label();
            let reason = if !options.filters.is_empty()
                && !options.filters.iter().any(|filter| label.contains(filter))
            {
                "not selected by the name filters"
            } else if options.skip_clipboard && label.contains(CLIPBOARD) {
                "left out by --skip-clipboard"
            } else {
                return true;
            };
            skipped.push(format!("SKIPPED     {label}: {reason}"));
            false
        })
        .collect();
    (!selected.is_empty()).then_some((selected, skipped))
}

fn main() -> ExitCode {
    let options = match options(std::env::args_os().skip(1)) {
        Ok(options) => options,
        Err(message) => {
            eprintln!("pert-ui-native-checks: {message}\n\n{USAGE}");
            return ExitCode::from(USAGE_ERROR);
        }
    };
    if options.help {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    let mut checks = Vec::new();
    let mut versions = BTreeMap::<String, BTreeSet<String>>::new();
    for (package, features) in PACKAGES {
        let mut arguments = vec!["test", "--no-run", "-p", package, "--tests"];
        if !features.is_empty() {
            arguments.extend(["--features", features]);
        }
        let built = match build(&arguments) {
            Ok(built) => built,
            Err(message) => {
                eprintln!("pert-ui-native-checks: {message}");
                return ExitCode::FAILURE;
            }
        };
        for (name, found) in built.versions {
            versions.entry(name).or_default().extend(found);
        }
        for binary in &built.executables {
            match ignored_tests(binary) {
                Ok(found) => checks.extend(found),
                Err(message) => {
                    eprintln!("pert-ui-native-checks: {message}");
                    return ExitCode::FAILURE;
                }
            }
        }
    }
    checks.push(Check::WorkspaceRestart);
    checks.push(Check::GalleryInteractions);
    checks.push(Check::SoftwareInteractions);
    let Some((checks, skipped)) = select(checks, &options) else {
        eprintln!("pert-ui-native-checks: no check matched");
        return ExitCode::FAILURE;
    };
    if options.list {
        for check in &checks {
            println!("{}", check.label());
        }
        return ExitCode::SUCCESS;
    }
    let gallery = if checks.iter().any(|check| {
        matches!(
            check,
            Check::WorkspaceRestart | Check::GalleryInteractions | Check::SoftwareInteractions
        )
    }) {
        let built = build(&["build", "-p", "pert-ui-gallery", "--bin", "pert-ui-gallery"]);
        match built.map(|built| built.executables.into_iter().next()) {
            Ok(Some(binary)) => Some(binary.executable),
            Ok(None) => {
                eprintln!("pert-ui-native-checks: cargo reported no gallery executable");
                return ExitCode::FAILURE;
            }
            Err(message) => {
                eprintln!("pert-ui-native-checks: {message}");
                return ExitCode::FAILURE;
            }
        }
    } else {
        None
    };
    if session_locked() == Some(true) {
        eprintln!("pert-ui-native-checks: warning: {LOCKED}");
    }
    let macos = cfg!(target_os = "macos");
    let mut failed = 0;
    let mut summary = Vec::new();
    let mut hardware = BTreeSet::new();
    for check in &checks {
        let label = check.label();
        println!("\n=== {label}");
        let outcome = match (check, &gallery) {
            (Check::Test { binary, name }, _) => match execute(
                test_command(binary).args(["--ignored", "--exact", "--nocapture", name]),
                options.timeout,
            ) {
                Ok(execution) => {
                    hardware.extend(
                        execution
                            .output
                            .iter()
                            .filter_map(|line| line.split_once(HARDWARE))
                            .filter_map(|(_, rest)| rest.split(';').next())
                            .map(|adapter| adapter.trim().to_owned()),
                    );
                    test_outcome(&execution, macos)
                }
                Err(error) => Outcome::Failed(format!("could not run: {error}")),
            },
            (Check::WorkspaceRestart, Some(gallery)) => workspace_restart(gallery, options.timeout),
            (Check::GalleryInteractions, Some(gallery)) => run(
                Command::new(gallery).arg("--interaction-check"),
                options.timeout,
                process_outcome,
            ),
            (Check::SoftwareInteractions, Some(gallery)) => run(
                Command::new(gallery).args(["--interaction-check", "--cpu"]),
                options.timeout,
                process_outcome,
            ),
            (
                Check::WorkspaceRestart | Check::GalleryInteractions | Check::SoftwareInteractions,
                None,
            ) => Outcome::Failed("the gallery was not built".into()),
        };
        summary.push(match outcome {
            Outcome::Passed(elapsed) => {
                format!("passed      {label} ({:.1} s)", elapsed.as_secs_f64())
            }
            Outcome::Failed(reason) => {
                failed += 1;
                format!("FAILED      {label}: {reason}")
            }
            Outcome::TimedOut(limit) => {
                failed += 1;
                format!("TIMED OUT   {label} after {} s", limit.as_secs())
            }
            Outcome::Unsupported(reason) => format!("UNSUPPORTED {label}: {reason}"),
        });
    }
    println!(
        "\nNative checks on {} {}:",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    println!("  OS:      {}", os_version());
    if std::env::consts::OS == "linux" {
        println!("  Session: {}", session());
        if session_locked() == Some(true) {
            println!("  Warning: {LOCKED}");
        }
    }
    if hardware.is_empty() {
        println!("  GPU:     not reported; no check that prints its adapter ran");
    }
    for adapter in &hardware {
        println!("  GPU:     {adapter}");
    }
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    println!(
        "  rustc:   {}",
        first_line(rustc, &["--version"]).unwrap_or_else(|| "not detected".into())
    );
    println!("  Crates:  {}", describe_versions(&versions));
    for line in summary.iter().chain(&skipped) {
        println!("  {line}");
    }
    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        eprintln!("{failed} of {} native checks did not pass", checks.len());
        ExitCode::FAILURE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The markers are matched against output that other packages print. The
    /// sources are read at compile time, so a moved file fails the build and a
    /// reworded message fails here instead of silently misclassifying checks.
    #[test]
    fn markers_match_the_texts_the_checks_print() {
        let facade = include_str!("../../../crates/pert-ui/src/platform/smoke.rs");
        let renderer = include_str!("../../../crates/pert-ui-wgpu/src/native_smoke.rs");
        for (file, source) in [("smoke.rs", facade), ("native_smoke.rs", renderer)] {
            assert!(
                source.contains(MAIN_THREAD_REQUIRED),
                "{file} no longer prints {MAIN_THREAD_REQUIRED:?}"
            );
        }
        assert!(
            renderer.contains(&format!("\"{HARDWARE}{{:?}}; ")),
            "native_smoke.rs no longer prints the adapter as {HARDWARE:?} followed by `;`"
        );
    }

    fn test_check(name: &str) -> Check {
        Check::Test {
            binary: TestBinary {
                package: "pert-ui".into(),
                kind: "lib".into(),
                target: "pert_ui".into(),
                executable: PathBuf::from("/w/target/debug/deps/pert_ui-1"),
                directory: PathBuf::from("/w/crates/pert-ui"),
            },
            name: name.into(),
        }
    }

    fn selected(arguments: &[&str]) -> Option<(Vec<String>, Vec<String>)> {
        let checks = vec![
            test_check("platform::smoke::clipboard_round_trip"),
            test_check("platform::smoke::resize"),
            Check::WorkspaceRestart,
            Check::GalleryInteractions,
        ];
        let options = parse(arguments).expect("valid arguments");
        select(checks, &options)
            .map(|(run, skipped)| (run.iter().map(Check::label).collect(), skipped))
    }

    #[test]
    fn without_filters_every_check_is_selected() {
        let (run, skipped) = selected(&[]).expect("checks selected");
        assert_eq!(
            run,
            [
                "pert-ui::platform::smoke::clipboard_round_trip",
                "pert-ui::platform::smoke::resize",
                WORKSPACE_RESTART,
                INTERACTIONS,
            ]
        );
        assert!(skipped.is_empty(), "{skipped:?}");
    }

    #[test]
    fn name_filters_select_matching_labels_and_skip_the_rest_with_a_reason() {
        let (run, skipped) = selected(&["resize", "workspace"]).expect("checks selected");
        assert_eq!(run, ["pert-ui::platform::smoke::resize", WORKSPACE_RESTART]);
        assert_eq!(
            skipped,
            [
                "SKIPPED     pert-ui::platform::smoke::clipboard_round_trip: \
                 not selected by the name filters",
                "SKIPPED     pert-ui-gallery::interaction_check: \
                 not selected by the name filters",
            ]
        );
    }

    #[test]
    fn skip_clipboard_leaves_out_the_clipboard_check_with_its_reason() {
        let (run, skipped) = selected(&["--skip-clipboard"]).expect("checks selected");
        assert_eq!(
            run,
            [
                "pert-ui::platform::smoke::resize",
                WORKSPACE_RESTART,
                INTERACTIONS
            ]
        );
        assert_eq!(
            skipped,
            [
                "SKIPPED     pert-ui::platform::smoke::clipboard_round_trip: \
                 left out by --skip-clipboard"
            ]
        );
    }

    #[test]
    fn no_matching_check_selects_nothing() {
        assert_eq!(selected(&["no-such-check"]), None);
        assert_eq!(selected(&["--skip-clipboard", "clipboard"]), None);
    }

    fn parse(arguments: &[&str]) -> Result<Options, String> {
        options(arguments.iter().map(OsString::from))
    }

    #[test]
    fn options_default_and_combine() {
        assert_eq!(
            parse(&[]),
            Ok(Options {
                help: false,
                list: false,
                skip_clipboard: false,
                timeout: DEFAULT_TIMEOUT,
                filters: Vec::new(),
            })
        );
        let options = parse(&[
            "--list",
            "resize",
            "--skip-clipboard",
            "--timeout",
            "7",
            "gpu",
        ])
        .expect("valid arguments");
        assert!(options.list && options.skip_clipboard && !options.help);
        assert_eq!(options.timeout, Duration::from_secs(7));
        assert_eq!(options.filters, ["resize", "gpu"]);
        assert!(parse(&["--help"]).expect("help").help);
    }

    #[test]
    fn options_reject_invalid_arguments() {
        for arguments in [
            &["--timeout"][..],
            &["--timeout", "0"],
            &["--timeout", "-3"],
            &["--timeout", "soon"],
            &["--bogus"],
        ] {
            assert!(parse(arguments).is_err(), "{arguments:?} was accepted");
        }
        assert_eq!(
            parse(&["--verbose"]),
            Err("unknown option --verbose".into())
        );
    }

    #[cfg(unix)]
    #[test]
    fn options_reject_arguments_that_are_not_unicode() {
        use std::os::unix::ffi::OsStringExt;
        let argument = OsString::from_vec(vec![b'x', 0xff]);
        assert!(options([argument]).is_err());
    }

    /// A command that runs `script` in the platform shell.
    fn shell(script: &str) -> Command {
        if cfg!(windows) {
            let mut command = Command::new("cmd");
            command.args(["/C", script]);
            command
        } else {
            let mut command = Command::new("sh");
            command.args(["-c", script]);
            command
        }
    }

    /// A process that waits far longer than any timeout in these tests, itself
    /// rather than through a shell, so killing it ends it.
    fn sleeper() -> Command {
        if cfg!(windows) {
            let mut command = Command::new("ping");
            command.args(["-n", "60", "127.0.0.1"]);
            command
        } else {
            let mut command = Command::new("sleep");
            command.arg("60");
            command
        }
    }

    const LONG: Duration = Duration::from_secs(30);

    #[test]
    fn a_successful_process_passes_with_its_output() {
        let execution = execute(&mut shell("echo ready"), LONG).expect("runs");
        assert!(matches!(execution.finish, Finish::Exited(status) if status.success()));
        assert_eq!(execution.output, ["ready"]);
        assert!(matches!(process_outcome(&execution), Outcome::Passed(_)));
    }

    #[test]
    fn a_failing_process_fails() {
        let outcome = run(&mut shell("exit 1"), LONG, process_outcome);
        assert!(matches!(outcome, Outcome::Failed(_)), "{outcome:?}");
    }

    #[test]
    fn a_process_past_its_timeout_is_killed_and_never_passes() {
        let limit = Duration::from_millis(300);
        let start = Instant::now();
        let execution = execute(&mut sleeper(), limit).expect("runs");
        assert!(start.elapsed() < LONG, "the sleeper was not killed");
        assert!(matches!(execution.finish, Finish::TimedOut(at) if at == limit));
        assert_eq!(process_outcome(&execution), Outcome::TimedOut(limit));
        assert_eq!(test_outcome(&execution, false), Outcome::TimedOut(limit));
        let outcome = run(&mut sleeper(), limit, process_outcome);
        assert_eq!(outcome, Outcome::TimedOut(limit));
    }

    #[cfg(unix)]
    #[test]
    fn a_hidden_gallery_probe_fails_immediately_with_its_cause() {
        let start = Instant::now();
        let execution = execute(
            &mut shell("echo 'Interaction interrupted: hidden window'; while :; do :; done"),
            LONG,
        )
        .expect("runs");
        assert!(start.elapsed() < Duration::from_secs(5));
        assert!(matches!(
            process_outcome(&execution),
            Outcome::Failed(reason) if reason == "Interaction interrupted: hidden window"
        ));
    }

    #[test]
    fn a_missing_program_fails_to_start() {
        let mut command = Command::new("pert-ui-native-checks-no-such-program");
        assert!(execute(&mut command, LONG).is_err());
        let outcome = run(&mut command, LONG, process_outcome);
        assert!(
            matches!(&outcome, Outcome::Failed(reason) if reason.starts_with("could not run")),
            "{outcome:?}"
        );
    }

    fn exited(script: &str, output: &[&str]) -> Execution {
        let mut execution = execute(&mut shell(script), LONG).expect("runs");
        execution.output = output.iter().map(|line| (*line).to_owned()).collect();
        execution
    }

    #[test]
    fn a_test_passes_only_when_exactly_one_test_passed() {
        let one = exited(
            "exit 0",
            &[
                "running 1 test",
                "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.20s",
            ],
        );
        assert!(matches!(test_outcome(&one, false), Outcome::Passed(_)));
        let none = exited(
            "exit 0",
            &[
                "running 0 tests",
                "test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 0.00s",
            ],
        );
        assert!(matches!(test_outcome(&none, false), Outcome::Failed(_)));
        let silent = exited("exit 0", &[]);
        assert!(matches!(test_outcome(&silent, false), Outcome::Failed(_)));
        let failed = exited(
            "exit 101",
            &["test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 4 filtered out"],
        );
        assert!(matches!(test_outcome(&failed, false), Outcome::Failed(_)));
    }

    #[test]
    fn the_main_thread_limitation_is_unsupported_only_on_macos() {
        let denied = exited(
            "exit 101",
            &[
                "Error: EventLoop(\"macOS native tests need the process main thread, which libtest does not provide\")",
                "test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 4 filtered out",
            ],
        );
        assert!(matches!(
            test_outcome(&denied, true),
            Outcome::Unsupported(_)
        ));
        assert!(matches!(test_outcome(&denied, false), Outcome::Failed(_)));
    }

    #[test]
    fn build_messages_yield_test_binaries_and_dependency_versions() {
        let mut build = Build::default();
        for line in [
            r#"{"reason":"compiler-artifact","package_id":"registry+https://github.com/rust-lang/crates.io-index#winit@0.30.13","manifest_path":"/r/winit/Cargo.toml","target":{"kind":["lib"],"name":"winit"},"executable":null}"#,
            r#"{"reason":"compiler-artifact","package_id":"path+file:///w/crates/pert-ui#0.2.22","manifest_path":"/w/crates/pert-ui/Cargo.toml","target":{"kind":["lib"],"name":"pert_ui"},"executable":"/w/target/debug/deps/pert_ui-1"}"#,
            r#"{"reason":"compiler-artifact","package_id":"path+file:///w/crates/pert-ui#0.2.22","manifest_path":"/w/crates/pert-ui/Cargo.toml","target":{"kind":["test"],"name":"windows"},"executable":"/w/target/debug/deps/windows-2"}"#,
            r#"{"reason":"build-finished","success":true}"#,
        ] {
            build.record(&serde_json::from_str(line).expect("valid JSON"));
        }
        assert_eq!(
            build.versions,
            BTreeMap::from([("winit".into(), BTreeSet::from(["0.30.13".into()]))])
        );
        let labels: Vec<_> = build
            .executables
            .iter()
            .map(|binary| {
                assert_eq!(binary.package, "pert-ui");
                assert_eq!(binary.directory, Path::new("/w/crates/pert-ui"));
                Check::Test {
                    binary: binary.clone(),
                    name: "case".into(),
                }
                .label()
            })
            .collect();
        assert_eq!(labels, ["pert-ui::case", "pert-ui/tests/windows::case"]);
    }

    #[test]
    fn package_names_come_from_either_package_id_form() {
        assert_eq!(
            package_name("path+file:///w/apps/gallery#pert-ui-gallery@0.2.22").as_deref(),
            Some("pert-ui-gallery")
        );
        assert_eq!(
            package_name("path+file:///w/crates/pert-ui-wgpu#0.2.22").as_deref(),
            Some("pert-ui-wgpu")
        );
        assert_eq!(package_name("no fragment"), None);
    }
}
