//! The Controls › Chips & breadcrumbs page: static and removable chips owned
//! by application data, and breadcrumbs over a sample folder path.
use crate::gallery::{label, page, section};
use pert_ui::*;

const TAGS: [&str; 5] = ["Draft", "Review", "Urgent", "Shared", "Archived"];

/// The application side of the removable chips: it holds the tags, describes
/// one chip per tag keyed by the tag, and describes them again when the data
/// changes. A chip only reports removal; this owner removes the tag.
struct TagEditor {
    tags: Signal<Vec<String>>,
    status: Signal<String>,
    shown: Option<Vec<String>>,
}
impl TagEditor {
    fn describe(&self, tags: &[String]) -> Vec<Element> {
        let chips = tags
            .iter()
            .map(|tag| {
                let (data, status, removed) = (self.tags.clone(), self.status.clone(), tag.clone());
                Element::new(
                    tag.clone(),
                    Chip::removable(tag.clone(), move |_| {
                        let mut list = data.get()?;
                        list.retain(|tag| *tag != removed);
                        data.set(list)?;
                        status.set(format!("Removed tag {removed}")).map(|_| ())
                    }),
                )
            })
            .collect::<Vec<_>>();
        let content = if chips.is_empty() {
            label("no-tags", "No tags. Press Reset tags.")
        } else {
            Element::new("chips", Wrap::new(chips))
        };
        vec![content]
    }
}
impl Component for TagEditor {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let tags = cx.read(&self.tags.read_only())?;
        if self.shown.as_ref() != Some(&tags) {
            cx.request_children(self.describe(&tags));
            self.shown = Some(tags);
        }
        Ok(())
    }
}

fn all_tags() -> Vec<String> {
    TAGS.iter().map(|tag| (*tag).to_owned()).collect()
}

fn chips_section(ui: &Runtime, status: &Signal<String>) -> Element {
    let tags = ui.signal(all_tags());
    let reset = tags.clone();
    section(
        "chip-section",
        "Chips",
        [
            label(
                "chip-help",
                "Static chips take no Tab stop. Removable chips take one each: Delete, \
                 Backspace or the × button asks the owner to remove the tag, and focus \
                 moves on to the next chip.",
            ),
            Element::new(
                "static-chips",
                Wrap::new([
                    Element::new("chip-info", Chip::new("Info").leading_icon(Icon::Info)),
                    Element::new("chip-plain", Chip::new("Plain")),
                    Element::new(
                        "chip-locked",
                        Chip::removable("Locked", |_| Ok(())).disabled(),
                    ),
                ]),
            ),
            Element::new(
                "tag-editor",
                TagEditor {
                    tags,
                    status: status.clone(),
                    shown: None,
                },
            ),
            Element::new(
                "reset-tags",
                Button::new("Reset tags", move |_| reset.set(all_tags()).map(|_| ())),
            ),
        ],
    )
}

/// A sample folder path, outermost first: (key, label).
const FOLDERS: [(&str, &str); 6] = [
    ("/", "Computer"),
    ("/home", "home"),
    ("/home/user", "user"),
    ("/home/user/projects", "projects"),
    ("/home/user/projects/album", "album"),
    ("/home/user/projects/album/mixes", "mixes"),
];

/// The application side of a breadcrumb: it keeps the current folder and
/// describes the breadcrumb again from its own update after a navigation.
struct Location {
    /// The breadcrumb's key, distinct for each location on the page.
    crumbs: &'static str,
    path: Signal<String>,
    status: Signal<String>,
    shown: Option<String>,
}
impl Component for Location {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let path = cx.read(&self.path.read_only())?;
        if self.shown.as_ref() == Some(&path) {
            return Ok(());
        }
        let segments = FOLDERS
            .iter()
            .filter(|(key, _)| path.starts_with(key))
            .map(|(key, label)| BreadcrumbSegment::new(*key, *label));
        let (target, status) = (self.path.clone(), self.status.clone());
        let breadcrumb = Breadcrumb::new(segments, move |_, key| {
            target.set(key.to_owned())?;
            status.set(format!("Navigated to {key}")).map(|_| ())
        })?;
        cx.request_children(vec![Element::new(self.crumbs, breadcrumb)]);
        self.shown = Some(path);
        Ok(())
    }
}

