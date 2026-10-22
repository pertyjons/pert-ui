//! A searchable picker over application-supplied items, shared by the
//! command palette and other pickers such as history or search results.
use crate::layout::rect;
use crate::*;
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::{Rc, Weak},
};

/// One row a [`QuickPick`] can offer.
///
/// The fields are plain data, readable by a [`PickMatcher`]; the builder
/// methods set them in a chain. The `key` identifies the row within one opened
/// session: the picker keys its list rows by it, keeps the highlight on it and
/// hands the chosen item back to the application, so it must be unique and
/// not blank, which [`QuickPickController::open`] checks. A disabled item is
/// still listed and can still be chosen, because choosing it is where the
/// application explains why it is unavailable; it is drawn muted and reported
/// disabled.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::PickItem;
/// let item = PickItem::new("page/colors", "Colors").detail("Controls").kind("Page");
/// assert_eq!(item.key, "page/colors");
/// assert_eq!(item.label, "Colors");
/// assert_eq!(item.detail.as_deref(), Some("Controls"));
/// assert_eq!(item.kind.as_deref(), Some("Page"));
/// assert!(!item.disabled);
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
#[must_use]
pub struct PickItem {
    /// The row's identity within a session; unique and not blank.
    pub key: String,
    /// The row's text and, by default, what the matcher searches.
    pub label: String,
    /// Text shown at the end of the row, for example a location or a state.
    /// It yields to the label and is shortened with an ellipsis when the row
    /// is too narrow for both.
    pub detail: Option<String>,
    /// What kind of item this is ("Command", "Page", "Search result"), shown
    /// after the detail, so one picker can list items from several sources.
    pub kind: Option<String>,
    /// Keyboard shortcut text reported to inspection and assistive technology.
    pub shortcut: Option<String>,
    /// Whether the item is disabled; see the type documentation.
    pub disabled: bool,
}
impl PickItem {
    /// An enabled item with `key` and `label` and nothing else.
    pub fn new(key: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            detail: None,
            kind: None,
            shortcut: None,
            disabled: false,
        }
    }
    /// Sets the `detail` field.
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
    /// Sets the `kind` field.
    pub fn kind(mut self, kind: impl Into<String>) -> Self {
        self.kind = Some(kind.into());
        self
    }
    /// Sets the `shortcut` field.
    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }
    /// Sets the `disabled` field.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// Scores and orders [`PickItem`] rows for a query: a [`QuickPick`]'s
/// replaceable matching and ranking policy.
///
/// A matcher is a pure function of the query and one item that returns `None`
/// when the item does not match and otherwise a score, where higher ranks
/// earlier. It must be deterministic, bounded and free of side effects: it
/// runs on the UI thread for every item each time the query or the session
/// changes, and must not write bindings or perform I/O. [`Self::rank`] applies
/// it to a list, so ranking is testable without a window or a runtime.
///
/// The default, [`Self::fuzzy`], matches the query against the item's label
/// with [`Self::fuzzy_score`].
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{PickItem, PickMatcher};
/// let items = [
///     PickItem::new("a", "Save"),
///     PickItem::new("b", "Save As…"),
///     PickItem::new("c", "Show Sidebar").detail("View"),
/// ];
/// let labels = |order: Vec<usize>| -> Vec<&str> {
///     order.into_iter().map(|index| items[index].label.as_str()).collect()
/// };
/// assert_eq!(labels(PickMatcher::fuzzy().rank("sav", &items)), ["Save", "Save As…"]);
/// // A replacement matcher, here one that also searches the detail, decides alone.
/// let detail = PickMatcher::new(|query, item| {
///     item.detail.as_deref().and_then(|d| PickMatcher::fuzzy_score(query, d))
/// });
/// assert_eq!(labels(detail.rank("view", &items)), ["Show Sidebar"]);
/// ```
#[derive(Clone)]
#[must_use]
pub struct PickMatcher(Rc<Score>);
/// A matcher's scoring function.
type Score = dyn Fn(&str, &PickItem) -> Option<u32>;
impl PickMatcher {
    /// Uses `score` as the matching and ranking policy; see the type
    /// documentation for its rules.
    pub fn new(score: impl Fn(&str, &PickItem) -> Option<u32> + 'static) -> Self {
        Self(Rc::new(score))
    }
    /// The default policy: [`Self::fuzzy_score`] of the query against the
    /// item's label.
    pub fn fuzzy() -> Self {
        Self::new(|query, item| Self::fuzzy_score(query, &item.label))
    }
    /// The score of one item, or `None` when it does not match.
    #[must_use]
    pub fn score(&self, query: &str, item: &PickItem) -> Option<u32> {
        (self.0)(query, item)
    }
    /// The positions in `items` of the matching items, highest score first and
    /// ties in the given order. Items that do not match are left out; disabled
    /// items are ranked like any other.
    #[must_use]
    pub fn rank(&self, query: &str, items: &[PickItem]) -> Vec<usize> {
        let mut scored: Vec<(u32, usize)> = items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| self.score(query, item).map(|score| (score, index)))
            .collect();
        // Stable: equal scores keep the given order.
        scored.sort_by_key(|scored| std::cmp::Reverse(scored.0));
        scored.into_iter().map(|(_, index)| index).collect()
    }
    /// A case-insensitive subsequence match of `query` against `text`,
    /// ignoring whitespace in the query.
    ///
    /// Returns `Some(0)` for a query without other characters, so an empty
    /// query keeps every item in the given order, and `None` when the query's
    /// characters do not all appear in `text` in order. Otherwise the score is
    /// higher for matches at the start of `text` or of a word (after a
    /// non-alphanumeric character or at a lower-to-upper case change), for
    /// consecutive matched characters, and when the query appears as one
    /// contiguous run. Characters are compared after Unicode lowercase
    /// mapping. Matching is greedy and leftmost, so the score is a
    /// deterministic heuristic rather than the best possible alignment; the
    /// work is linear in the combined length of `query` and `text`, including
    /// the search for the contiguous run.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::PickMatcher;
    /// assert_eq!(PickMatcher::fuzzy_score("", "Anything"), Some(0));
    /// assert_eq!(PickMatcher::fuzzy_score("xyz", "Save"), None);
    /// let word = PickMatcher::fuzzy_score("ss", "Show Sidebar").unwrap();
    /// let inner = PickMatcher::fuzzy_score("ss", "Class list").unwrap();
    /// assert!(word > inner);
    /// ```
    #[must_use]
    pub fn fuzzy_score(query: &str, text: &str) -> Option<u32> {
        const MATCH: u32 = 1;
        const WORD_START: u32 = 8;
        const TEXT_START: u32 = 8;
        const CONSECUTIVE: u32 = 4;
        const CONTIGUOUS: u32 = 16;
        let needle: Vec<char> = query
            .chars()
            .filter(|c| !c.is_whitespace())
            .flat_map(char::to_lowercase)
            .collect();
        if needle.is_empty() {
            return Some(0);
        }
        // Lowercased text with a word-start flag on the first character each
        // original character produced.
        let mut hay: Vec<(char, bool)> = Vec::with_capacity(text.len());
        let mut previous: Option<char> = None;
        for c in text.chars() {
            let start = previous
                .is_none_or(|p| !p.is_alphanumeric() || (p.is_lowercase() && c.is_uppercase()));
            for (n, lower) in c.to_lowercase().enumerate() {
                hay.push((lower, start && n == 0));
            }
            previous = Some(c);
        }
        let mut score = 0u32;
        let mut next = 0;
        let mut last: Option<usize> = None;
        for (index, (c, start)) in hay.iter().enumerate() {
            if next < needle.len() && *c == needle[next] {
                score = score.saturating_add(MATCH);
                if *start {
                    score = score.saturating_add(WORD_START);
                }
                if index == 0 {
                    score = score.saturating_add(TEXT_START);
                }
                if last.is_some_and(|last| last + 1 == index) {
                    score = score.saturating_add(CONSECUTIVE);
                }
                last = Some(index);
                next += 1;
            }
        }
        if next < needle.len() {
            return None;
        }
        if let Some(at) = contiguous_run(&needle, &hay) {
            score = score.saturating_add(CONTIGUOUS);
            if hay[at].1 {
                score = score.saturating_add(CONTIGUOUS);
            }
        }
        Some(score)
    }
}
/// The first position in `hay` where `needle` occurs as one contiguous run,
/// found with the Knuth–Morris–Pratt failure table so the work is linear in
/// the lengths of both, whatever their repetitions.
fn contiguous_run(needle: &[char], hay: &[(char, bool)]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    // failure[i]: length of the longest proper prefix of needle[..=i] that is
    // also its suffix.
    let mut failure = vec![0usize; needle.len()];
    let mut matched = 0;
    for i in 1..needle.len() {
        while matched > 0 && needle[i] != needle[matched] {
            matched = failure[matched - 1];
        }
        if needle[i] == needle[matched] {
            matched += 1;
        }
        failure[i] = matched;
    }
    matched = 0;
    for (index, (c, _)) in hay.iter().enumerate() {
        while matched > 0 && *c != needle[matched] {
            matched = failure[matched - 1];
        }
        if *c == needle[matched] {
            matched += 1;
            if matched == needle.len() {
                return Some(index + 1 - needle.len());
            }
        }
    }
    None
}
impl Default for PickMatcher {
    /// [`Self::fuzzy`].
    fn default() -> Self {
        Self::fuzzy()
    }
}
/// Shows the policy without its function.
impl std::fmt::Debug for PickMatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PickMatcher").finish_non_exhaustive()
    }
}

