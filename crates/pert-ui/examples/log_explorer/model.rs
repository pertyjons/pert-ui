//! The application's log data: entries with identities independent of where
//! they are stored, a line parser that rejects malformed lines, and a
//! deterministic generator for the sample log. Nothing here is pert-ui's.
use std::{collections::HashMap, fmt::Write as _, rc::Rc};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Level {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}
impl Level {
    pub const ALL: [Self; 5] = [
        Self::Trace,
        Self::Debug,
        Self::Info,
        Self::Warn,
        Self::Error,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Trace => "TRACE",
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
        }
    }
    fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|level| level.name() == text)
    }
}

/// A line's identity: its line number in the file, or a number past the
/// file's lines for tailed lines. Never a position in a vector.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LineId(pub u64);
impl LineId {
    pub fn key(self) -> String {
        format!("l{}", self.0)
    }
    pub fn from_key(key: &str) -> Option<Self> {
        key.strip_prefix('l')?.parse().ok().map(Self)
    }
}

/// One parsed line, as the worker sends it (`Send`: no `Rc`).
#[derive(Clone, Debug, PartialEq)]
pub struct Parsed {
    pub id: LineId,
    /// `HH:MM:SS.mmm`.
    pub time: String,
    pub level: Level,
    pub source: String,
    pub message: String,
}

/// One stored line.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub id: LineId,
    pub time: String,
    pub level: Level,
    pub source: Rc<str>,
    pub message: String,
}

/// The longest message the views show; longer text is cut for display and
/// kept whole for copying. Display text has a size limit of its own.
pub const PREVIEW: usize = 400;

impl Entry {
    /// The message as the table shows it.
    pub fn preview(&self) -> &str {
        match self.message.char_indices().nth(PREVIEW) {
            Some((end, _)) => &self.message[..end],
            None => &self.message,
        }
    }
    /// The row's searchable text: its cells in column order, separated by a
    /// space, as the table joins `TableItems::text`.
    pub fn searchable(&self) -> String {
        format!(
            "{} {} {} {} {}",
            self.id.0 + 1,
            self.time,
            self.level.name(),
            self.source,
            self.message
        )
    }
    /// The whole line, as copied.
    pub fn line(&self) -> String {
        format!(
            "{} {} {} {}",
            self.time,
            self.level.name(),
            self.source,
            self.message
        )
    }
}

/// `2026-10-19T12:00:00.123 LEVEL source message…`; `None` for a line that
/// does not have that shape, which the loader counts as skipped.
pub fn parse(id: LineId, line: &str) -> Option<Parsed> {
    let mut words = line.splitn(4, ' ');
    let stamp = words.next()?;
    let (date, time) = stamp.split_once('T')?;
    if !valid_date(date) || !valid_time(time) {
        return None;
    }
    let level = Level::parse(words.next()?)?;
    let source = words.next()?.to_owned();
    let message = words.next().unwrap_or("").to_owned();
    Some(Parsed {
        id,
        time: time.to_owned(),
        level,
        source,
        message,
    })
}

/// A number of exactly `digits` ASCII digits at `range` of `text`.
fn number(text: &str, range: std::ops::Range<usize>) -> Option<u32> {
    let part = text.get(range.clone())?;
    (part.len() == range.len() && part.bytes().all(|b| b.is_ascii_digit()))
        .then(|| part.parse().ok())
        .flatten()
}

/// `YYYY-MM-DD` with a month and a day that exist.
fn valid_date(date: &str) -> bool {
    let bytes = date.as_bytes();
    if date.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }
    let (Some(year), Some(month), Some(day)) =
        (number(date, 0..4), number(date, 5..7), number(date, 8..10))
    else {
        return false;
    };
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1..=days).contains(&day)
}

/// `HH:MM:SS.mmm` within a day.
fn valid_time(time: &str) -> bool {
    let bytes = time.as_bytes();
    if time.len() != 12 || bytes[2] != b':' || bytes[5] != b':' || bytes[8] != b'.' {
        return false;
    }
    matches!(
        (
            number(time, 0..2),
            number(time, 3..5),
            number(time, 6..8),
            number(time, 9..12)
        ),
        (Some(0..24), Some(0..60), Some(0..60), Some(_))
    )
}

/// The store: entries in arrival order and an index from identity to
/// position, so lookups never assume that a line number is a position.
#[derive(Default)]
pub struct Store {
    pub entries: Vec<Entry>,
    positions: HashMap<LineId, usize>,
    sources: HashMap<String, Rc<str>>,
}
impl Store {
    pub fn push(&mut self, parsed: Parsed) -> usize {
        let source = self
            .sources
            .entry(parsed.source.clone())
            .or_insert_with(|| Rc::from(parsed.source.as_str()))
            .clone();
        let position = self.entries.len();
        self.positions.insert(parsed.id, position);
        self.entries.push(Entry {
            id: parsed.id,
            time: parsed.time,
            level: parsed.level,
            source,
            message: parsed.message,
        });
        position
    }
    pub fn get(&self, id: LineId) -> Option<&Entry> {
        self.entries.get(*self.positions.get(&id)?)
    }
    pub fn clear(&mut self) {
        self.entries.clear();
        self.positions.clear();
    }
}

pub const SOURCES: [&str; 10] = [
    "auth",
    "db",
    "http",
    "cache",
    "worker-1",
    "worker-2",
    "worker-3",
    "scheduler",
    "mailer",
    "billing",
];

/// A deterministic pseudo-random sample line `index`.
pub fn sample_line(index: u64) -> String {
    let mut x = index
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    let mut next = || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        x
    };
    let level = match next() % 100 {
        0..=19 => Level::Trace,
        20..=49 => Level::Debug,
        50..=89 => Level::Info,
        90..=97 => Level::Warn,
        _ => Level::Error,
    };
    let source = SOURCES[(next() % SOURCES.len() as u64) as usize];
    let millis = index * 37;
    let (h, m, s, ms) = (
        (millis / 3_600_000) % 24,
        (millis / 60_000) % 60,
        (millis / 1000) % 60,
        millis % 1000,
    );
    let mut text = format!(
        "2026-10-19T{h:02}:{m:02}:{s:02}.{ms:03} {} {source} ",
        level.name()
    );
    let words = [
        "request",
        "completed",
        "user",
        "timeout",
        "retry",
        "cache",
        "miss",
        "connection",
        "closed",
        "query",
    ];
    for _ in 0..(4 + next() % 8) {
        let _ = write!(text, "{} ", words[(next() % words.len() as u64) as usize]);
    }
    let _ = write!(text, "id={}", next() % 100_000);
    text
}

/// Every this many lines the sample has one the parser rejects, as real logs
/// have continuation lines and garbage.
pub const MALFORMED_EVERY: u64 = 997;

/// Writes `lines` sample lines to `path`, every [`MALFORMED_EVERY`]th of them
/// malformed.
pub fn generate(path: &std::path::Path, lines: u64) -> std::io::Result<()> {
    use std::io::Write;
    let mut file = std::io::BufWriter::new(std::fs::File::create(path)?);
    for index in 0..lines {
        if index % MALFORMED_EVERY == MALFORMED_EVERY - 1 {
            writeln!(file, "    at continuation of the previous line")?;
        } else {
            writeln!(file, "{}", sample_line(index))?;
        }
    }
    file.flush()
}
