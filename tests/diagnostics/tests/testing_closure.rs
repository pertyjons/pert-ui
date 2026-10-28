//! The `testing` module's closure, checked from the committed API listings.
//!
//! `pert_ui::testing` and `pert_ui_runtime::testing` promise that a headless
//! test needs only them and the prelude: each type that a member's
//! constructors, methods and associated constants take or return, and each
//! type a member's public fields and variants hold, is in `testing` or in the
//! prelude, and the same holds for every name this adds. This test reads the
//! listings under `api/` (cargo-public-api's `--simplified` output, the
//! version `api/generate.sh` pins), starts from the seeds, follows those
//! signatures and payloads across the runtime, core, CPU renderer and facade
//! packages, and fails for every reached pert-ui type that is neither a
//! member of `testing`, nor in the prelude, nor in [`EXCLUSIONS`].
//!
//! - **Seeds:** every type `testing` exports except `HostIntegration`, whose
//!   vocabulary is the `host` module's, and the [`RUNTIME_SEEDS`] methods of
//!   `Runtime` the module's names exist for.
//! - **Followed:** inherent methods and associated constants (a trait's own
//!   methods and associated types for a trait), public fields, tuple-variant
//!   payloads and struct-variant fields. Not followed: trait
//!   implementations, an error nested in an error's field or payload, and the
//!   prelude members, excluded names and non-pert-ui types, which stop the
//!   traversal.
//! - **Configurations:** the runtime and the facade's default, `cpu`,
//!   no-default and `desktop-cpu` listings, where `testing` or the prelude
//!   differ.
//!
//! It also checks that every exclusion is reached and is no member, that the
//! facade's `testing` holds every name of the runtime's, and that the seed
//! methods exist. Any listing line it does not recognize, any type name two
//! declarations disagree on and any unresolved `pert_ui…` type path fail the
//! test, so a format change or a parse gap cannot pass silently.
//!
//! Setting `PERT_UI_API_DIR` to another directory of listings runs the check
//! against them, as the change that introduced it did against the listings
//! from before its additions.
//!
//! Limits: it checks types, not functions or constants; it sees the x86_64
//! Linux view the listings record; the API listing gate, not this test, keeps
//! the listings current; and a new `Runtime` method written for tests joins
//! [`RUNTIME_SEEDS`] in the change that adds it.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::OnceLock;

/// The `Runtime` methods `testing`'s names exist for: input by hand, reading
/// what is shown, development validation and captures. `Runtime` is a
/// prelude member, so its other methods belong to their families.
const RUNTIME_SEEDS: [&str; 9] = [
    "set_viewport",
    "key_input",
    "focus_next",
    "pointer_input",
    "command_key_input",
    "inspection_id",
    "query",
    "validation_report",
    "request_capture",
];

/// Names a seed reaches that a test does not write, each with the module
/// that holds it. An excluded name stops the traversal.
const EXCLUSIONS: &[(&str, &str)] = &[
    // The window, event, parameter and live-source answers of a query:
    // records for an application's own tooling.
    ("WindowInspection", "inspection"),
    ("EventHistory", "inspection"),
    ("ParameterInspection", "inspection"),
    ("SourceInspection", "inspection"),
    // What a component reports about its geometry, invalidation and
    // validation hints: a test reads them by field.
    ("BoxModelInspection", "component and inspection"),
    ("Invalidation", "component and inspection"),
    ("ValidationHints", "component and inspection"),
    // A host's surface size; a test builds its viewport with `Viewport::new`.
    ("PhysicalExtent", "host"),
    // Key routing a host acts on and the record a component reads.
    ("KeyDispatch", "host"),
    ("KeyEvent", "component and inspection"),
    ("KeyTransition", "component"),
    // The scene `Runtime::paint` returns, passed on to the CPU renderer.
    ("DisplayList", "component and host"),
    // A constructor's validation error, propagated with `?`.
    ("GeometryError", "crate root"),
];

/// A type, identified by its package and its name without generics.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct TypeKey {
    package: String,
    name: String,
}

/// A followed reference from a member of one type to another type.
#[derive(Debug)]
struct Edge {
    /// The member, such as `Driver::semantics` or `TestError::NotRevealed`.
    via: String,
    target: TypeKey,
    /// A field or variant payload, to which the nested-error rule applies.
    payload: bool,
}

#[derive(Debug, Default)]
struct TypeInfo {
    is_error: bool,
    edges: Vec<Edge>,
    inherent_methods: BTreeSet<String>,
    /// Each declaration of the name: its line and its members with the
    /// module prefixes of every path removed, for the ambiguity guard.
    groups: Vec<(String, Vec<String>)>,
}

#[derive(Debug, Default)]
struct ModuleEntry {
    names: BTreeSet<String>,
    globs: Vec<String>,
}

/// What the parser knows about one configuration's listings.
#[derive(Debug, Default)]
struct Index {
    /// Declared type names (structs, enums, traits, unions, aliases) by
    /// package.
    declared: BTreeMap<String, BTreeSet<String>>,
    types: BTreeMap<TypeKey, TypeInfo>,
    modules: BTreeMap<String, ModuleEntry>,
    /// The names of free functions and constants: the listing's `pub fn`
    /// and `pub const` lines outside any type, which are no seeds.
    values: BTreeSet<String>,
    /// Parse, resolution and ambiguity failures.
    problems: Vec<String>,
}

/// The block a member line belongs to.
#[derive(Clone, Debug)]
enum Block {
    None,
    /// After a struct, enum or union declaration: its fields and variants.
    Declaration(TypeKey),
    /// After a trait declaration, until the next `impl` header.
    TraitDeclaration(TypeKey),
    Inherent(Option<TypeKey>),
    TraitImpl,
}

fn is_pert(segment: &str) -> bool {
    segment == "pert_ui" || segment.starts_with("pert_ui_")
}

fn is_ident_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn upper(segment: &str) -> bool {
    segment.starts_with(|c: char| c.is_ascii_uppercase())
}

/// Splits a leading path into its segments, generics removed, and the rest
/// of the line from the first `(`, space or single `:` outside generics.
fn split_path(text: &str) -> (Vec<String>, &str) {
    let (segments, _, rest) = split_path_generics(text);
    (segments, rest)
}

