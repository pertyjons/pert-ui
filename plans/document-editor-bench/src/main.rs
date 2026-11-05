//! Edit latency and heap use of `TextArea` on the current path, headless.
//!
//! See plans/document-editor.md, "Baseline". Every number this prints comes
//! from one run on one machine; the plan records the machine and the run.
//! Build and run with `cargo run --release` and a target directory outside the
//! pert-ui repository.
use pert_ui_runtime::{
    Element, InspectValue, KeyAction, KeyCode, RegionLayout, Runtime, Signal, TextArea, Viewport,
    WidgetId,
};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
use std::time::{Duration, Instant};

/// Counts live and peak heap bytes allocated through the Rust global
/// allocator. Allocations outside it (none are expected headless) are missed.
struct Counting;
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
// SAFETY: forwards every call to `System` unchanged and only updates counters.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(layout) };
        if !p.is_null() {
            let live = LIVE.fetch_add(layout.size(), Relaxed) + layout.size();
            PEAK.fetch_max(live, Relaxed);
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, layout: Layout) {
        unsafe { System.dealloc(p, layout) };
        LIVE.fetch_sub(layout.size(), Relaxed);
    }
    unsafe fn realloc(&self, p: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let q = unsafe { System.realloc(p, layout, size) };
        if !q.is_null() {
            if size >= layout.size() {
                let live = LIVE.fetch_add(size - layout.size(), Relaxed) + size - layout.size();
                PEAK.fetch_max(live, Relaxed);
            } else {
                LIVE.fetch_sub(layout.size() - size, Relaxed);
            }
        }
        q
    }
}
#[global_allocator]
static ALLOC: Counting = Counting;

fn live() -> usize {
    LIVE.load(Relaxed)
}
fn reset_peak() -> usize {
    let now = live();
    PEAK.store(now, Relaxed);
    now
}
fn kib(bytes: usize) -> f64 {
    bytes as f64 / 1024.0
}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Deterministic ASCII English-like prose of exactly `bytes` bytes:
/// sentences of 6-18 words, paragraphs of 3-7 sentences separated by a blank
/// line (two LFs). The last word may be cut.
fn prose(bytes: usize) -> String {
    const WORDS: &[&str] = &[
        "the",
        "of",
        "and",
        "a",
        "to",
        "in",
        "is",
        "you",
        "that",
        "it",
        "he",
        "was",
        "for",
        "on",
        "are",
        "as",
        "with",
        "his",
        "they",
        "at",
        "be",
        "this",
        "have",
        "from",
        "or",
        "one",
        "had",
        "by",
        "word",
        "but",
        "not",
        "what",
        "all",
        "were",
        "when",
        "we",
        "there",
        "can",
        "window",
        "editor",
        "document",
        "paragraph",
        "selection",
        "measurement",
        "between",
        "latency",
        "memory",
        "through",
        "application",
        "boundary",
        "keyboard",
        "composition",
        "river",
        "mountain",
        "evening",
        "quietly",
        "remarkable",
        "unfortunately",
        "library",
    ];
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = |n: usize| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state % n as u64) as usize
    };
    let mut text = String::with_capacity(bytes + 64);
    while text.len() < bytes {
        if !text.is_empty() {
            text.push_str("\n\n");
        }
        for s in 0..3 + next(5) {
            if s > 0 {
                text.push(' ');
            }
            for w in 0..6 + next(13) {
                let word = WORDS[next(WORDS.len())];
                if w == 0 {
                    let mut c = word.chars();
                    let first = c.next().unwrap_or('x').to_ascii_uppercase();
                    text.push(first);
                    text.push_str(c.as_str());
                } else {
                    text.push(' ');
                    text.push_str(word);
                }
            }
            text.push('.');
        }
    }
    text.truncate(bytes);
    text
}