/// Where a [`QuickPickController`]'s session stands, as
/// [`QuickPickController::status`] reports it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum PickStatus {
    /// Nothing is presented.
    Closed,
    /// The picker is presented, possibly showing why the last choice did not
    /// succeed.
    Open,
    /// An item was chosen and the application reported its result as
    /// pending with [`PickChoice::Pending`]: the picker has closed, and ends
    /// `Closed` when the result succeeds and `Open`, showing the message, when
    /// it fails.
    Pending,
}

/// A result the application finishes later, such as a queued command or a
/// reveal: a binding read by the picker, which reacts when it reports a
/// terminal result.
///
/// The `resolve` function maps the binding's value to `None` while the result
/// is pending, `Some(Ok(()))` when it succeeded or was cancelled (the picker
/// stays closed), and `Some(Err(message))` when it failed (the picker reopens
/// showing `message`). It must be pure and bounded.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{PickPending, Runtime};
/// let ui = Runtime::new();
/// let done = ui.signal(None::<bool>);
/// let pending = PickPending::new(done.read_only(), |value| match value {
///     None => None,
///     Some(true) => Some(Ok(())),
///     Some(false) => Some(Err("It did not work.".to_owned())),
/// });
/// assert_eq!(pending.result()?, None);
/// done.set(Some(false))?;
/// assert_eq!(pending.result()?, Some(Err("It did not work.".to_owned())));
/// # Ok::<(), pert_ui::RuntimeError>(())
/// ```
#[derive(Clone)]
#[must_use]
pub struct PickPending(Rc<Poll>);
/// Reads a pending result, with dependencies when given an update context.
type Poll = dyn Fn(Option<&mut UpdateContext>) -> Result<Option<Result<(), String>>, RuntimeError>;
impl PickPending {
    /// A pending result read from `binding` through `resolve`; see the type
    /// documentation.
    pub fn new<S: Clone + 'static>(
        binding: impl Into<ReadSignal<S>>,
        resolve: impl Fn(&S) -> Option<Result<(), String>> + 'static,
    ) -> Self {
        let binding: ReadSignal<S> = binding.into();
        Self(Rc::new(move |cx| {
            let value = match cx {
                Some(cx) => cx.read(&binding)?,
                None => binding.get()?,
            };
            Ok(resolve(&value))
        }))
    }
    /// The current result: `None` while pending, otherwise the terminal
    /// result. Reading creates no subscription.
    ///
    /// # Errors
    /// Returns the binding's read errors, such as `Closed` after shutdown.
    pub fn result(&self) -> Result<Option<Result<(), String>>, RuntimeError> {
        (self.0)(None)
    }
    fn read(&self, cx: &mut UpdateContext) -> Result<Option<Result<(), String>>, RuntimeError> {
        (self.0)(Some(cx))
    }
}
/// Shows only the type.
impl std::fmt::Debug for PickPending {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PickPending").finish_non_exhaustive()
    }
}

/// What the application did with a chosen item, returned from the
/// [`QuickPick`]'s choose callback.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum PickChoice {
    /// The choice is complete: the picker closes and the session ends.
    Done,
    /// The choice finishes later; see [`PickPending`]. The picker closes now.
    Pending(PickPending),
    /// The choice was refused at once: the picker stays open and shows the
    /// message above the list.
    Refused(String),
}

/// One opened session: the items and their keys.
#[derive(Clone)]
struct Session {
    generation: u64,
    items: Rc<[PickItem]>,
    keys: Rc<HashMap<String, usize>>,
}
impl PartialEq for Session {
    fn eq(&self, other: &Self) -> bool {
        self.generation == other.generation
    }
}

#[derive(Clone)]
enum Phase {
    Closed,
    Open { message: Option<String> },
    Pending { choice: u64, pending: PickPending },
}
impl PartialEq for Phase {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Closed, Self::Closed) => true,
            (Self::Open { message: a }, Self::Open { message: b }) => a == b,
            (Self::Pending { choice: a, .. }, Self::Pending { choice: b, .. }) => a == b,
            _ => false,
        }
    }
}

/// The user's highlight, tied to the query it was chosen under so that a new
/// query starts again at the best match.
#[derive(Clone, PartialEq)]
struct Highlight {
    query: String,
    item: usize,
}