/// [`split_path`], also returning the generics of the last segment, such as a
/// method's own `<F: FnOnce(B)>`, whose bounds are part of its signature.
fn split_path_generics(text: &str) -> (Vec<String>, &str, &str) {
    let bytes = text.as_bytes();
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut generics = (0, 0);
    let mut depth = 0usize;
    let mut i = 0;
    while i < bytes.len() {
        let byte = bytes[i];
        if byte == b'<' {
            if depth == 0 {
                generics = (i, i);
            }
            depth += 1;
        } else if byte == b'>' && (i == 0 || bytes[i - 1] != b'-') {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                generics.1 = i + 1;
            }
        } else if depth == 0 {
            if byte == b':' && bytes.get(i + 1) == Some(&b':') {
                segments.push(std::mem::take(&mut current));
                generics = (0, 0);
                i += 2;
                continue;
            }
            if byte == b':' || byte == b'(' || byte == b' ' {
                break;
            }
            current.push(char::from(byte));
        }
        i += 1;
    }
    segments.push(current);
    (segments, &text[generics.0..generics.1], &text[i..])
}

/// Every `pert_ui…` path in a text, as its byte range and segments.
fn pert_paths(text: &str) -> Vec<(usize, usize, Vec<String>)> {
    let bytes = text.as_bytes();
    let mut paths = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let starts = is_ident_byte(bytes[i])
            && (i == 0
                || !(is_ident_byte(bytes[i - 1]) || bytes[i - 1] == b':' || bytes[i - 1] == b'\''));
        if !starts {
            i += 1;
            continue;
        }
        let start = i;
        let mut segments = Vec::new();
        loop {
            let begin = i;
            while i < bytes.len() && is_ident_byte(bytes[i]) {
                i += 1;
            }
            segments.push(text[begin..i].to_owned());
            if bytes.get(i) == Some(&b':')
                && bytes.get(i + 1) == Some(&b':')
                && bytes.get(i + 2).is_some_and(|b| is_ident_byte(*b))
            {
                i += 2;
            } else {
                break;
            }
        }
        if is_pert(&segments[0]) {
            paths.push((start, i, segments));
        }
    }
    paths
}

/// Finds `needle` outside angle brackets and parentheses.
fn find_top_level(text: &str, needle: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut depth = 0usize;
    for i in 0..bytes.len() {
        match bytes[i] {
            b'<' | b'(' => depth += 1,
            b'>' if i > 0 && bytes[i - 1] == b'-' => {}
            b'>' | b')' => depth = depth.saturating_sub(1),
            _ => {}
        }
        if depth == 0 && text[i..].starts_with(needle) {
            return Some(i);
        }
    }
    None
}

/// Strips a leading balanced `<…>`.
fn skip_generics(text: &str) -> &str {
    if !text.starts_with('<') {
        return text;
    }
    let bytes = text.as_bytes();
    let mut depth = 0usize;
    for i in 0..bytes.len() {
        match bytes[i] {
            b'<' => depth += 1,
            b'>' if i > 0 && bytes[i - 1] == b'-' => {}
            b'>' => {
                depth -= 1;
                if depth == 0 {
                    return &text[i + 1..];
                }
            }
            _ => {}
        }
    }
    text
}

/// Removes leading `#[…] ` attributes.
fn strip_attributes(mut line: &str) -> &str {
    while line.starts_with("#[") {
        match line.find("] ") {
            Some(end) => line = &line[end + 2..],
            None => break,
        }
    }
    line
}

const DECLARATIONS: [&str; 4] = ["pub struct ", "pub enum ", "pub union ", "pub trait "];
const ITEMS: [&str; 4] = ["pub const fn ", "pub fn ", "pub const ", "pub type "];

impl Index {
    fn build(listings: &[&str]) -> Self {
        let mut index = Self::default();
        for listing in listings {
            index.declare(listing);
        }
        for listing in listings {
            index.declare_aliases(listing);
        }
        for listing in listings {
            index.parse(listing);
        }
        index.guard();
        index
    }

    fn package(listing: &str) -> String {
        listing
            .lines()
            .next()
            .and_then(|line| line.strip_prefix("pub mod "))
            .unwrap_or_default()
            .to_owned()
    }

    /// The first segment after the package that names a declared type of
    /// that package, searched in `segments[1..end]`. Types do not nest, so
    /// the segments after it are members: a variant named like a type does
    /// not become the owner.
    fn type_segment(&self, segments: &[String], end: usize) -> Option<(usize, TypeKey)> {
        let package = segments.first()?;
        let names = self.declared.get(package)?;
        (1..end).find(|&i| names.contains(&segments[i])).map(|i| {
            (
                i,
                TypeKey {
                    package: package.clone(),
                    name: segments[i].clone(),
                },
            )
        })
    }

    fn declare(&mut self, listing: &str) {
        for line in listing.lines() {
            let line = strip_attributes(line);
            if let Some(rest) = DECLARATIONS.iter().find_map(|k| line.strip_prefix(k)) {
                let (segments, _) = split_path(rest);
                if let (Some(package), Some(name)) = (segments.first(), segments.last()) {
                    self.declared
                        .entry(package.clone())
                        .or_default()
                        .insert(name.clone());
                }
            }
        }
    }

    /// A `pub type` outside any block whose last-but-one segment is a module
    /// is a standalone alias, a declaration of its own.
    fn declare_aliases(&mut self, listing: &str) {
        for line in listing.lines() {
            if let Some(rest) = line.strip_prefix("pub type ") {
                let (segments, _) = split_path(rest);
                if is_pert(&segments[0])
                    && self.type_segment(&segments, segments.len() - 1).is_none()
                {
                    let name = segments[segments.len() - 1].clone();
                    self.declared
                        .entry(segments[0].clone())
                        .or_default()
                        .insert(name);
                }
            }
        }
    }

    /// The line with every `pert_ui…` path shortened to start at its type.
    fn normalize(&self, line: &str) -> String {
        let mut out = String::new();
        let mut last = 0;
        for (start, end, segments) in pert_paths(line) {
            out.push_str(&line[last..start]);
            match self.type_segment(&segments, segments.len()) {
                Some((i, _)) => out.push_str(&segments[i..].join("::")),
                None => out.push_str(&line[start..end]),
            }
            last = end;
        }
        out.push_str(&line[last..]);
        out
    }