fn deepest() -> String {
    FOLDERS[FOLDERS.len() - 1].0.to_owned()
}

fn breadcrumb_section(ui: &Runtime, status: &Signal<String>) -> Result<Element, RuntimeError> {
    let path = ui.signal(deepest());
    let reset = path.clone();
    let location = |key: &str, crumbs| {
        Element::new(
            key,
            Location {
                crumbs,
                path: path.clone(),
                status: status.clone(),
                shown: None,
            },
        )
    };
    let narrow = SizeLimits::new(Size::new(0.0, 0.0)?, Constraints::new(Some(260.0), None)?)?;
    Ok(section(
        "breadcrumb-section",
        "Breadcrumbs",
        [
            label(
                "breadcrumb-help",
                "Activate a folder to navigate up; the last segment is the current folder. \
                 Both breadcrumbs follow one location. The narrow one collapses its middle \
                 folders behind the … button.",
            ),
            location("wide-location", "wide-crumbs"),
            Element::new(
                "narrow-frame",
                Container::new(location("narrow-location", "narrow-crumbs")).size_limits(narrow),
            ),
            Element::new(
                "open-deepest",
                Button::new("Open mixes", move |_| reset.set(deepest()).map(|_| ())),
            ),
        ],
    ))
}

/// Sample options for the multi-select choosers: (value, label).
const INSTRUMENTS: [(&str, &str); 16] = [
    ("bass", "Bass"),
    ("cello", "Cello"),
    ("clarinet", "Clarinet"),
    ("drums", "Drums"),
    ("flute", "Flute"),
    ("guitar", "Guitar"),
    ("harp", "Harp"),
    ("horn", "Horn"),
    ("oboe", "Oboe"),
    ("organ", "Organ"),
    ("piano", "Piano"),
    ("synth", "Synthesizer"),
    ("trombone", "Trombone"),
    ("trumpet", "Trumpet"),
    ("viola", "Viola"),
    ("violin", "Violin"),
];

fn instruments() -> Vec<Choice<&'static str>> {
    INSTRUMENTS
        .iter()
        .map(|(value, label)| {
            let choice = Choice::new(*value, *label);
            if *value == "organ" {
                choice.disabled()
            } else {
                choice
            }
        })
        .collect()
}

/// A chooser with a line below that mirrors its bound value.
fn chooser(
    ui: &Runtime,
    key: &str,
    chosen: &[&'static str],
    commit: MultiSelectCommit,
) -> Result<[Element; 2], RuntimeError> {
    let value = ui.signal(chosen.to_vec());
    let mirror = value
        .read_only()
        .map(|values| format!("Bound value: [{}]", values.join(", ")))?;
    let chooser = MultiSelect::new(ui, value, instruments())?
        .commit(commit)
        .placeholder("Choose instruments…");
    Ok([
        Element::new(key, chooser).with_label("Instruments"),
        Element::new(format!("{key}-value"), Label::new(mirror)),
    ])
}

fn multi_select_section(ui: &Runtime) -> Result<Element, RuntimeError> {
    let [each, each_value] = chooser(
        ui,
        "instruments",
        &["piano", "violin"],
        MultiSelectCommit::EachToggle,
    )?;
    let [close, close_value] = chooser(ui, "ensemble", &["cello"], MultiSelectCommit::OnClose)?;
    Ok(section(
        "multi-select-section",
        "Multi-select",
        [
            label(
                "multi-select-help",
                "Open a chooser with Enter, Down or a click. Type to filter; Up and Down move \
                 the highlight while focus stays in the search field, Enter or a click toggles. \
                 Escape clears the query, then closes. Organ is disabled. Remove a chosen \
                 instrument with its chip's × button, Delete or Backspace.",
            ),
            label(
                "each-help",
                "Writes every toggle at once (MultiSelectCommit::EachToggle):",
            ),
            each,
            each_value,
            label(
                "close-help",
                "Writes when the popup closes by a click outside or Tab; Escape discards \
                 (MultiSelectCommit::OnClose):",
            ),
            close,
            close_value,
        ],
    ))
}

pub(crate) fn chips_and_breadcrumbs(
    ui: &Runtime,
    status: &Signal<String>,
) -> Result<Element, RuntimeError> {
    Ok(page([
        chips_section(ui, status),
        multi_select_section(ui)?,
        breadcrumb_section(ui, status)?,
    ]))
}