/// Opens, closes and observes one [`QuickPick`].
///
/// The controller owns the picker's session: the items it lists, the query,
/// the highlight and the state of the last choice. Clones address the same
/// session. One mounted picker presents a controller at a time; it claims the
/// controller when it mounts, and a second picker mounted with it stays closed
/// until the first unmounts. It then presents the controller's next session,
/// but not one the first picker was presenting when it unmounted.
///
/// Whether the user can open a picker at all is the application's decision:
/// nothing opens it except [`Self::open`], called from the application's own
/// commands, buttons or shortcuts. Reads and writes follow signal phase rules:
/// open and close from event callbacks, command actions and application code,
/// never from `update`.
#[derive(Clone)]
#[must_use]
pub struct QuickPickController {
    query: Signal<String>,
    session: Signal<Option<Session>>,
    phase: Signal<Phase>,
    highlight: Signal<Option<Highlight>>,
    list: ListController,
    owner: Rc<RefCell<Weak<()>>>,
    generation: Rc<Cell<u64>>,
    choices: Rc<Cell<u64>>,
    /// The session whose picker unmounted while it was presented.
    orphaned: Rc<Cell<u64>>,
}
/// Shows the status, or the error reading it after runtime shutdown.
impl std::fmt::Debug for QuickPickController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QuickPickController")
            .field("status", &self.status())
            .finish_non_exhaustive()
    }
}
impl QuickPickController {
    /// Creates a closed controller belonging to `ui`.
    ///
    /// `ui` is any [`StateOwner`](crate::StateOwner): the
    /// [`Runtime`](crate::Runtime) while the application builds its tree, or
    /// the [`UpdateContext`](crate::UpdateContext) or
    /// [`EventContext`](crate::EventContext) of a component that builds this
    /// while it describes its children.
    pub fn new(ui: &impl crate::StateOwner) -> Self {
        Self {
            query: ui.signal(String::new()),
            session: ui.signal(None),
            phase: ui.signal(Phase::Closed),
            highlight: ui.signal(None),
            list: ListController::new(ui),
            owner: Rc::new(RefCell::new(Weak::new())),
            generation: Rc::new(Cell::new(0)),
            choices: Rc::new(Cell::new(0)),
            orphaned: Rc::new(Cell::new(0)),
        }
    }
    fn same(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.owner, &other.owner)
    }
    /// Opens the picker over `items`, listed in the given order for an empty
    /// query and ranked by the picker's [`PickMatcher`] otherwise.
    ///
    /// Opening while a session is open replaces it: the query is cleared and
    /// the highlight returns to the first item. A choice still pending from
    /// an earlier session no longer reopens the picker.
    ///
    /// # Errors
    /// Returns [`LayoutError::InvalidChild`] for a blank or duplicate item key
    /// without opening, and the signal write errors, such as
    /// `WriteDuringUpdate` or `Closed` after shutdown.
    pub fn open(&self, items: Vec<PickItem>) -> Result<(), RuntimeError> {
        let mut keys = HashMap::with_capacity(items.len());
        for (index, item) in items.iter().enumerate() {
            if item.key.trim().is_empty() || keys.insert(item.key.clone(), index).is_some() {
                return Err(LayoutError::InvalidChild.into());
            }
        }
        let generation = self.generation.get().wrapping_add(1);
        self.generation.set(generation);
        self.session.set(Some(Session {
            generation,
            items: items.into(),
            keys: Rc::new(keys),
        }))?;
        self.query.set(String::new())?;
        self.highlight.set(None)?;
        self.phase.set(Phase::Open { message: None })?;
        Ok(())
    }
    /// Closes a presented picker, including one showing why a choice failed.
    /// Returns whether it was presented. While a choice is pending the picker
    /// is not presented, so this changes nothing then and the result can
    /// still reopen it.
    ///
    /// # Errors
    /// Returns the signal read and write errors, such as `WriteDuringUpdate`
    /// or `Closed` after shutdown.
    pub fn close(&self) -> Result<bool, RuntimeError> {
        let open = self.status()? == PickStatus::Open;
        if open {
            self.phase.set(Phase::Closed)?;
        }
        Ok(open)
    }
    /// The session's current status. Reading creates no subscription.
    ///
    /// # Errors
    /// Returns the signal read errors, such as `Closed` after shutdown.
    pub fn status(&self) -> Result<PickStatus, RuntimeError> {
        let phase = self.phase.get()?;
        let result = match &phase {
            Phase::Pending { pending, .. } => pending.result()?,
            _ => None,
        };
        let session = self.session.get()?;
        Ok(self.presented(&phase, result, session.as_ref()).0)
    }
    /// The status a phase presents, with the message for a failed choice.
    fn presented(
        &self,
        phase: &Phase,
        result: Option<Result<(), String>>,
        session: Option<&Session>,
    ) -> (PickStatus, Option<String>) {
        let orphaned = session.is_some_and(|s| s.generation == self.orphaned.get());
        match phase {
            Phase::Closed => (PickStatus::Closed, None),
            Phase::Open { .. } if orphaned => (PickStatus::Closed, None),
            Phase::Open { message } => (PickStatus::Open, message.clone()),
            Phase::Pending { .. } => match result {
                None => (PickStatus::Pending, None),
                Some(Ok(())) => (PickStatus::Closed, None),
                Some(Err(message)) if !orphaned => (PickStatus::Open, Some(message)),
                Some(Err(_)) => (PickStatus::Closed, None),
            },
        }
    }
    /// Reads the presentation with dependencies, for a component's update.
    fn read_presented(
        &self,
        cx: &mut UpdateContext,
    ) -> Result<(PickStatus, Option<String>), RuntimeError> {
        let phase = cx.read(&self.phase)?;
        let session = cx.read(&self.session)?;
        let result = match &phase {
            Phase::Pending { pending, .. } => pending.read(cx)?,
            _ => None,
        };
        Ok(self.presented(&phase, result, session.as_ref()))
    }
    /// Hands the item at `index` of the open session to `choose` and applies
    /// its answer.
    fn choose(
        &self,
        cx: &mut EventContext,
        index: usize,
        choose: &RefCell<ChooseFn>,
    ) -> Result<(), RuntimeError> {
        let Some(session) = self.session.get()? else {
            return Ok(());
        };
        let Some(item) = session.items.get(index) else {
            return Ok(());
        };
        let generation = session.generation;
        let choice = {
            let mut choose = choose
                .try_borrow_mut()
                .map_err(|_| RuntimeError::BorrowConflict)?;
            choose(cx, item)?
        };
        // The callback may have opened a newer session through its captured
        // controller; its answer belongs to the session it was chosen from.
        if self.session.get()?.map(|session| session.generation) != Some(generation) {
            return Ok(());
        }
        match choice {
            PickChoice::Done => self.phase.set(Phase::Closed)?,
            PickChoice::Pending(pending) => {
                let choice = self.choices.get().wrapping_add(1);
                self.choices.set(choice);
                self.phase.set(Phase::Pending { choice, pending })?
            }
            PickChoice::Refused(message) => self.phase.set(Phase::Open {
                message: Some(message),
            })?,
        };
        Ok(())
    }
}

/// The application's handling of a chosen item.
type ChooseFn = Box<dyn FnMut(&mut EventContext, &PickItem) -> Result<PickChoice, RuntimeError>>;

/// The picker's configuration, applied from the next opening.
#[derive(Clone)]
struct Config {
    matcher: PickMatcher,
    /// Fixed texts, bindings or roles of the installed string table.
    placeholder: TextSource,
    empty_text: TextSource,
    name: TextSource,
    list_name: TextSource,
    width: f32,
}

/// The configurable texts as last read, in the order of [`Config`].
#[derive(Clone, Default, PartialEq)]
struct Texts {
    placeholder: String,
    empty_text: String,
    name: String,
    list_name: String,
}

/// The ranked view of the open session that the content, its rows and its key
/// handler share.
#[derive(Default)]
struct View {
    query: String,
    session: Option<Session>,
    ranked: Rc<[usize]>,
}
impl View {
    /// The highlighted item: the user's choice while it was made under the
    /// current query and still matches, otherwise the best match.
    fn effective(&self, highlight: Option<&Highlight>) -> Option<usize> {
        highlight
            .filter(|h| h.query == self.query && self.ranked.contains(&h.item))
            .map(|h| h.item)
            .or_else(|| self.ranked.first().copied())
    }
    fn key_of(&self, index: usize) -> Option<String> {
        self.session
            .as_ref()
            .and_then(|session| session.items.get(index))
            .map(|item| item.key.clone())
    }
}