    /// The followed edges a member's text names.
    fn edges(&mut self, via: &str, text: &str, payload: bool, origin: &str) -> Vec<Edge> {
        let mut edges = Vec::new();
        for (_, _, segments) in pert_paths(text) {
            match self.type_segment(&segments, segments.len()) {
                Some((_, target)) => edges.push(Edge {
                    via: via.to_owned(),
                    target,
                    payload,
                }),
                None => {
                    if segments.last().is_some_and(|s| upper(s)) {
                        self.problems.push(format!(
                            "unresolved type path `{}` in `{origin}`",
                            segments.join("::")
                        ));
                    }
                }
            }
        }
        edges
    }

    fn add_edges(&mut self, owner: &TypeKey, edges: Vec<Edge>) {
        let info = self.types.entry(owner.clone()).or_default();
        // Repeated declarations list the same members; follow the first.
        if info.groups.len() <= 1 {
            info.edges.extend(edges);
        }
    }

    fn module_of(segments: &[String]) -> String {
        segments[..segments.len() - 1].join("::")
    }

    fn parse(&mut self, listing: &str) {
        let package = Self::package(listing);
        if !is_pert(&package) {
            self.problems
                .push("a listing does not start with `pub mod pert_ui…`".to_owned());
            return;
        }
        let mut block = Block::None;
        let mut group: Option<TypeKey> = None;
        for raw in listing.lines() {
            let line = strip_attributes(raw);
            if let Some(path) = line.strip_prefix("pub mod ") {
                self.modules.entry(path.to_owned()).or_default();
                block = Block::None;
                group = None;
            } else if let Some(path) = line.strip_prefix("pub use ") {
                if let Some((module, glob)) = path.split_once("::<<") {
                    let target = glob.strip_suffix("::*>>").unwrap_or(glob);
                    self.modules
                        .entry(module.to_owned())
                        .or_default()
                        .globs
                        .push(target.to_owned());
                } else {
                    let (segments, rest) = split_path(path);
                    if !rest.is_empty() || segments.len() < 2 {
                        self.problems.push(format!("unknown line `{raw}`"));
                    } else {
                        let module = Self::module_of(&segments);
                        let name = segments[segments.len() - 1].clone();
                        self.modules.entry(module).or_default().names.insert(name);
                    }
                }
                block = Block::None;
                group = None;
            } else if let Some((keyword, rest)) = DECLARATIONS
                .iter()
                .find_map(|k| line.strip_prefix(k).map(|rest| (*k, rest)))
            {
                let (segments, _) = split_path(rest);
                let key = TypeKey {
                    package: segments[0].clone(),
                    name: segments[segments.len() - 1].clone(),
                };
                let module = Self::module_of(&segments);
                self.modules
                    .entry(module)
                    .or_default()
                    .names
                    .insert(key.name.clone());
                self.types
                    .entry(key.clone())
                    .or_default()
                    .groups
                    .push((raw.to_owned(), Vec::new()));
                block = if keyword == "pub trait " {
                    Block::TraitDeclaration(key.clone())
                } else {
                    Block::Declaration(key.clone())
                };
                group = Some(key);
            } else if let Some(header) = line.strip_prefix("impl") {
                block = self.impl_block(header);
                self.record(&group, line);
            } else if let Some((keyword, rest)) = ITEMS
                .iter()
                .find_map(|k| line.strip_prefix(k).map(|rest| (*k, rest)))
            {
                self.item(raw, line, keyword, rest, &block, &mut group);
            } else if let Some(rest) = line.strip_prefix("pub ") {
                self.member(raw, line, rest, &block, &group);
            } else {
                self.problems.push(format!("unknown line `{raw}`"));
            }
        }
    }

    fn record(&mut self, group: &Option<TypeKey>, line: &str) {
        if let Some(key) = group {
            let normalized = self.normalize(line);
            if let Some(info) = self.types.get_mut(key)
                && let Some((_, members)) = info.groups.last_mut()
            {
                members.push(normalized);
            }
        }
    }

    fn impl_block(&mut self, header: &str) -> Block {
        let header = skip_generics(header).trim_start();
        let (trait_text, target) = match find_top_level(header, " for ") {
            Some(at) => (Some(&header[..at]), &header[at + 5..]),
            None => (None, header),
        };
        let target = match find_top_level(target, " where ") {
            Some(at) => &target[..at],
            None => target,
        };
        let (segments, _) = split_path(target);
        let key = self
            .type_segment(&segments, segments.len())
            .map(|(_, key)| key);
        match trait_text {
            Some(trait_text) => {
                if trait_text == "core::error::Error"
                    && let Some(key) = key
                {
                    self.types.entry(key).or_default().is_error = true;
                }
                Block::TraitImpl
            }
            None => Block::Inherent(key),
        }
    }