struct Ui {
    ui: Runtime,
    value: Signal<String>,
    editor: WidgetId,
    clock: Duration,
}
impl Ui {
    /// A TextArea filling a `width` x `height` logical viewport at `scale`,
    /// as the center of a RegionLayout.
    fn new(text: String, width: f32, height: f32, scale: f64) -> Result<Self> {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(width, height, scale)?)?;
        let value = ui.signal(text);
        let editor = Element::new("editor", TextArea::new(value.clone()));
        let root = ui.mount_tree(Element::new("root", RegionLayout::new().center(editor)))?;
        let editor = ui.find_descendant(&root, "editor")?.ok_or("no editor")?;
        let mut this = Self {
            ui,
            value,
            editor,
            clock: Duration::ZERO,
        };
        this.ui.focus(&this.editor.clone())?;
        this.frame().map_err(|e| format!("first frame: {e}"))?;
        Ok(this)
    }
    fn frame(&mut self) -> Result<()> {
        self.clock += Duration::from_millis(1);
        self.ui
            .flush(self.clock)
            .map_err(|e| format!("flush: {e}"))?;
        self.ui.paint().map_err(|e| format!("paint: {e}"))?;
        Ok(())
    }
    fn key(&mut self, code: KeyCode) -> Result<()> {
        self.ui.key_input(code, KeyAction::Press)?;
        self.ui.key_input(code, KeyAction::Release)?;
        Ok(())
    }
    fn property(&self, name: &str) -> Option<InspectValue> {
        self.ui
            .inspect()
            .components
            .into_iter()
            .find(|c| c.key == "editor")
            .and_then(|c| c.details.properties.get(name).cloned())
    }
    fn caret(&self) -> usize {
        match self.property("selection") {
            Some(InspectValue::Text(s)) => s
                .split("..")
                .nth(1)
                .and_then(|f| f.parse().ok())
                .unwrap_or(0),
            _ => 0,
        }
    }
    /// Moves the caret down by displayed lines until it reaches the middle.
    fn caret_to_middle(&mut self) -> Result<usize> {
        let middle = self.value.get()?.len() / 2;
        for _ in 0..10_000 {
            if self.caret() >= middle {
                break;
            }
            self.key(KeyCode::ArrowDown)?;
            self.frame()?;
        }
        Ok(self.caret())
    }
}

#[derive(Default)]
struct Series {
    total: Vec<f64>,
    input: Vec<f64>,
    flush: Vec<f64>,
    paint: Vec<f64>,
}
fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}
fn percentile(values: &[f64], p: f64) -> f64 {
    let mut v = values.to_vec();
    v.sort_by(f64::total_cmp);
    let i = ((v.len() - 1) as f64 * p).round() as usize;
    v[i]
}
fn summary(values: &[f64]) -> String {
    format!(
        "median {:7.3}  p95 {:7.3}  max {:7.3}",
        percentile(values, 0.5),
        percentile(values, 0.95),
        percentile(values, 1.0)
    )
}

const WARMUP: usize = 5;
const SAMPLES: usize = 60;

/// Times `SAMPLES` edits after `WARMUP`, each followed by a flush and paint.
/// Every edit changes the text, so no edit hits the prepared-text cache.
fn edits(ui: &mut Ui, enter: bool) -> Result<Series> {
    let mut s = Series::default();
    for i in 0..WARMUP + SAMPLES {
        let t0 = Instant::now();
        if enter {
            ui.key(KeyCode::Enter)?;
        } else {
            ui.ui.text_input("x")?;
        }
        let t1 = Instant::now();
        ui.clock += Duration::from_millis(1);
        ui.ui.flush(ui.clock)?;
        let t2 = Instant::now();
        ui.ui.paint()?;
        let t3 = Instant::now();
        if i >= WARMUP {
            s.total.push(ms(t3 - t0));
            s.input.push(ms(t1 - t0));
            s.flush.push(ms(t2 - t1));
            s.paint.push(ms(t3 - t2));
        }
    }
    Ok(s)
}

fn measure(target: usize) -> Result<()> {
    // Headroom so every timed edit fits under the 16 KiB limit.
    let headroom = 2 * (WARMUP + SAMPLES) + 8;
    let start = target - headroom;
    for enter in [false, true] {
        let before = live();
        let mut ui = Ui::new(prose(start), 800.0, 600.0, 1.0)?;
        let mounted = live() - before;
        let middle = ui.caret_to_middle()?;
        let settled = reset_peak();
        let series = edits(&mut ui, enter)?;
        let peak = PEAK.load(Relaxed) - settled;
        let retained = live() as isize - settled as isize;
        let end = ui.value.get()?.len();
        assert_eq!(end, start + WARMUP + SAMPLES, "every edit landed");
        println!(
            "{:>5} B start, {:>5} B end, caret {:>5} | {:<6} total {} ms",
            start,
            end,
            middle,
            if enter { "Enter" } else { "insert" },
            summary(&series.total)
        );
        println!(
            "      input median {:7.3}  flush median {:7.3}  paint median {:7.3} ms",
            percentile(&series.input, 0.5),
            percentile(&series.flush, 0.5),
            percentile(&series.paint, 0.5)
        );
        println!(
            "      heap: after mount+frame {:8.1} KiB, transient peak above settled during series {:8.1} KiB, retained after series {:8.1} KiB",
            kib(mounted),
            kib(peak),
            retained as f64 / 1024.0
        );
        drop(ui);
    }
    Ok(())
}