struct Shared {
    controller: QuickPickController,
    config: RefCell<Config>,
    choose: Rc<RefCell<ChooseFn>>,
    modal: Cell<bool>,
    view: RefCell<View>,
    /// The configurable texts as last read by the picker or its content;
    /// `None` until the first read.
    texts: RefCell<Option<Texts>>,
}
impl Shared {
    /// One of the configurable texts as last read. Before the first read,
    /// fixed text and the installed table's text for a role stand in, so the
    /// picker is named from the start; a binding is empty until read.
    fn text(&self, read: impl Fn(&Texts) -> &String) -> String {
        if let Some(texts) = self.texts.borrow().as_ref() {
            return read(texts).clone();
        }
        let strings = self.controller.query.shared().strings.borrow().clone();
        let initial = |source: &TextSource| {
            source
                .fixed()
                .map(RichText::plain_text)
                .or_else(|| source.role().map(|role| strings.text(role).to_owned()))
                .unwrap_or_default()
        };
        let config = self.config.borrow();
        read(&Texts {
            placeholder: initial(&config.placeholder),
            empty_text: initial(&config.empty_text),
            name: initial(&config.name),
            list_name: initial(&config.list_name),
        })
        .clone()
    }
    /// Reads the configurable texts in an update, subscribing the component
    /// to their bindings and to the string table for roles. Returns whether
    /// any changed.
    fn read_texts(&self, cx: &mut UpdateContext) -> Result<bool, RuntimeError> {
        let sources = {
            let config = self.config.borrow();
            [
                config.placeholder.clone(),
                config.empty_text.clone(),
                config.name.clone(),
                config.list_name.clone(),
            ]
        };
        let [placeholder, empty_text, name, list_name] = sources;
        let texts = Texts {
            placeholder: placeholder.read(cx)?.plain_text(),
            empty_text: empty_text.read(cx)?.plain_text(),
            name: name.read(cx)?.plain_text(),
            list_name: list_name.read(cx)?.plain_text(),
        };
        let changed = self.texts.borrow().as_ref() != Some(&texts);
        if changed {
            *self.texts.borrow_mut() = Some(texts);
        }
        Ok(changed)
    }
    /// Moves the highlight by `step` rows, wrapping for single steps and
    /// stopping at the ends for pages, and scrolls the row into view.
    fn step(&self, step: isize, wrap: bool) -> Result<(), RuntimeError> {
        let highlight = self.controller.highlight.get()?;
        let view = self.view.borrow();
        let len = view.ranked.len();
        if len == 0 {
            return Ok(());
        }
        let current = view
            .effective(highlight.as_ref())
            .and_then(|item| view.ranked.iter().position(|e| *e == item))
            .unwrap_or(0);
        let len = isize::try_from(len).map_err(|_| RuntimeError::Exhausted)?;
        let current = isize::try_from(current).map_err(|_| RuntimeError::Exhausted)?;
        let next = if wrap {
            (current + step).rem_euclid(len)
        } else {
            (current + step).clamp(0, len - 1)
        };
        let item = view.ranked[usize::try_from(next).map_err(|_| RuntimeError::Exhausted)?];
        let query = view.query.clone();
        let key = view.key_of(item);
        drop(view);
        self.controller
            .highlight
            .set(Some(Highlight { query, item }))?;
        if let Some(key) = key {
            // The receipt is not needed: a mounted row is enough.
            let _ = self.controller.list.reveal(key)?;
        }
        Ok(())
    }
    fn choose(&self, cx: &mut EventContext, index: usize) -> Result<(), RuntimeError> {
        let choose = self.choose.clone();
        self.controller.choose(cx, index, &choose)
    }
    fn on_key(&self, cx: &mut EventContext, key: SearchKey) -> Result<(), RuntimeError> {
        let page = isize::try_from(QuickPick::VISIBLE_ROWS).unwrap_or(isize::MAX);
        match key {
            SearchKey::Up => self.step(-1, true),
            SearchKey::Down => self.step(1, true),
            SearchKey::PageUp => self.step(-page, false),
            SearchKey::PageDown => self.step(page, false),
            SearchKey::Submit => {
                let highlight = self.controller.highlight.get()?;
                let item = self.view.borrow().effective(highlight.as_ref());
                match item {
                    Some(item) => self.choose(cx, item),
                    None => Ok(()),
                }
            }
        }
    }
}

/// A searchable overlay that lists application-supplied [`PickItem`]s and
/// hands the one the user chooses to the application.
///
/// The picker wraps the application's content (`anchor`), which it lays out
/// as its own allocation, and presents over it, when its
/// [`QuickPickController`] opens, a panel with a [`SearchField`] and a
/// virtualized list of the session's items. Each row shows the item's label
/// at the start and its detail and kind at the end, muted when the item is
/// disabled. The label has priority: it takes the width it needs, leaving the
/// detail up to a third of the row or whatever the label leaves, and either
/// text that does not fit ends in an ellipsis on one line.
/// [`CommandPalette`] is a `QuickPick` over the registered commands;
/// history lists and search results use the same interaction.
///
/// - **Surface.** Window-modal by default: the panel sits near the top of the
///   window (one control height below its top edge) over a dimmed backdrop,
///   Tab stays inside it, and a press on the backdrop closes it.
///   [`Self::nonmodal`] presents it instead as a nonmodal surface at the top
///   of the anchor, which closes on an outside press and when focus leaves it.
///   Escape, hiding or removing the picker, and window focus loss close it
///   either way. Opening focuses the search field, and closing restores the
///   previously focused component.
/// - **Filtering and ranking** are the [`PickMatcher`]'s, which is pure;
///   [`Self::matcher`] replaces it. An empty query lists every item in the
///   given order.
/// - **Keyboard.** Typing edits the query; Up and Down move the highlight,
///   wrapping at the ends, and Page Up and Page Down move it by
///   [`Self::VISIBLE_ROWS`], all without leaving the field. A new query
///   highlights its best match. Enter chooses the highlighted item. Escape
///   first clears a non-empty query and then closes the picker, as
///   [`SearchField`] documents; a pointer press and release on a row chooses
///   it.
/// - **Choosing.** The choose callback runs in the event phase with the
///   chosen item, disabled ones included, and answers with a [`PickChoice`]:
///   [`PickChoice::Done`] closes the picker, [`PickChoice::Pending`] closes it
///   and reopens it with a message if the result later fails, and
///   [`PickChoice::Refused`] keeps it open with a message above the list, so
///   no choice is skipped silently. An error from the callback is returned
///   from the input call and leaves the picker as it was. Since `Done` and
///   `Pending` close the picker, queue follow-up work with
///   [`EventContext::defer_detached`]: work queued with
///   [`EventContext::defer`] is bound to a widget inside the picker and is
///   cancelled when the picker closes before the flush delivers it.
///
/// Configuration ([`Self::matcher`], [`Self::placeholder`],
/// [`Self::empty_text`], [`Self::name`], [`Self::list_name`], [`Self::width`],
/// [`Self::nonmodal`]) applies from the next opening when the picker is
/// described again, and a new description's choose callback replaces the old
/// one. The controller is fixed when the picker mounts; a description with a
/// different controller is ignored, so give a picker with another controller
/// a different key.
/// A failed Pending choice reopens the same session with its existing settings;
/// only a new controller opening adopts the latest description. Closed inspection
/// keeps the last session's configured name. String-table changes still resolve
/// the active configuration's text roles immediately. Equivalent descriptions
/// schedule no layout; choose callbacks are replaced immediately.
///
/// Inspection reports a `Dialog` (modal) or `Popover` with the picker's name
/// and `open`, `status` and `modal` properties, and the results as a
/// `ListBox` whose items carry each row's label, shortcut, highlight and
/// disabled state.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{
///     Element, KeyAction, KeyCode, Label, PickChoice, PickItem, PickStatus, QuickPick,
///     QuickPickController, Runtime,
/// };
/// use std::{cell::RefCell, rc::Rc, time::Duration};
/// let mut ui = Runtime::new();
/// ui.set_viewport(pert_ui::Viewport::new(800.0, 600.0, 1.0)?)?;
/// let picker = QuickPickController::new(&ui);
/// let chosen = Rc::new(RefCell::new(None));
/// let record = chosen.clone();
/// ui.mount_tree(Element::new(
///     "app",
///     QuickPick::new(
///         picker.clone(),
///         Element::new("doc", Label::new("Document")),
///         move |_, item| {
///             *record.borrow_mut() = Some(item.key.clone());
///             Ok(PickChoice::Done)
///         },
///     )
///     .name("Go to page"),
/// ))?;
/// picker.open(vec![
///     PickItem::new("colors", "Colors").kind("Page"),
///     PickItem::new("knobs", "Knobs").kind("Page"),
/// ])?;
/// ui.flush(Duration::ZERO)?;
/// // Focus is in the search field: type, then choose the best match.
/// ui.text_input("kn")?;
/// ui.key_input(KeyCode::Enter, KeyAction::Press)?;
/// ui.flush(Duration::ZERO)?;
/// assert_eq!(chosen.borrow().as_deref(), Some("knobs"));
/// assert_eq!(picker.status()?, PickStatus::Closed);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[must_use]
pub struct QuickPick {
    shared: Rc<Shared>,
    token: Rc<()>,
    anchor: Option<Element>,
    described_modal: bool,
    pending_config: Option<Config>,
    session_generation: Option<u64>,
    shown: bool,
    status: PickStatus,
}
impl QuickPick {
    /// The preferred width of the panel, 520 logical pixels, narrowed to the
    /// window; [`Self::width`] changes it.
    pub const DEFAULT_WIDTH: Length = Length::trusted(520.0);
    /// The number of rows the list shows before it scrolls, and the distance
    /// Page Up and Page Down move the highlight.
    pub const VISIBLE_ROWS: usize = 10;