    /// A `pub fn`, `pub const` or `pub type` line.
    fn item(
        &mut self,
        raw: &str,
        line: &str,
        keyword: &str,
        rest: &str,
        block: &Block,
        group: &mut Option<TypeKey>,
    ) {
        let (segments, own_generics, signature) = split_path_generics(rest);
        let last = segments.len() - 1;
        if !is_pert(&segments[0]) {
            // A trait implementation for a primitive, such as `bool::Error`.
            if matches!(block, Block::TraitImpl) {
                self.record(group, line);
            } else {
                self.problems.push(format!("unknown line `{raw}`"));
            }
            return;
        }
        let owner = self.type_segment(&segments, last);
        let Some((at, owner)) = owner else {
            if keyword == "pub type " {
                // A standalone alias: a declaration whose one edge is its target.
                let key = TypeKey {
                    package: segments[0].clone(),
                    name: segments[last].clone(),
                };
                let module = Self::module_of(&segments);
                self.modules
                    .entry(module)
                    .or_default()
                    .names
                    .insert(key.name.clone());
                let target = signature.strip_prefix(" = ").unwrap_or(signature);
                // The target is the alias's one member, so the ambiguity guard
                // compares the targets of same-name aliases.
                let member = format!("{own_generics} = {}", self.normalize(target));
                self.types
                    .entry(key.clone())
                    .or_default()
                    .groups
                    .push((raw.to_owned(), vec![member]));
                let edges = self.edges(&key.name, target, false, raw);
                self.add_edges(&key, edges);
                *group = Some(key);
            } else {
                // Free functions and constants are not types; the check
                // skips them as seeds, by this record of their kind.
                self.values.insert(segments[last].clone());
            }
            return;
        };
        let via = segments[at..].join("::");
        match block {
            Block::TraitImpl => self.record(group, line),
            Block::Inherent(Some(key)) if *key == owner && keyword != "pub type " => {
                self.record(group, line);
                let edges = self.edges(&via, &format!("{own_generics}{signature}"), false, raw);
                if keyword.ends_with("fn ") {
                    self.types
                        .entry(owner.clone())
                        .or_default()
                        .inherent_methods
                        .insert(segments[last].clone());
                }
                self.add_edges(&owner, edges);
            }
            Block::TraitDeclaration(key) if *key == owner => {
                self.record(group, line);
                let edges = self.edges(&via, &format!("{own_generics}{signature}"), false, raw);
                self.add_edges(&owner, edges);
            }
            _ => self.problems.push(format!("unknown line `{raw}`")),
        }
    }

    /// A field, a variant or a struct-variant field.
    fn member(
        &mut self,
        raw: &str,
        line: &str,
        rest: &str,
        block: &Block,
        group: &Option<TypeKey>,
    ) {
        let (segments, payload) = split_path(rest);
        let owner = self.type_segment(&segments, segments.len() - 1);
        let (Some((at, owner)), Block::Declaration(key)) = (owner, block) else {
            self.problems.push(format!("unknown line `{raw}`"));
            return;
        };
        if owner != *key {
            self.problems.push(format!("unknown line `{raw}`"));
            return;
        }
        let via = segments[at..].join("::");
        let text = if let Some(field) = payload.strip_prefix(": ") {
            field
        } else if payload.starts_with('(') {
            payload
        } else if payload.is_empty() {
            ""
        } else {
            self.problems.push(format!("unknown line `{raw}`"));
            return;
        };
        self.record(group, line);
        let edges = self.edges(&via, text, true, raw);
        self.add_edges(&owner, edges);
    }

    /// Every declaration of a name in a package lists the same members, and
    /// no name is declared by two packages.
    fn guard(&mut self) {
        let mut packages: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for (key, info) in &self.types {
            if info.groups.is_empty() {
                continue;
            }
            packages.entry(&key.name).or_default().push(&key.package);
            let mut sorted: Vec<(&String, Vec<&String>)> = info
                .groups
                .iter()
                .map(|(line, members)| {
                    let mut members: Vec<&String> = members.iter().collect();
                    members.sort();
                    (line, members)
                })
                .collect();
            let (first_line, first) = sorted.remove(0);
            for (line, members) in sorted {
                if members != first {
                    self.problems.push(format!(
                        "ambiguous name `{}`: `{first_line}` and `{line}` list different members",
                        key.name
                    ));
                }
            }
        }
        for (name, owners) in packages {
            if owners.len() > 1 {
                self.problems.push(format!(
                    "ambiguous name `{name}`: declared by {}",
                    owners.join(" and ")
                ));
            }
        }
    }

    /// The names declared or re-exported under a module path, globs
    /// expanded.
    fn members(&self, module: &str) -> BTreeSet<String> {
        let mut names = BTreeSet::new();
        let mut pending = vec![module.to_owned()];
        let mut seen = BTreeSet::new();
        while let Some(path) = pending.pop() {
            if !seen.insert(path.clone()) {
                continue;
            }
            if let Some(entry) = self.modules.get(&path) {
                names.extend(entry.names.iter().cloned());
                pending.extend(entry.globs.iter().cloned());
            }
        }
        names
    }

    /// The type a name resolves to; names are unique across packages.
    fn lookup(&self, name: &str) -> Option<TypeKey> {
        self.types
            .keys()
            .find(|key| key.name == name && !self.types[*key].groups.is_empty())
            .cloned()
    }
}

/// What one configuration's check found.
#[derive(Debug, Default)]
struct Outcome {
    /// Reached names outside `testing`, the prelude and the exclusions, with
    /// the path that reached each.
    missing: BTreeMap<String, String>,
    /// Excluded names the traversal reached.
    reached_exclusions: BTreeSet<String>,
    /// The members of `testing`.
    testing: BTreeSet<String>,
    /// Parse, resolution, ambiguity and seed failures.
    problems: Vec<String>,
}

fn check(
    listings: &[&str],
    testing: &str,
    prelude: &str,
    runtime_seeds: &[&str],
    exclusions: &[&str],
) -> Outcome {
    let index = Index::build(listings);
    let members = index.members(testing);
    let prelude = index.members(prelude);
    let mut outcome = Outcome {
        testing: members.clone(),
        problems: index.problems.clone(),
        ..Outcome::default()
    };

    let mut visited: BTreeSet<TypeKey> = BTreeSet::new();
    let mut queue: VecDeque<(TypeKey, String)> = VecDeque::new();
    for name in &members {
        if name == "HostIntegration" {
            continue;
        }
        match index.lookup(name) {
            Some(key) => {
                if visited.insert(key.clone()) {
                    queue.push_back((key, String::new()));
                }
            }
            // A function or constant, by its listing line, is no seed.
            None if index.values.contains(name) => {}
            // Anything else that no declaration defines, such as a re-export
            // renamed with `as`, would leave its closure unchecked, so it
            // fails.
            None => outcome.problems.push(format!(
                "`testing` member `{name}` resolves to no declared type"
            )),
        }
    }
    let mut frontier: Vec<(&Edge, String, bool)> = Vec::new();
    if !runtime_seeds.is_empty() {
        match index
            .lookup("Runtime")
            .and_then(|key| index.types.get(&key))
        {
            Some(runtime) => {
                for seed in runtime_seeds {
                    if !runtime.inherent_methods.contains(*seed) {
                        outcome
                            .problems
                            .push(format!("seed `Runtime::{seed}` is not an inherent method"));
                    }
                }
                for edge in &runtime.edges {
                    let method = edge.via.rsplit("::").next().unwrap_or_default();
                    if runtime_seeds.contains(&method) {
                        frontier.push((edge, String::new(), false));
                    }
                }
            }
            None => outcome
                .problems
                .push("seed type `Runtime` is not declared".to_owned()),
        }
    }

    let mut reach = |edge: &Edge,
                     chain: &str,
                     queue: &mut VecDeque<(TypeKey, String)>,
                     outcome: &mut Outcome| {
        let name = &edge.target.name;
        let chain = if chain.is_empty() {
            edge.via.clone()
        } else {
            format!("{chain} → {}", edge.via)
        };
        if members.contains(name) || prelude.contains(name) {
            return;
        }
        if exclusions.contains(&name.as_str()) {
            outcome.reached_exclusions.insert(name.clone());
            return;
        }
        if visited.insert(edge.target.clone()) {
            outcome
                .missing
                .insert(name.clone(), format!("{chain} → {name}"));
            queue.push_back((edge.target.clone(), chain));
        }
    };
    for (edge, chain, _) in frontier {
        reach(edge, &chain, &mut queue, &mut outcome);
    }
    while let Some((key, chain)) = queue.pop_front() {
        let Some(info) = index.types.get(&key) else {
            continue;
        };
        for edge in &info.edges {
            let nested_error = edge.payload
                && info.is_error
                && index.types.get(&edge.target).is_some_and(|t| t.is_error);
            if !nested_error {
                reach(edge, &chain, &mut queue, &mut outcome);
            }
        }
    }
    outcome
}