fn outcome<T: std::fmt::Debug>(r: &Result<T>) -> String {
    match r {
        Ok(v) => format!("Ok({v:?})"),
        Err(e) => format!("Err({e})"),
    }
}

fn limits() -> Result<()> {
    const MAX: usize = pert_ui_runtime::TextEditError::MAX_TEXT_BYTES;
    println!("-- limits (MAX_TEXT_BYTES = {MAX})");
    // A binding one byte over the limit.
    {
        let before = live();
        let mut ui = Ui::new(prose(MAX + 1), 800.0, 600.0, 1.0)?;
        println!(
            "binding of {} B at mount: frame Ok; binding_error {:?}; shown text length {:?}; heap after mount {:.1} KiB",
            MAX + 1,
            ui.property("binding_error"),
            ui.property("text").map(|t| match t {
                InspectValue::Text(t) => t.len(),
                _ => usize::MAX,
            }),
            kib(live() - before)
        );
        let typed = ui.ui.text_input("x");
        let framed = ui.frame();
        println!(
            "  typing into it: text_input {}, frame {}, binding length {}",
            outcome(&typed.map_err(Into::into)),
            outcome(&framed),
            ui.value.get()?.len()
        );
    }
    // An editor exactly at the limit refuses a further byte.
    {
        let mut ui = Ui::new(prose(MAX), 800.0, 600.0, 1.0)?;
        let typed = ui.ui.text_input("x");
        let framed = ui.frame();
        println!(
            "binding of {MAX} B, typing one byte: text_input {}, frame {}, binding length {}, binding_error {:?}",
            outcome(&typed.map_err(Into::into)),
            outcome(&framed),
            ui.value.get()?.len(),
            ui.property("binding_error")
        );
    }
    // The raster side below the byte limit: a narrow editor at scale 1 and
    // a wide one at scale 2.
    for (label, width, height, scale, bytes) in [
        ("320 x 600 filled, scale 1", 320.0, 600.0, 1.0, MAX),
        ("320 x 600 filled, scale 1", 320.0, 600.0, 1.0, MAX / 2),
        ("800 x 600 filled, scale 2", 800.0, 600.0, 2.0, MAX),
        ("800 x 600 filled, scale 2", 800.0, 600.0, 2.0, MAX / 2),
    ] {
        let r = Ui::new(prose(bytes), width, height, scale);
        println!(
            "{label}, {bytes} B at mount: {}",
            outcome(&r.as_ref().map(|_| ()).map_err(|e| e.to_string().into()))
        );
    }
    // Typing line breaks into a narrow editor until preparation refuses:
    // where does the failure surface?
    {
        let mut ui = Ui::new(prose(MAX / 2), 320.0, 600.0, 1.0)?;
        ui.caret_to_middle()?;
        let mut presses = 0;
        let result = loop {
            presses += 1;
            if presses > 2000 {
                break "no failure in 2000 presses".to_owned();
            }
            if let Err(e) = ui.key(KeyCode::Enter) {
                break format!("key_input failed: {e}");
            }
            if let Err(e) = ui.frame() {
                break format!("frame failed: {e}");
            }
        };
        let after = ui.frame();
        println!(
            "Enter repeatedly, 320 x 600 filled, scale 1, from {} B: after {presses} presses {result}; binding length {}; next frame {}",
            MAX / 2,
            ui.value.get()?.len(),
            outcome(&after)
        );
    }
    Ok(())
}

fn main() -> Result<()> {
    println!(
        "-- edit latency: TextArea filling 800 x 600 logical px at scale 1, monospace default"
    );
    for target in [4 * 1024, 8 * 1024, 16 * 1024] {
        measure(target)?;
    }
    limits()?;
    Ok(())
}