    /// Wraps `anchor` and presents the picker `controller` opens over it,
    /// calling `choose` with each item the user chooses; see the type
    /// documentation. The controller is claimed when the picker mounts, not
    /// here.
    pub fn new(
        controller: QuickPickController,
        anchor: Element,
        choose: impl FnMut(&mut EventContext, &PickItem) -> Result<PickChoice, RuntimeError> + 'static,
    ) -> Self {
        Self {
            shared: Rc::new(Shared {
                controller,
                config: RefCell::new(Config {
                    matcher: PickMatcher::fuzzy(),
                    placeholder: crate::StringRole::SearchPlaceholder.into(),
                    empty_text: crate::StringRole::NoMatches.into(),
                    name: crate::StringRole::PickerName.into(),
                    list_name: crate::StringRole::PickerResults.into(),
                    width: Self::DEFAULT_WIDTH.get(),
                }),
                choose: Rc::new(RefCell::new(Box::new(choose))),
                modal: Cell::new(true),
                view: RefCell::new(View::default()),
                texts: RefCell::new(None),
            }),
            token: Rc::new(()),
            anchor: Some(anchor),
            described_modal: true,
            pending_config: None,
            session_generation: None,
            shown: false,
            status: PickStatus::Closed,
        }
    }
    /// Presents the picker as a nonmodal surface at the top of the anchor
    /// instead of a window-modal one; see the type documentation.
    pub fn nonmodal(mut self) -> Self {
        self.described_modal = false;
        self
    }
    /// Replaces the matching and ranking policy, [`PickMatcher::fuzzy`] by
    /// default.
    pub fn matcher(self, matcher: PickMatcher) -> Self {
        self.shared.config.borrow_mut().matcher = matcher;
        self
    }
    /// Sets the search field's hint text as a [`TextSource`]: fixed text, a
    /// binding the hint follows, or a [`StringRole`](crate::StringRole) of the
    /// installed string table. By default the installed
    /// [`UiStrings`](crate::UiStrings) supply
    /// [`StringRole::SearchPlaceholder`](crate::StringRole::SearchPlaceholder),
    /// "Search" in English. Only the plain text is used. Like the other
    /// settings, it applies from the next opening; a binding or role is read
    /// while the picker is mounted, so a change relabels an open picker.
    pub fn placeholder(self, text: impl Into<TextSource>) -> Self {
        self.shared.config.borrow_mut().placeholder = text.into();
        self
    }
    /// Sets the text shown when no item matches the query, a [`TextSource`]
    /// like [`Self::placeholder`]. Without it the string table supplies
    /// [`StringRole::NoMatches`](crate::StringRole::NoMatches), "No matches"
    /// in English.
    pub fn empty_text(self, text: impl Into<TextSource>) -> Self {
        self.shared.config.borrow_mut().empty_text = text.into();
        self
    }
    /// Sets the name inspection and assistive technology report for the
    /// panel. Without it the string table supplies
    /// [`StringRole::PickerName`](crate::StringRole::PickerName), "Quick pick"
    /// in English. A [`TextSource`] like [`Self::placeholder`]; only its plain
    /// text is used.
    pub fn name(self, name: impl Into<TextSource>) -> Self {
        self.shared.config.borrow_mut().name = name.into();
        self
    }
    /// Sets the name reported for the result list. Without it the string
    /// table supplies [`StringRole::PickerResults`](crate::StringRole::PickerResults),
    /// "Results" in English. A [`TextSource`] like [`Self::placeholder`]; only
    /// its plain text is used.
    pub fn list_name(self, name: impl Into<TextSource>) -> Self {
        self.shared.config.borrow_mut().list_name = name.into();
        self
    }
    /// Sets the panel's preferred width, [`Self::DEFAULT_WIDTH`] by default;
    /// it is narrowed to the window. Any width greater than zero is valid;
    /// [`Length`] already excludes non-finite and negative values.
    ///
    /// # Errors
    /// Returns [`GeometryError::Empty`] for zero.
    pub fn width(self, width: Length) -> Result<Self, GeometryError> {
        if width.get() == 0.0 {
            return Err(GeometryError::Empty);
        }
        self.shared.config.borrow_mut().width = width.get();
        Ok(self)
    }
    fn owns(&self) -> bool {
        self.shared
            .controller
            .owner
            .borrow()
            .upgrade()
            .is_some_and(|owner| Rc::ptr_eq(&owner, &self.token))
    }
    /// Checks description ownership independently of presentation invalidation.
    pub(crate) fn same_controller(&self, next: &Self) -> bool {
        self.shared.controller.same(&next.shared.controller)
    }
    fn claim(&self) -> bool {
        let mut owner = self.shared.controller.owner.borrow_mut();
        match owner.upgrade() {
            Some(current) if !Rc::ptr_eq(&current, &self.token) => false,
            _ => {
                *owner = Rc::downgrade(&self.token);
                true
            }
        }
    }
    fn kind(&self) -> SurfaceKind {
        if self.shared.modal.get() {
            SurfaceKind::Dialog
        } else {
            SurfaceKind::ContextMenu
        }
    }
    /// The status this picker presents, from its last update.
    pub(crate) fn presented_status(&self) -> PickStatus {
        self.status
    }
}
impl Component for QuickPick {
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        if !self.same_controller(&next) {
            return None;
        }
        self.pending_config = Some(next.shared.config.borrow().clone());
        // The new description's callback replaces the old one. A callback
        // running now (re-entrantly) keeps the old one.
        if let (Ok(mut current), Ok(mut new)) = (
            self.shared.choose.try_borrow_mut(),
            next.shared.choose.try_borrow_mut(),
        ) {
            std::mem::swap(&mut *current, &mut *new);
        }
        self.described_modal = next.described_modal;
        // Described settings do not affect the active session. Its next
        // generation wakes this owner and adopts the latest snapshot.
        None
    }
    fn mounted(&mut self) {
        self.shared.modal.set(self.described_modal);
        self.claim();
    }
    fn unmounted(&mut self) {
        if !self.owns() {
            return;
        }
        // A picker removed while presented, or while its session waits for a
        // pending result with the surface hidden, must not reappear in
        // another picker for the same session.
        let pending = matches!(
            self.shared.controller.phase.get(),
            Ok(Phase::Pending { .. })
        );
        if (self.shown || pending)
            && let Ok(Some(session)) = self.shared.controller.session.get()
        {
            self.shared.controller.orphaned.set(session.generation);
        }
        *self.shared.controller.owner.borrow_mut() = Weak::new();
    }
    fn take_children(&mut self) -> Vec<Element> {
        let Some(anchor) = self.anchor.take() else {
            return Vec::new();
        };
        vec![
            Element::new("anchor", Container::new(anchor)).structural(),
            Element::new(
                "surface",
                PickPanel {
                    shared: self.shared.clone(),
                    frame: Cell::new(None),
                    children: vec![
                        Element::new(
                            "frame",
                            crate::popover::transient_panel(
                                Element::new(
                                    "content",
                                    PickContent {
                                        shared: self.shared.clone(),
                                        applied: None,
                                        list_serial: 0,
                                        message: false,
                                        texts: None,
                                    },
                                )
                                .structural(),
                            ),
                        )
                        .structural(),
                    ],
                },
            ),
        ]
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        // The panel's name may come from the string table or a binding, also
        // while closed.
        self.shared.read_texts(cx)?;
        if !self.claim() {
            // Another picker presents the controller. Stay subscribed to its
            // session so that this one claims it when the next session opens
            // after the other picker unmounted.
            cx.read(&self.shared.controller.phase)?;
            cx.read(&self.shared.controller.session)?;
            self.shown = false;
            self.status = PickStatus::Closed;
            return Ok(());
        }
        let (status, _) = self.shared.controller.read_presented(cx)?;
        let generation = cx
            .read_paint(&self.shared.controller.session)?
            .map(|session| session.generation);
        if status == PickStatus::Open && generation != self.session_generation {
            if let Some(config) = self.pending_config.take() {
                *self.shared.config.borrow_mut() = config;
            }
            self.shared.modal.set(self.described_modal);
            self.session_generation = generation;
            // A new opening may replace an already visible session, so no
            // visibility transition is guaranteed to remeasure the panel.
            cx.request_layout();
        }
        self.status = status;
        self.shown = status == PickStatus::Open;
        Ok(())
    }
    fn child_visible(&self, key: &str) -> bool {
        key != "surface" || self.shown
    }
    fn child_surface(&self, key: &str) -> Option<SurfaceKind> {
        (key == "surface").then(|| self.kind())
    }
    fn dismiss_surface(
        &mut self,
        _: &mut EventContext,
        _: SurfaceDismissal,
    ) -> Result<(), RuntimeError> {
        self.shared.controller.close()?;
        self.shown = false;
        Ok(())
    }
    fn measure(&self, cx: &MeasureContext<'_>, a: Constraints) -> Result<Size, RuntimeError> {
        crate::popover::measure_anchor(cx, a)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        if cx.len() > 2 {
            return Err(LayoutError::InvalidChild.into());
        }
        if !cx.is_empty() {
            cx.place(0, Rect::from_size(size))?;
        }
        if cx.len() == 2 {
            if self.shared.modal.get() {
                // The runtime places a modal surface over the whole window.
                cx.place(1, Rect::from_size(size))?;
            } else {
                let viewport = cx.measurement().viewport();
                let available = Constraints::new(Some(viewport.width()), Some(viewport.height()))?;
                let panel = cx.measure(1, available)?;
                let top = cx.measurement().metric(MetricRole::ControlHeight);
                cx.place(
                    1,
                    // Centered on the anchor; the runtime keeps it inside the window.
                    rect(
                        (size.width() - panel.width()) / 2.0,
                        top,
                        panel.width(),
                        panel.height(),
                    )?,
                )?;
            }
        }
        Ok(())
    }
    fn keyboard_event(
        &mut self,
        _: &mut EventContext,
        _: KeyEvent,
        _: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        Ok(InputOutcome::Ignored)
    }
    fn inspect(&self) -> ComponentDetails {
        let mut d = ComponentDetails::default();
        d.properties
            .insert("open".into(), InspectValue::Bool(self.shown));
        d.properties.insert(
            "status".into(),
            InspectValue::Text(format!("{:?}", self.status)),
        );
        d.properties
            .insert("modal".into(), InspectValue::Bool(self.shared.modal.get()));
        d.semantics = Some(
            Semantics::new(if self.shared.modal.get() {
                SemanticRole::Dialog
            } else {
                SemanticRole::Popover
            })
            .name(self.shared.text(|texts| &texts.name))
            .expanded(self.shown)
            .enabled_actions(self.shown.then_some(SemanticAction::Dismiss)),
        );
        d
    }
}