/// One checked configuration: its listings and module paths.
struct Configuration {
    name: &'static str,
    listings: &'static [&'static str],
    testing: &'static str,
    prelude: &'static str,
}

const PACKAGE_LISTINGS: [&str; 3] = [
    "pert-ui-runtime/default.txt",
    "pert-ui-core/default.txt",
    "pert-ui-cpu/default.txt",
];

const CONFIGURATIONS: [Configuration; 5] = [
    Configuration {
        name: "runtime",
        listings: &["pert-ui-runtime/default.txt", "pert-ui-core/default.txt"],
        testing: "pert_ui_runtime::testing",
        prelude: "pert_ui_runtime::prelude",
    },
    Configuration {
        name: "default",
        listings: &["pert-ui/default.txt"],
        testing: "pert_ui::testing",
        prelude: "pert_ui::prelude",
    },
    Configuration {
        name: "cpu",
        listings: &["pert-ui/cpu.txt"],
        testing: "pert_ui::testing",
        prelude: "pert_ui::prelude",
    },
    Configuration {
        name: "no-default",
        listings: &["pert-ui/no-default.txt"],
        testing: "pert_ui::testing",
        prelude: "pert_ui::prelude",
    },
    Configuration {
        name: "desktop-cpu",
        listings: &["pert-ui/no-default-desktop-cpu.txt"],
        testing: "pert_ui::testing",
        prelude: "pert_ui::prelude",
    },
];

/// Reads a listing from `api/`, or from the directory `PERT_UI_API_DIR`
/// names, which runs the check against other listings, such as an earlier
/// commit's, for a mutation check.
fn read(listing: &str) -> String {
    let directory = std::env::var("PERT_UI_API_DIR")
        .unwrap_or_else(|_| format!("{}/../../api", env!("CARGO_MANIFEST_DIR")));
    let path = format!("{directory}/{listing}");
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("reading {path}: {error}"))
}