/// The surface root: for a modal picker the whole window with a dimmed
/// backdrop and the frame near its top, otherwise just the frame.
struct PickPanel {
    shared: Rc<Shared>,
    frame: Cell<Option<Rect>>,
    children: Vec<Element>,
}
impl PickPanel {
    fn frame_constraints(
        &self,
        cx: &MeasureContext<'_>,
        size: Size,
    ) -> Result<Constraints, RuntimeError> {
        let margin = cx.metric(MetricRole::GroupPadding);
        let width = self
            .shared
            .config
            .borrow()
            .width
            .min((size.width() - 2.0 * margin).max(0.0));
        let top = cx.metric(MetricRole::ControlHeight);
        let height = (size.height() - top - margin).max(0.0);
        Ok(Constraints::new(Some(width), Some(height))?)
    }
}
impl Component for PickPanel {
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, cx: &MeasureContext<'_>, a: Constraints) -> Result<Size, RuntimeError> {
        let viewport = cx.viewport();
        let window = Size::new(viewport.width(), viewport.height())?;
        let frame = cx.measure(0, self.frame_constraints(cx, window)?)?;
        if self.shared.modal.get() {
            Ok(window)
        } else {
            Ok(fit(a, frame)?)
        }
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        if self.shared.modal.get() {
            let constraints = self.frame_constraints(cx.measurement(), size)?;
            let frame = cx.measure(0, constraints)?;
            let width = frame
                .width()
                .min(constraints.width().unwrap_or(frame.width()));
            let height = frame
                .height()
                .min(constraints.height().unwrap_or(frame.height()));
            let top = cx.measurement().metric(MetricRole::ControlHeight);
            let placed = rect(((size.width() - width) / 2.0).max(0.0), top, width, height)?;
            self.frame.set(Some(placed));
            cx.place(0, placed)
        } else {
            self.frame.set(Some(Rect::from_size(size)));
            cx.place(0, Rect::from_size(size))
        }
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        if self.shared.modal.get()
            && let Some(size) = p.size()
        {
            p.fill_rect(
                Rect::from_size(size),
                Color::from_linear(0.0, 0.0, 0.0, 0.35)?,
            );
        }
        Ok(())
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        event: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        if !self.shared.modal.get() {
            return Ok(InputOutcome::Ignored);
        }
        // The backdrop belongs to the picker: a press on it closes the
        // picker, and nothing passes through to the content below.
        if let PointerKind::Press(_) = event.kind()
            && let Some(point) = cx.to_local(event.position())
            && self.frame.get().is_some_and(|frame| !frame.contains(point))
        {
            self.shared.controller.close()?;
        }
        Ok(InputOutcome::Handled)
    }
    fn keyboard_event(
        &mut self,
        _: &mut EventContext,
        _: KeyEvent,
        _: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        Ok(InputOutcome::Ignored)
    }
    fn focus_within_changed(
        &mut self,
        _: &mut EventContext,
        within: bool,
    ) -> Result<(), RuntimeError> {
        if !within {
            self.shared.controller.close()?;
        }
        Ok(())
    }
}