/// Each configuration's outcome, with the runtime `testing` members its
/// index holds, computed once for the tests below.
fn outcomes() -> &'static Vec<(&'static str, Outcome, BTreeSet<String>)> {
    static OUTCOMES: OnceLock<Vec<(&'static str, Outcome, BTreeSet<String>)>> = OnceLock::new();
    OUTCOMES.get_or_init(|| {
        let exclusions: Vec<&str> = EXCLUSIONS.iter().map(|(name, _)| *name).collect();
        CONFIGURATIONS
            .iter()
            .map(|configuration| {
                let mut files: Vec<String> =
                    configuration.listings.iter().map(|l| read(l)).collect();
                if configuration.name != "runtime" {
                    files.extend(PACKAGE_LISTINGS.iter().map(|l| read(l)));
                }
                let texts: Vec<&str> = files.iter().map(String::as_str).collect();
                let outcome = check(
                    &texts,
                    configuration.testing,
                    configuration.prelude,
                    &RUNTIME_SEEDS,
                    &exclusions,
                );
                let runtime_testing = Index::build(&texts).members("pert_ui_runtime::testing");
                (configuration.name, outcome, runtime_testing)
            })
            .collect()
    })
}

/// Assertion 1, with the parse, resolution and ambiguity guards: every
/// reached pert-ui type is in `testing`, in the prelude or excluded.
#[test]
fn every_reached_type_is_in_testing_or_the_prelude() {
    let mut failures = Vec::new();
    for (name, outcome, _) in outcomes() {
        for problem in &outcome.problems {
            failures.push(format!("{name}: {problem}"));
        }
        for (missing, path) in &outcome.missing {
            failures.push(format!(
                "{name}: `{missing}` is not in `testing` or the prelude, reached by {path}"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// Assertion 2: every exclusion is reached in some configuration and is not
/// a member of `testing`.
#[test]
fn every_exclusion_is_reached_and_not_a_member() {
    let mut failures = Vec::new();
    for (excluded, module) in EXCLUSIONS {
        if !outcomes()
            .iter()
            .any(|(_, outcome, _)| outcome.reached_exclusions.contains(*excluded))
        {
            failures.push(format!(
                "excluded `{excluded}` ({module}) is reached in no configuration"
            ));
        }
        for (name, outcome, _) in outcomes() {
            if outcome.testing.contains(*excluded) {
                failures.push(format!(
                    "{name}: excluded `{excluded}` is a member of `testing`"
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Assertion 3: the facade's `testing` holds every name the runtime's does.
#[test]
fn the_facade_testing_keeps_the_runtime_list() {
    let mut failures = Vec::new();
    for (name, outcome, runtime_testing) in
        outcomes().iter().filter(|(name, ..)| *name != "runtime")
    {
        assert!(
            !runtime_testing.is_empty(),
            "{name}: the runtime listing has no `testing` module"
        );
        for missing in runtime_testing.difference(&outcome.testing) {
            failures.push(format!("{name}: `pert_ui::testing` lacks `{missing}`"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Assertion 4: the seed methods exist as inherent `Runtime` methods.
#[test]
fn the_runtime_seeds_exist() {
    for (name, outcome, _) in outcomes() {
        let seeds: Vec<&String> = outcome
            .problems
            .iter()
            .filter(|p| p.starts_with("seed"))
            .collect();
        assert!(seeds.is_empty(), "{name}: {seeds:?}");
    }
}

/// The rules, each on an inline fragment in the listing format.
mod rules {
    use super::check;

    const HEAD: &str = "pub mod pert_ui_runtime\npub mod pert_ui_runtime::prelude\npub mod pert_ui_runtime::testing\n";

    fn run(body: &str, exclusions: &[&str]) -> super::Outcome {
        let listing = format!("{HEAD}{body}");
        check(
            &[&listing],
            "pert_ui_runtime::testing",
            "pert_ui_runtime::prelude",
            &[],
            exclusions,
        )
    }

    fn missing(body: &str) -> Vec<String> {
        let outcome = run(body, &[]);
        assert!(outcome.problems.is_empty(), "{:?}", outcome.problems);
        outcome.missing.into_keys().collect()
    }

    #[test]
    fn a_field_type_is_followed() {
        let body = "pub struct pert_ui_runtime::testing::A\n\
                    pub pert_ui_runtime::testing::A::b: core::option::Option<pert_ui_runtime::B>\n\
                    pub struct pert_ui_runtime::B\n";
        assert_eq!(missing(body), ["B"]);
    }

    #[test]
    fn a_tuple_payload_is_followed() {
        let body = "pub enum pert_ui_runtime::testing::A\n\
                    pub pert_ui_runtime::testing::A::Some(alloc::vec::Vec<pert_ui_runtime::B>, u32)\n\
                    pub pert_ui_runtime::testing::A::None\n\
                    pub struct pert_ui_runtime::B\n";
        assert_eq!(missing(body), ["B"]);
    }

    #[test]
    fn a_struct_variant_field_belongs_to_its_enum() {
        // The variant is named like a declared type; the enum stays the owner.
        let body = "pub enum pert_ui_runtime::testing::A\n\
                    pub pert_ui_runtime::testing::A::C\n\
                    pub pert_ui_runtime::testing::A::C::field: pert_ui_runtime::B\n\
                    pub struct pert_ui_runtime::B\n\
                    pub struct pert_ui_runtime::C\n";
        let outcome = run(body, &[]);
        assert!(outcome.problems.is_empty(), "{:?}", outcome.problems);
        assert_eq!(outcome.missing.keys().collect::<Vec<_>>(), ["B"]);
        assert!(
            outcome.missing["B"].contains("A::C::field"),
            "{:?}",
            outcome.missing
        );
    }

    #[test]
    fn inherent_methods_are_followed_and_trait_implementations_are_not() {
        let body = "pub struct pert_ui_runtime::testing::A\n\
                    impl pert_ui_runtime::A\n\
                    pub fn pert_ui_runtime::A::get(&self, &[pert_ui_runtime::B]) -> (pert_ui_runtime::C, u8)\n\
                    impl core::convert::From<pert_ui_runtime::D> for pert_ui_runtime::A\n\
                    pub fn pert_ui_runtime::A::from(pert_ui_runtime::D) -> Self\n\
                    pub struct pert_ui_runtime::B\n\
                    pub struct pert_ui_runtime::C\n\
                    pub struct pert_ui_runtime::D\n";
        assert_eq!(missing(body), ["B", "C"]);
    }

    #[test]
    fn an_impl_fn_parameter_is_followed() {
        let body = "pub struct pert_ui_runtime::testing::A\n\
                    impl<'a> pert_ui_runtime::A<'a>\n\
                    pub fn pert_ui_runtime::A<'a>::until(&mut self, impl core::ops::function::FnMut(&pert_ui_runtime::B) -> core::result::Result<bool, pert_ui_runtime::C>)\n\
                    pub struct pert_ui_runtime::B\n\
                    pub struct pert_ui_runtime::C\n";
        assert_eq!(missing(body), ["B", "C"]);
    }

    #[test]
    fn a_callback_bound_is_followed_as_an_impl_parameter_is() {
        // The same callback, as an `impl FnOnce` parameter and as a bound on
        // the method's own generic parameter.
        let spellings = [
            "pub fn pert_ui_runtime::A::run(&self, impl core::ops::function::FnOnce(pert_ui_runtime::B) -> pert_ui_runtime::C)",
            "pub fn pert_ui_runtime::A::run<F: core::ops::function::FnOnce(pert_ui_runtime::B) -> pert_ui_runtime::C>(&self, F)",
            "pub fn pert_ui_runtime::A::run<F>(&self, F) where F: core::ops::function::FnOnce(pert_ui_runtime::B) -> pert_ui_runtime::C",
        ];
        for method in spellings {
            let body = format!(
                "pub struct pert_ui_runtime::testing::A\n\
                 impl pert_ui_runtime::A\n\
                 {method}\n\
                 pub struct pert_ui_runtime::B\n\
                 pub struct pert_ui_runtime::C\n"
            );
            assert_eq!(missing(&body), ["B", "C"], "{method}");
        }
    }

    #[test]
    fn a_nested_error_is_not_followed() {
        let body = "pub enum pert_ui_runtime::testing::Failure\n\
                    pub pert_ui_runtime::testing::Failure::Drawing(pert_ui_runtime::Inner)\n\
                    pub pert_ui_runtime::testing::Failure::Refused(pert_ui_runtime::Reason)\n\
                    impl core::error::Error for pert_ui_runtime::Failure\n\
                    pub fn pert_ui_runtime::Failure::source(&self) -> core::option::Option<&(dyn core::error::Error + 'static)>\n\
                    pub enum pert_ui_runtime::Inner\n\
                    impl core::error::Error for pert_ui_runtime::Inner\n\
                    pub struct pert_ui_runtime::Reason\n";
        assert_eq!(missing(body), ["Reason"]);
    }

    #[test]
    fn the_prelude_stops_the_traversal() {
        let body = "pub struct pert_ui_runtime::prelude::P\n\
                    pub pert_ui_runtime::prelude::P::q: pert_ui_runtime::Q\n\
                    pub struct pert_ui_runtime::testing::A\n\
                    pub pert_ui_runtime::testing::A::p: pert_ui_runtime::P\n\
                    pub struct pert_ui_runtime::Q\n";
        assert!(missing(body).is_empty());
    }

    #[test]
    fn an_exclusion_stops_the_traversal_and_is_recorded() {
        let body = "pub struct pert_ui_runtime::testing::A\n\
                    pub pert_ui_runtime::testing::A::x: pert_ui_runtime::X\n\
                    pub struct pert_ui_runtime::X\n\
                    pub pert_ui_runtime::X::y: pert_ui_runtime::Y\n\
                    pub struct pert_ui_runtime::Y\n";
        let outcome = run(body, &["X"]);
        assert!(outcome.missing.is_empty(), "{:?}", outcome.missing);
        assert!(outcome.reached_exclusions.contains("X"));
    }

    #[test]
    fn an_unused_exclusion_is_not_reached() {
        let body = "pub struct pert_ui_runtime::testing::A\npub struct pert_ui_runtime::Z\n";
        let outcome = run(body, &["Z"]);
        assert!(outcome.reached_exclusions.is_empty());
    }

    #[test]
    fn a_name_two_declarations_disagree_on_is_ambiguous() {
        let body = "pub struct pert_ui_runtime::testing::A\n\
                    pub pert_ui_runtime::testing::A::x: u8\n\
                    pub struct pert_ui_runtime::other::A\n\
                    pub pert_ui_runtime::other::A::y: u8\n";
        let outcome = run(body, &[]);
        assert!(
            outcome
                .problems
                .iter()
                .any(|p| p.starts_with("ambiguous name `A`")),
            "{:?}",
            outcome.problems
        );
        let core = "pub mod pert_ui_core\npub struct pert_ui_core::A\n";
        let runtime = format!("{HEAD}pub struct pert_ui_runtime::testing::A\n");
        let outcome = check(
            &[&runtime, core],
            "pert_ui_runtime::testing",
            "pert_ui_runtime::prelude",
            &[],
            &[],
        );
        assert!(
            outcome
                .problems
                .iter()
                .any(|p| p.contains("declared by pert_ui_core and pert_ui_runtime")),
            "{:?}",
            outcome.problems
        );
    }

    #[test]
    fn repeated_declarations_of_one_item_agree() {
        let body = "pub struct pert_ui_runtime::A\n\
                    pub pert_ui_runtime::A::b: pert_ui_runtime::B\n\
                    pub struct pert_ui_runtime::testing::A\n\
                    pub pert_ui_runtime::testing::A::b: pert_ui_runtime::B\n\
                    pub struct pert_ui_runtime::B\n";
        assert_eq!(missing(body), ["B"]);
    }

    #[test]
    fn an_unknown_line_fails_with_its_text() {
        let outcome = run(
            "pub struct pert_ui_runtime::testing::A\npub macro pert_ui_runtime::m\n",
            &[],
        );
        assert!(
            outcome
                .problems
                .iter()
                .any(|p| p.contains("pub macro pert_ui_runtime::m")),
            "{:?}",
            outcome.problems
        );
        let outcome = run(
            "pub struct pert_ui_runtime::A\nimpl pert_ui_runtime::A\npub type pert_ui_runtime::A::Output = u8\n",
            &[],
        );
        assert!(
            outcome
                .problems
                .iter()
                .any(|p| p.starts_with("unknown line")),
            "{:?}",
            outcome.problems
        );
    }

    #[test]
    fn an_unresolved_type_path_fails() {
        let body = "pub struct pert_ui_runtime::testing::A\npub pert_ui_runtime::testing::A::b: pert_ui_runtime::Missing\n";
        let outcome = run(body, &[]);
        assert!(
            outcome
                .problems
                .iter()
                .any(|p| p.contains("unresolved type path `pert_ui_runtime::Missing`")),
            "{:?}",
            outcome.problems
        );
    }

    // The associated-type forms, copied from the committed listings.

    #[test]
    fn a_trait_implementation_associated_type_is_not_a_declaration() {
        let body = "pub struct pert_ui_runtime::testing::Magnification(_)\n\
                    impl core::convert::TryFrom<f32> for pert_ui_runtime::Magnification\n\
                    pub type pert_ui_runtime::Magnification::Error = pert_ui_runtime::GeometryError\n\
                    pub fn pert_ui_runtime::Magnification::try_from(f32) -> core::result::Result<Self, Self::Error>\n\
                    impl core::convert::From<pert_ui_runtime::Magnification> for f32\n\
                    pub fn f32::from(pert_ui_runtime::Magnification) -> Self\n\
                    pub struct pert_ui_runtime::GeometryError\n";
        assert!(missing(body).is_empty());
    }

    #[test]
    fn differing_associated_types_of_one_path_are_not_ambiguous() {
        let core = "pub mod pert_ui_core\n\
                    pub struct pert_ui_core::ChoiceId(_)\n\
                    impl core::convert::TryFrom<&str> for pert_ui_core::ChoiceId\n\
                    pub type pert_ui_core::ChoiceId::Error = pert_ui_core::ParameterError\n\
                    pub fn pert_ui_core::ChoiceId::try_from(&str) -> core::result::Result<Self, Self::Error>\n\
                    impl core::convert::TryFrom<alloc::string::String> for pert_ui_core::ChoiceId\n\
                    pub type pert_ui_core::ChoiceId::Error = pert_ui_core::ParameterError\n\
                    pub fn pert_ui_core::ChoiceId::try_from(alloc::string::String) -> core::result::Result<Self, Self::Error>\n\
                    impl core::convert::TryFrom<pert_ui_core::AnyValue> for pert_ui_core::ChoiceId\n\
                    pub type pert_ui_core::ChoiceId::Error = pert_ui_core::AnyValue\n\
                    pub fn pert_ui_core::ChoiceId::try_from(pert_ui_core::AnyValue) -> core::result::Result<Self, Self::Error>\n\
                    pub enum pert_ui_core::AnyValue\n\
                    pub enum pert_ui_core::ParameterError\n";
        let runtime = format!(
            "{HEAD}pub struct pert_ui_runtime::testing::A\npub pert_ui_runtime::testing::A::id: pert_ui_core::ChoiceId\n"
        );
        let outcome = check(
            &[&runtime, core],
            "pert_ui_runtime::testing",
            "pert_ui_runtime::prelude",
            &[],
            &[],
        );
        assert!(outcome.problems.is_empty(), "{:?}", outcome.problems);
        assert_eq!(outcome.missing.keys().collect::<Vec<_>>(), ["ChoiceId"]);
    }

    #[test]
    fn a_trait_declaration_associated_type_is_a_member() {
        let core = "pub mod pert_ui_core\n\
                    pub trait pert_ui_core::HostValue: core::marker::Sized\n\
                    pub type pert_ui_core::HostValue::Canonical: pert_ui_core::ParameterValue\n\
                    pub type pert_ui_core::HostValue::Error: core::error::Error + 'static\n\
                    pub fn pert_ui_core::HostValue::from_canonical(Self::Canonical) -> core::result::Result<Self, Self::Error>\n\
                    pub fn pert_ui_core::HostValue::to_canonical(&self) -> core::result::Result<Self::Canonical, Self::Error>\n\
                    pub trait pert_ui_core::ParameterValue: pert_ui_core::family::sealed::ParameterValueSeal + core::clone::Clone\n";
        let runtime = format!(
            "{HEAD}pub struct pert_ui_runtime::testing::A\npub pert_ui_runtime::testing::A::v: pert_ui_core::HostValue\n"
        );
        let outcome = check(
            &[&runtime, core],
            "pert_ui_runtime::testing",
            "pert_ui_runtime::prelude",
            &[],
            &[],
        );
        assert!(outcome.problems.is_empty(), "{:?}", outcome.problems);
        assert_eq!(
            outcome.missing.keys().collect::<Vec<_>>(),
            ["HostValue", "ParameterValue"]
        );
    }

    #[test]
    fn a_primitive_owner_is_ignored_with_its_block() {
        let core = "pub mod pert_ui_core\n\
                    pub enum pert_ui_core::AnyValue\n\
                    pub pert_ui_core::AnyValue::Boolean(bool)\n\
                    impl core::convert::TryFrom<pert_ui_core::AnyValue> for bool\n\
                    pub type bool::Error = pert_ui_core::AnyValue\n\
                    pub fn bool::try_from(pert_ui_core::AnyValue) -> core::result::Result<Self, Self::Error>\n";
        let runtime = format!("{HEAD}pub struct pert_ui_runtime::testing::A\n");
        let outcome = check(
            &[&runtime, core],
            "pert_ui_runtime::testing",
            "pert_ui_runtime::prelude",
            &[],
            &[],
        );
        assert!(outcome.problems.is_empty(), "{:?}", outcome.problems);
        assert!(outcome.missing.is_empty());
    }

    #[test]
    fn same_name_aliases_with_different_targets_are_ambiguous() {
        let body = "pub struct pert_ui_runtime::testing::A\n\
                    pub pert_ui_runtime::testing::A::list: pert_ui_runtime::Items\n\
                    pub type pert_ui_runtime::Items = alloc::vec::Vec<pert_ui_runtime::Item>\n\
                    pub mod pert_ui_runtime::other\n\
                    pub type pert_ui_runtime::other::Items = alloc::vec::Vec<pert_ui_runtime::Other>\n\
                    pub struct pert_ui_runtime::Item\n\
                    pub struct pert_ui_runtime::Other\n";
        let outcome = run(body, &[]);
        assert!(
            outcome
                .problems
                .iter()
                .any(|p| p.starts_with("ambiguous name `Items`")),
            "{:?}",
            outcome.problems
        );
        // Repeats of one alias, under two paths, agree.
        let body = "pub struct pert_ui_runtime::testing::A\n\
                    pub pert_ui_runtime::testing::A::list: pert_ui_runtime::Items\n\
                    pub type pert_ui_runtime::Items = alloc::vec::Vec<pert_ui_runtime::Item>\n\
                    pub mod pert_ui_runtime::other\n\
                    pub type pert_ui_runtime::other::Items = alloc::vec::Vec<pert_ui_runtime::other::Item>\n\
                    pub struct pert_ui_runtime::Item\n";
        assert_eq!(missing(body), ["Item", "Items"]);
    }

    #[test]
    fn a_testing_member_without_a_declaration_fails() {
        // A re-export renamed with `as`: the module lists the new name, while
        // the declaration index knows only the defining one.
        // A single-letter type name fails too, while a function and a
        // constant, known by their listing lines in the defining package, do
        // not.
        let core = "pub mod pert_ui_core\n\
                    pub struct pert_ui_core::Original\n\
                    pub fn pert_ui_core::helper() -> u8\n\
                    pub const pert_ui_core::LIMIT: usize\n";
        let runtime = format!(
            "{HEAD}pub use pert_ui_runtime::testing::B\n\
             pub use pert_ui_runtime::testing::LIMIT\n\
             pub use pert_ui_runtime::testing::Renamed\n\
             pub use pert_ui_runtime::testing::helper\n"
        );
        let outcome = check(
            &[&runtime, core],
            "pert_ui_runtime::testing",
            "pert_ui_runtime::prelude",
            &[],
            &[],
        );
        assert_eq!(
            outcome.problems,
            [
                "`testing` member `B` resolves to no declared type",
                "`testing` member `Renamed` resolves to no declared type",
            ]
        );
    }

    #[test]
    fn a_standalone_alias_is_a_declaration_with_one_edge() {
        let body = "pub struct pert_ui_runtime::testing::A\n\
                    pub pert_ui_runtime::testing::A::list: pert_ui_runtime::Items\n\
                    pub type pert_ui_runtime::Items = alloc::vec::Vec<pert_ui_runtime::Item>\n\
                    pub struct pert_ui_runtime::Item\n";
        assert_eq!(missing(body), ["Item", "Items"]);
    }
}