/// The search field, the outcome message and the results, described from
/// the session in `update`.
struct PickContent {
    shared: Rc<Shared>,
    /// The session generation, query and message last described.
    applied: Option<(Option<u64>, String, Option<String>)>,
    /// Increases when the results change, so the list starts at its top.
    list_serial: u64,
    message: bool,
    /// The texts this content last described its parts with. The picker's
    /// own update stores newly read texts in `Shared` first, so this content
    /// keeps its own record to notice a change.
    texts: Option<Texts>,
}
impl PickContent {
    fn describe(&self, message: Option<&str>) -> Result<Vec<Element>, RuntimeError> {
        let placeholder = self.shared.text(|texts| &texts.placeholder);
        let empty_text = self.shared.text(|texts| &texts.empty_text);
        let shared = self.shared.clone();
        let mut children = vec![
            Element::new(
                "search",
                SearchField::new(self.shared.controller.query.clone())
                    .placeholder(placeholder)
                    .on_key(move |cx, key| shared.on_key(cx, key)),
            )
            .structural(),
        ];
        if let Some(message) = message {
            children.push(
                Element::new("message", Label::new(message).color_role(ColorRole::Error))
                    .structural(),
            );
        }
        let view = self.shared.view.borrow();
        if view.ranked.is_empty() {
            children.push(
                Element::new("empty", Label::new(empty_text).color_role(ColorRole::Muted))
                    .structural(),
            );
        } else if let Some(session) = &view.session {
            let source = ListSource::new(PickItems {
                shared: self.shared.clone(),
                session: session.clone(),
                ranked: view.ranked.clone(),
            });
            // A fixed source for this list: the list is replaced with a new
            // key whenever the results change.
            let items = self.shared.controller.query.map(move |_| source.clone())?;
            children.push(
                Element::new(
                    format!("results-{}", self.list_serial),
                    VirtualList::estimated(items, ROW_ESTIMATE)?
                        .controller(self.shared.controller.list.clone()),
                )
                .structural(),
            );
        }
        Ok(children)
    }
    fn list_height(&self, cx: &MeasureContext<'_>) -> f32 {
        let rows = self
            .shared
            .view
            .borrow()
            .ranked
            .len()
            .min(QuickPick::VISIBLE_ROWS);
        // Rows are at least one control tall; see `PickRow::measure`.
        cx.metric(MetricRole::ControlHeight) * rows as f32
    }
}
/// `size` within `available`, which may leave either axis unbounded.
fn fit(available: Constraints, size: Size) -> Result<Size, RuntimeError> {
    Ok(Size::new(
        available
            .width()
            .map_or(size.width(), |w| size.width().min(w)),
        available
            .height()
            .map_or(size.height(), |h| size.height().min(h)),
    )?)
}
/// The list's row estimate before rows are measured, in logical pixels; rows
/// measure to the theme's control height.
const ROW_ESTIMATE: f32 = 32.0;
impl Component for PickContent {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let session = cx.read(&self.shared.controller.session)?;
        let query = cx.read(&self.shared.controller.query)?;
        let (_, message) = self.shared.controller.read_presented(cx)?;
        let generation = session.as_ref().map(|s| s.generation);
        let state = (generation, query.clone(), message.clone());
        self.shared.read_texts(cx)?;
        let texts = self.shared.texts.borrow().clone().unwrap_or_default();
        let translated = self.texts.as_ref() != Some(&texts);
        self.texts = Some(texts);
        if self.applied.as_ref() == Some(&state) && !translated {
            return Ok(());
        }
        let results_changed = self
            .applied
            .as_ref()
            .is_none_or(|(g, q, _)| *g != generation || *q != query);
        if results_changed {
            let matcher = self.shared.config.borrow().matcher.clone();
            let mut view = self.shared.view.borrow_mut();
            view.ranked = match &session {
                Some(session) => matcher.rank(&query, &session.items).into(),
                None => Rc::from([]),
            };
            view.query = query;
            view.session = session;
            self.list_serial = self.list_serial.wrapping_add(1);
        }
        self.message = message.is_some();
        cx.request_children(self.describe(message.as_deref())?);
        self.applied = Some(state);
        Ok(())
    }
    fn measure(&self, cx: &MeasureContext<'_>, a: Constraints) -> Result<Size, RuntimeError> {
        let pad = cx.metric(MetricRole::ItemGap);
        let width = a.width().unwrap_or(QuickPick::DEFAULT_WIDTH.get());
        let inner = (width - 2.0 * pad).max(0.0);
        let free = Constraints::new(Some(inner), None)?;
        let mut height = pad;
        let last = cx.len().saturating_sub(1);
        for index in 0..cx.len() {
            let child = if index == last && !self.shared.view.borrow().ranked.is_empty() {
                self.list_height(cx)
            } else {
                cx.measure(index, free)?.height()
            };
            height += child + pad;
        }
        fit(a, Size::new(width, height)?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        let pad = cx.measurement().metric(MetricRole::ItemGap);
        let inner = (size.width() - 2.0 * pad).max(0.0);
        let free = Constraints::new(Some(inner), None)?;
        let mut y = pad;
        let last = cx.len().saturating_sub(1);
        let has_list = !self.shared.view.borrow().ranked.is_empty();
        for index in 0..cx.len() {
            let height = if index == last && has_list {
                (size.height() - y - pad).max(0.0)
            } else {
                cx.measure(index, free)?.height()
            };
            cx.place(index, rect(pad, y, inner, height)?)?;
            y += height + pad;
        }
        Ok(())
    }
    fn inspect(&self) -> ComponentDetails {
        let mut d = ComponentDetails::default();
        let view = self.shared.view.borrow();
        let highlight = self.shared.controller.highlight.get().ok().flatten();
        let effective = view.effective(highlight.as_ref());
        d.properties
            .insert("query".into(), InspectValue::Text(view.query.clone()));
        d.properties.insert(
            "results".into(),
            InspectValue::Number(view.ranked.len() as f64),
        );
        d.properties
            .insert("message".into(), InspectValue::Bool(self.message));
        let items = view.session.iter().flat_map(|session| {
            view.ranked.iter().map(move |index| {
                let entry = &session.items[*index];
                let mut item = SemanticItem::new(entry.label.clone())
                    .selected(effective == Some(*index))
                    .disabled(entry.disabled);
                if let Some(shortcut) = &entry.shortcut {
                    item = item.shortcut(shortcut.clone());
                }
                item.action(SemanticAction::Activate)
            })
        });
        d.semantics = Some(
            Semantics::new(SemanticRole::ListBox)
                .name(self.shared.text(|texts| &texts.list_name))
                .items(items.collect::<Vec<_>>()),
        );
        d
    }
}

struct PickItems {
    shared: Rc<Shared>,
    session: Session,
    ranked: Rc<[usize]>,
}
impl ListItems for PickItems {
    fn len(&self) -> usize {
        self.ranked.len()
    }
    fn key(&self, index: usize) -> Option<String> {
        let item = *self.ranked.get(index)?;
        self.session.items.get(item).map(|item| item.key.clone())
    }
    fn build(&self, index: usize) -> Option<Element> {
        let position = *self.ranked.get(index)?;
        let item = self.session.items.get(position)?;
        // The end of the row: the detail, then the kind.
        let detail = [item.detail.as_deref(), item.kind.as_deref()]
            .into_iter()
            .flatten()
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>()
            .join(" · ");
        let description = [item.shortcut.as_deref(), item.kind.as_deref()]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" · ");
        Some(Element::new(
            item.key.clone(),
            PickRow {
                shared: self.shared.clone(),
                item: position,
                label: item.label.clone(),
                description: (!description.is_empty()).then_some(description),
                enabled: !item.disabled,
                highlighted: false,
                hovered: false,
                pressed: false,
                labels_dirty: false,
                detail,
            },
        ))
    }
    fn index_of(&self, key: &str) -> Option<usize> {
        let item = *self.session.keys.get(key)?;
        self.ranked.iter().position(|e| *e == item)
    }
}

/// One result: the item's label at the start and its detail and kind at the
/// end; a pointer press and release on it chooses the item.
struct PickRow {
    shared: Rc<Shared>,
    item: usize,
    label: String,
    description: Option<String>,
    enabled: bool,
    highlighted: bool,
    hovered: bool,
    pressed: bool,
    detail: String,
    labels_dirty: bool,
}
impl PickRow {
    fn labels(&self) -> Vec<Element> {
        let name_role = if !self.enabled {
            ColorRole::Muted
        } else if self.highlighted {
            ColorRole::OnTab
        } else {
            ColorRole::OnSurface
        };
        let detail_role = if self.enabled && self.highlighted {
            ColorRole::OnTab
        } else {
            ColorRole::Muted
        };
        vec![
            Element::new(
                "name",
                Label::new(self.label.clone())
                    .overflow(TextOverflow::Ellipsis)
                    .color_role(name_role),
            )
            .structural(),
            Element::new(
                "detail",
                Label::new(self.detail.clone())
                    .overflow(TextOverflow::Ellipsis)
                    .style(TextStyle::Caption)
                    .color_role(detail_role),
            )
            .structural(),
        ]
    }
}
impl Component for PickRow {
    fn retain_initial_children(&self) -> bool {
        true
    }
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        let labels =
            self.label != next.label || self.detail != next.detail || self.enabled != next.enabled;
        let paint = labels || self.item != next.item || self.description != next.description;
        self.shared = next.shared;
        self.item = next.item;
        self.label = next.label;
        self.detail = next.detail;
        self.description = next.description;
        self.enabled = next.enabled;
        self.labels_dirty |= labels;
        paint.then_some(Invalidation::Paint)
    }
    fn take_children(&mut self) -> Vec<Element> {
        self.labels()
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let highlight = cx.read_paint(&self.shared.controller.highlight)?;
        // The read invalidates this row whenever the highlight moves.
        let highlighted =
            self.shared.view.borrow().effective(highlight.as_ref()) == Some(self.item);
        if highlighted != self.highlighted || self.labels_dirty {
            self.highlighted = highlighted;
            self.labels_dirty = false;
            cx.request_children(self.labels());
        }
        Ok(())
    }
    fn measure(&self, cx: &MeasureContext<'_>, a: Constraints) -> Result<Size, RuntimeError> {
        let free = Constraints::new(None, None)?;
        let name = cx.measure(0, free)?;
        let detail = cx.measure(1, free)?;
        let height = cx
            .metric(MetricRole::ControlHeight)
            .max(name.height())
            .max(detail.height());
        let width = a.width().unwrap_or(name.width() + detail.width());
        fit(a, Size::new(width, height)?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        let pad = cx.measurement().metric(MetricRole::ControlPaddingX);
        let gap = cx.measurement().metric(MetricRole::ItemGap);
        let free = Constraints::new(None, None)?;
        let available = (size.width() - 2.0 * pad).max(0.0);
        let natural_name = cx.measure(0, free)?.width();
        let natural_detail = cx.measure(1, free)?.width();
        // The label comes first: it takes its width, keeping up to a third
        // of the row for the detail, which gets what the label leaves. Each
        // ends in an ellipsis when it does not fit, so a long detail such as
        // a location never hides the label.
        let spacing = if natural_detail > 0.0 { gap } else { 0.0 };
        let reserved = natural_detail.min(available / 3.0);
        let name_width = natural_name.min((available - reserved - spacing).max(0.0));
        let detail_width = natural_detail.min((available - name_width - spacing).max(0.0));
        let name = cx.measure(0, Constraints::new(Some(name_width), None)?)?;
        let detail = cx.measure(1, Constraints::new(Some(detail_width), None)?)?;
        let center = |h: f32| ((size.height() - h) / 2.0).max(0.0);
        cx.place(
            0,
            rect(
                pad,
                center(name.height()),
                name_width,
                name.height().min(size.height()),
            )?,
        )?;
        cx.place(
            1,
            rect(
                (size.width() - pad - detail_width).max(0.0),
                center(detail.height()),
                detail_width,
                detail.height().min(size.height()),
            )?,
        )
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        if let Some(size) = p.size() {
            if self.highlighted {
                p.fill_rect(Rect::from_size(size), p.color(ColorRole::TabSelected));
            } else if self.hovered {
                p.fill_rect(Rect::from_size(size), p.color(ColorRole::SurfaceHover));
            }
        }
        Ok(())
    }
    fn hover_changed(&mut self, cx: &mut EventContext, hovered: bool) {
        if hovered != self.hovered {
            self.hovered = hovered;
            cx.invalidate(Invalidation::Paint);
        }
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        event: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        let inside = match (cx.size(), cx.to_local(event.position())) {
            (Some(size), Some(point)) => Rect::from_size(size).contains(point),
            _ => false,
        };
        match event.kind() {
            PointerKind::Press(PointerButton::Primary) => {
                cx.capture_pointer()?;
                self.pressed = true;
                Ok(InputOutcome::Handled)
            }
            PointerKind::Release(PointerButton::Primary) if self.pressed => {
                self.pressed = false;
                cx.release_pointer();
                if inside {
                    self.shared.choose(cx, self.item)?;
                }
                Ok(InputOutcome::Handled)
            }
            _ => Ok(InputOutcome::Ignored),
        }
    }
    fn input_reset(&mut self) {
        self.pressed = false;
    }
    fn inspect(&self) -> ComponentDetails {
        let mut d = ComponentDetails::default();
        d.properties
            .insert("highlighted".into(), InspectValue::Bool(self.highlighted));
        let mut semantics = Semantics::new(SemanticRole::Button)
            .name(self.label.clone())
            .disabled(!self.enabled)
            .enabled_actions([SemanticAction::Activate]);
        if let Some(description) = &self.description {
            semantics = semantics.description(description.clone());
        }
        d.semantics = Some(semantics);
        d
    }
}

#[cfg(test)]
mod tests {
    use super::contiguous_run;

    fn naive(needle: &[char], hay: &[(char, bool)]) -> Option<usize> {
        hay.windows(needle.len())
            .position(|window| window.iter().map(|(c, _)| *c).eq(needle.iter().copied()))
    }

    /// Every word of `length` over a repetitive alphabet.
    fn words(length: usize) -> Vec<Vec<char>> {
        const ALPHABET: [char; 3] = ['a', 'b', 'c'];
        (0..ALPHABET.len().pow(length as u32))
            .map(|mut n| {
                (0..length)
                    .map(|_| {
                        let c = ALPHABET[n % ALPHABET.len()];
                        n /= ALPHABET.len();
                        c
                    })
                    .collect()
            })
            .collect()
    }

    #[test]
    fn the_linear_run_search_agrees_with_a_window_scan() {
        // Small needles and texts where the failure table's fallbacks matter.
        for needle_length in 1..=4 {
            for needle in words(needle_length) {
                for hay_length in 0..=6 {
                    for text in words(hay_length) {
                        let hay: Vec<(char, bool)> = text.iter().map(|c| (*c, false)).collect();
                        assert_eq!(
                            contiguous_run(&needle, &hay),
                            naive(&needle, &hay),
                            "{needle:?} in {text:?}"
                        );
                    }
                }
            }
        }
        // The review's adversarial case: a run of `a`s followed by `b`.
        let needle: Vec<char> = "a".repeat(2000).chars().chain(['b']).collect();
        let hay: Vec<(char, bool)> = "a"
            .repeat(4000)
            .chars()
            .chain("b".repeat(10).chars())
            .map(|c| (c, false))
            .collect();
        assert_eq!(contiguous_run(&needle, &hay), Some(2000));
    }
}
