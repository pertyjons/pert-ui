//! The Navigation page: revealing hidden targets, selecting a tab by key and
//! jumping across the gallery.
use crate::gallery::{find_at, gallery_tabs, label, page, page_address, section, toggle};
use crate::samples::ReceiptReadout;
use pert_ui::*;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

/// The identities the page's actions reveal. A widget has no identity until
/// it is mounted, so the gallery fills this in after mounting, by key.
#[derive(Clone, Default)]
pub(crate) struct NavigationTargets(Rc<RefCell<BTreeMap<&'static str, WidgetId>>>);
impl NavigationTargets {
    /// The demo strip whose tabs the page opens.
    pub(crate) const STRIP: &'static str = "nav-strip";
    /// A button at the end of a long, initially hidden tab.
    pub(crate) const OUTPUT: &'static str = "output-level";
    /// A button inside a view whose visibility the application binds.
    pub(crate) const EXPERT: &'static str = "expert-option";
    /// A button in the closed section of an accordion on the first tab.
    pub(crate) const BURIED: &'static str = "buried-option";
    /// The gallery's Collections page.
    pub(crate) const COLLECTIONS: &'static str = "collections";
    /// The strip whose selections the history demo records.
    pub(crate) const HISTORY_STRIP: &'static str = "history-strip";
    /// A button inside the closed custom drawer.
    pub(crate) const DRAWER_SETTING: &'static str = "drawer-setting";

    /// The address of every target below the gallery's category strip, as
    /// (key, address inside the page). The Navigation page reuses keys such
    /// as `help` and `content` in every section, so each target is named by
    /// its full address rather than searched for by key.
    pub(crate) const ADDRESSES: [(&'static str, &'static [&'static str]); 6] = [
        (
            Self::STRIP,
            &["navigation-demo", "content", "nav-stage", Self::STRIP],
        ),
        (
            Self::OUTPUT,
            &[
                "navigation-demo",
                "content",
                "nav-stage",
                Self::STRIP,
                "mixer",
                "channels",
                Self::OUTPUT,
            ],
        ),
        (
            Self::EXPERT,
            &[
                "navigation-demo",
                "content",
                "nav-stage",
                Self::STRIP,
                "advanced",
                "advanced-view",
                Self::EXPERT,
            ],
        ),
        (
            Self::BURIED,
            &[
                "navigation-demo",
                "content",
                "nav-stage",
                Self::STRIP,
                "general",
                "general-sections",
                "more",
                Self::BURIED,
            ],
        ),
        (
            Self::HISTORY_STRIP,
            &[
                "navigation-history",
                "content",
                "history-stage",
                Self::HISTORY_STRIP,
            ],
        ),
        (
            Self::DRAWER_SETTING,
            &[
                "navigation-custom",
                "content",
                "drawer",
                "drawer-contents",
                Self::DRAWER_SETTING,
            ],
        ),
    ];

    /// Records the targets once the gallery is mounted, each by its address
    /// through [`Runtime::find`].
    pub(crate) fn resolve(&self, ui: &Runtime, root: &WidgetId) -> Result<(), AppError> {
        let top = gallery_tabs(ui, root)?;
        let mut targets = self.0.borrow_mut();
        for (key, path) in Self::ADDRESSES {
            let address = page_address("layout-category", "navigation", path);
            targets.insert(key, find_at(ui, &top, &address)?);
        }
        let collections = find_at(
            ui,
            &top,
            &["data-category", "category-pages", Self::COLLECTIONS],
        )?;
        targets.insert(Self::COLLECTIONS, collections);
        Ok(())
    }

    /// The identity recorded for `key`, or `Unmounted` before the gallery has
    /// resolved it.
    pub(crate) fn get(&self, key: &str) -> Result<WidgetId, RuntimeError> {
        self.0
            .borrow()
            .get(key)
            .cloned()
            .ok_or(RuntimeError::Unmounted)
    }
}

/// Describes a selection change of the demo strip for the page's readout.
fn describe_selection(change: &SelectionChange) -> String {
    let origin = match change.origin {
        ChangeOrigin::User => "by the user",
        ChangeOrigin::Reveal => "by a reveal",
        ChangeOrigin::Restore => "by workspace restoration",
        ChangeOrigin::Redescription => "by a new description",
        ChangeOrigin::Presentation => "by a window change",
        _ => "by the application",
    };
    format!(
        "Strip selection: {}, {origin}",
        change.selected.as_deref().unwrap_or("none")
    )
}

/// Describes a reveal's progress for the page's readout.
fn describe(status: &RevealStatus) -> String {
    match status {
        RevealStatus::Pending => "Reveal: pending".to_owned(),
        RevealStatus::Finished(Ok(outcome)) => describe_outcome(outcome),
        RevealStatus::Finished(Err(error)) => format!("Reveal failed: {error}"),
        _ => "Reveal: cancelled".to_owned(),
    }
}

pub(crate) fn describe_outcome(outcome: &RevealOutcome) -> String {
    let activation = if outcome.activation().is_some() {
        ", its window asked for access"
    } else {
        ""
    };
    match outcome {
        RevealOutcome::Revealed { .. } => format!("Revealed{activation}"),
        RevealOutcome::RevealedAndFocused { .. } => format!("Revealed and focused{activation}"),
        RevealOutcome::Refused(RevealRefusal::ApplicationControlled { .. }) => {
            "Refused: the application controls that view's visibility".to_owned()
        }
        RevealOutcome::Refused(RevealRefusal::ModalBlocked) => {
            "Refused: a modal surface is open".to_owned()
        }
        RevealOutcome::Refused(RevealRefusal::Hidden { .. }) => {
            "Refused: the target is hidden explicitly".to_owned()
        }
        RevealOutcome::Refused(_) => "Refused: a container would not open".to_owned(),
        RevealOutcome::Unmounted => "The target was removed".to_owned(),
        _ => format!("{outcome:?}"),
    }
}

const HELP: &str = "The links reveal targets that are not visible yet: the strip below selects the tab that holds them, a closed accordion section opens, the scroll area scrolls to them, and \"and focus\" also moves focus. The advanced option sits in a view the application shows with the toggle, so revealing it is refused while the toggle is off. Selecting a tab by key runs the same transition as clicking its title. The line above the strip shows each selection change the strip reports, and whether the user, a reveal or a window change caused it.";

const HISTORY_HELP: &str = "Open the strip's places one after another, then go back and forward with the arrows, as in a browser. The arrows are disabled while there is nowhere to go, and their tooltips name the place they lead to. The clock icon lists the visited places, newest first: type to filter and press Enter to return to one. Only selections the user makes are recorded; returning to a place reveals its tab without recording it again.";

/// The history demo: a strip whose user selections a history records, and
/// the controls that move through it.
fn history_demo(ui: &Runtime, targets: &NavigationTargets) -> Result<Element, RuntimeError> {
    let history = NavigationHistory::new(ui)?;
    history.visit("alpha", "Alpha")?;
    let shown = ui.signal(String::from("Showing Alpha"));
    let (recording, record_shown) = (history.clone(), shown.clone());
    let strip = Tabs::new([
        Tab::new("alpha", "Alpha", label("alpha-text", "The first place.")),
        Tab::new("beta", "Beta", label("beta-text", "The second place.")),
        Tab::new("gamma", "Gamma", label("gamma-text", "The third place.")),
    ])
    .detachable(true)
    .on_select(move |_, change| {
        let Some(key) = change.selected.as_deref() else {
            return Ok(());
        };
        let place = match key {
            "beta" => ("beta", "Beta"),
            "gamma" => ("gamma", "Gamma"),
            _ => ("alpha", "Alpha"),
        };
        record_shown.set(format!("Showing {}", place.1))?;
        // Only the user's own selections are visits; a restoration reports
        // itself as a reveal and is not recorded again.
        if change.origin == ChangeOrigin::User {
            recording.visit(place.0, place.1)?;
        }
        Ok(())
    });
    let restoring = targets.clone();
    let navigator = HistoryNavigator::new(ui, history, move |cx, entry| {
        let strip = restoring.get(NavigationTargets::HISTORY_STRIP)?;
        let key = *entry.value();
        cx.defer_detached(move |ui| ui.reveal_child(&strip, key).map(|_| ()))
            .map(|_| ())
    })?
    .name("Demo history");
    Ok(section(
        "navigation-history",
        "Back, forward and history",
        [
            label("history-help", HISTORY_HELP),
            Element::new(
                "history-bar",
                Row::new([
                    Element::new("history-controls", navigator),
                    Element::new("history-shown", Label::new(shown.read_only())),
                ]),
            ),
            Element::new(
                "history-stage",
                Container::new(Element::new(NavigationTargets::HISTORY_STRIP, strip))
                    .preferred_size(Size::from([480, 120])),
            ),
        ],
    ))
}

/// A drawer written the way an application would write one, against the
/// public component contract only: it hides its one child until it is
/// opened, and opens itself when a reveal asks for something inside it.
/// Its open state is a signal it owns, so its own toggle and a reveal change
/// the same state, and the drawer reports `Changed` rather than refusing.
struct Drawer {
    child: Option<Element>,
    open: Signal<bool>,
    shown: bool,
}
impl Component for Drawer {
    fn take_children(&mut self) -> Vec<Element> {
        self.child.take().into_iter().collect()
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let open = cx.read(&self.open)?;
        if open != self.shown {
            self.shown = open;
            cx.request_layout();
        }
        Ok(())
    }
    fn child_visible(&self, _: &str) -> bool {
        self.shown
    }
    fn reveal_child(
        &mut self,
        cx: &mut EventContext,
        _: &str,
    ) -> Result<ChildReveal, RuntimeError> {
        self.open.set(true)?;
        self.shown = true;
        cx.invalidate(Invalidation::Layout);
        Ok(ChildReveal::Changed)
    }
}

const CUSTOM_HELP: &str = "A container the gallery writes itself, through the same public hook the built-in tabs and sections use: the drawer below hides its contents until it is opened. Open it with the switch, or follow the link, which reveals and focuses the setting inside it; the drawer opens itself to let the reveal through.";

/// The custom container demo: a drawer and the ways to open it.
fn custom_container_demo(
    ui: &Runtime,
    targets: &NavigationTargets,
) -> Result<Element, RuntimeError> {
    let open = ui.signal(false);
    let receipt = ui.signal(None::<RevealReceipt>);
    let (reveal_targets, reveal_receipt) = (targets.clone(), receipt.clone());
    Ok(section(
        "navigation-custom",
        "A custom container",
        [
            label("custom-help", CUSTOM_HELP),
            Element::new(
                "custom-actions",
                Wrap::new([
                    Element::new("drawer-open", Toggle::new("Drawer open", open.clone())),
                    Element::new(
                        "reveal-drawer",
                        Link::new("Reveal and focus the drawer's setting", move |cx| {
                            let target = reveal_targets.get(NavigationTargets::DRAWER_SETTING)?;
                            let started = cx.request_reveal(&target, true)?;
                            reveal_receipt.set(Some(started))
                        }),
                    ),
                ]),
            ),
            Element::new(
                "drawer-status",
                ReceiptReadout::new(
                    receipt.read_only(),
                    RevealReceipt::status_binding,
                    describe,
                    "Nothing revealed yet.",
                ),
            ),
            Element::new(
                "drawer",
                Drawer {
                    child: Some(Element::new(
                        "drawer-contents",
                        Column::new([
                            label("drawer-text", "The drawer's contents."),
                            Element::new(
                                NavigationTargets::DRAWER_SETTING,
                                Button::new("Drawer setting", |_| Ok(())),
                            ),
                        ]),
                    )),
                    open,
                    shown: false,
                },
            ),
        ],
    ))
}

/// A matcher that only matches at the start of words: every query word must
/// begin a word of the command's name, in any order, so "go col" finds
/// "Go to: Collections" while "llect" finds nothing. Earlier words rank
/// higher.
fn word_start_matcher() -> CommandMatcher {
    CommandMatcher::new(|query, entry| {
        let words: Vec<String> = entry
            .name
            .split(|c: char| !c.is_alphanumeric())
            .filter(|word| !word.is_empty())
            .map(str::to_lowercase)
            .collect();
        let mut penalty = 0_u32;
        for part in query.split_whitespace() {
            let part = part.to_lowercase();
            let position = words.iter().position(|word| word.starts_with(&part))?;
            penalty = penalty.saturating_add(u32::try_from(position).unwrap_or(u32::MAX));
        }
        Some(u32::MAX - penalty)
    })
}

const PALETTE_HELP: &str = "The gallery's own palette (Ctrl+Shift+P) is modal. This one is nonmodal: it opens beside the button, the rest of the page stays usable, and it closes when focus or a click leaves it. It also replaces the default fuzzy matcher with the gallery's own: every typed word must begin a word of the command's name, so \"go col\" finds Go to: Collections but \"llect\" finds nothing.";

/// The palette demo: a nonmodal palette over every command, with a custom
/// matcher.
fn palette_demo(ui: &Runtime) -> Result<Element, RuntimeError> {
    let controller = CommandPaletteController::new(ui);
    let opener = controller.clone();
    let anchor = Element::new(
        "palette-open",
        Button::new("Open the nonmodal palette", move |cx| {
            let context = cx.command_context(CommandSource::Button);
            let opener = opener.clone();
            cx.defer(move |ui| opener.open(ui, &context)).map(|_| ())
        }),
    );
    Ok(section(
        "navigation-palette",
        "A nonmodal palette with its own matcher",
        [
            label("palette-help", PALETTE_HELP),
            Element::new(
                "nonmodal-palette",
                CommandPalette::new(controller, anchor)
                    .nonmodal()
                    .matcher(word_start_matcher())
                    .placeholder("Type the start of each word"),
            ),
        ],
    ))
}

const JUMP_HELP: &str = "An internal link to another page, always underlined: it opens the Data & media category and its Collections page, two strips at once. A tab's view is not focusable, so focus stays here. The next link has rich content, an icon and a heading-styled span, and reveals and focuses the mixer's Output level above; its whole content is underlined and its plain text names it. Beside it, a disabled link is muted, skipped by Tab and ignores the pointer.";

pub(crate) fn navigation(
    ui: &Runtime,
    targets: &NavigationTargets,
    select_by_key: &CommandId,
) -> Result<Element, RuntimeError> {
    let advanced = ui.signal(false);
    let receipt = ui.signal(None::<RevealReceipt>);
    let selection = ui.signal(String::from("Strip selection: general, initially"));
    let record_selection = selection.clone();
    let reveal = |text: &str, target: &'static str, focus: bool| {
        let (targets, receipt) = (targets.clone(), receipt.clone());
        Link::new(text, move |cx| {
            let started = cx.request_reveal(&targets.get(target)?, focus)?;
            receipt.set(Some(started))
        })
    };
    let rich_reveal = |content: RichText, target: &'static str| {
        let (targets, receipt) = (targets.clone(), receipt.clone());
        Link::new(content, move |cx| {
            let started = cx.request_reveal(&targets.get(target)?, true)?;
            receipt.set(Some(started))
        })
    };
    let link = |key: &str, text: &str, target: &'static str, focus: bool| {
        Element::new(key, reveal(text, target, focus))
    };
    let mut rows: Vec<Element> = (1..=24)
        .map(|index| label(&format!("channel-{index}"), format!("Channel {index}")))
        .collect();
    rows.push(Element::new(
        NavigationTargets::OUTPUT,
        Button::new("Output level", |_| Ok(())),
    ));
    let select = select_by_key.clone();
    Ok(page([
        section(
            "navigation-demo",
            "Reveal a hidden target",
            [
                label("help", HELP),
                toggle("show-advanced", "Show advanced content", &advanced)?,
                Element::new(
                    "actions",
                    Wrap::new([
                        link(
                            "reveal-output",
                            "Reveal Output level",
                            NavigationTargets::OUTPUT,
                            false,
                        ),
                        link(
                            "focus-output",
                            "Reveal and focus Output level",
                            NavigationTargets::OUTPUT,
                            true,
                        ),
                        link(
                            "reveal-expert",
                            "Reveal the advanced option",
                            NavigationTargets::EXPERT,
                            true,
                        ),
                        link(
                            "reveal-buried",
                            "Reveal the buried option",
                            NavigationTargets::BURIED,
                            true,
                        ),
                        Element::new(
                            "select-general",
                            Button::new("Select General by key", move |cx| {
                                cx.invoke_command(
                                    &select,
                                    cx.command_context(CommandSource::Button),
                                )
                                .map(|_| ())
                            }),
                        ),
                    ]),
                ),
                Element::new(
                    "reveal-status",
                    ReceiptReadout::new(
                        receipt.read_only(),
                        RevealReceipt::status_binding,
                        describe,
                        "Nothing revealed yet.",
                    ),
                ),
                Element::new("selection-log", Label::new(selection.read_only())),
                Element::new(
                    "nav-stage",
                    Container::new(Element::new(
                        NavigationTargets::STRIP,
                        Tabs::new([
                            Tab::new(
                                "general",
                                "General",
                                Element::new(
                                    "general-sections",
                                    Accordion::new([
                                        Section::new(
                                            "summary",
                                            "Summary",
                                            label("general-text", "Nothing to see here."),
                                        ),
                                        Section::new(
                                            "more",
                                            "More options",
                                            Element::new(
                                                NavigationTargets::BURIED,
                                                Button::new("Buried option", |_| Ok(())),
                                            ),
                                        ),
                                    ])
                                    .mode(ExpandMode::Single)
                                    .expanded(["summary"])
                                    .persist(false),
                                ),
                            ),
                            Tab::new(
                                "mixer",
                                "Mixer",
                                Element::new("channels", Column::new(rows)),
                            ),
                            Tab::new(
                                "advanced",
                                "Advanced",
                                Element::new(
                                    "advanced-view",
                                    View::new(Element::new(
                                        NavigationTargets::EXPERT,
                                        Button::new("Expert option", |_| Ok(())),
                                    ))
                                    .visible_signal(advanced.read_only()),
                                ),
                            ),
                        ])
                        .detachable(true)
                        .on_select(move |_, change| {
                            record_selection.set(describe_selection(change))?;
                            Ok(())
                        }),
                    ))
                    .preferred_size(Size::from([480, 200])),
                ),
            ],
        ),
        history_demo(ui, targets)?,
        custom_container_demo(ui, targets)?,
        palette_demo(ui)?,
        section(
            "navigation-jump",
            "Jump to another page",
            [
                label("jump-help", JUMP_HELP),
                Element::new(
                    "jump-links",
                    Wrap::new([
                        Element::new(
                            "jump-collections",
                            reveal(
                                "Go to Data & media › Collections",
                                NavigationTargets::COLLECTIONS,
                                true,
                            )
                            .trailing_icon(Icon::ChevronRight)
                            .underline(LinkUnderline::Always),
                        ),
                        Element::new(
                            "jump-rich",
                            rich_reveal(
                                RichText::new()
                                    .icon(Icon::Tune)
                                    .gap()
                                    .text("Reveal the mixer's ")
                                    .span(
                                        TextSpan::new("Output level").font_role(FontRole::Heading),
                                    ),
                                NavigationTargets::OUTPUT,
                            ),
                        ),
                        Element::new(
                            "jump-unavailable",
                            Link::new("An unavailable destination", |_| Ok(())).disabled(),
                        ),
                    ]),
                ),
            ],
        ),
    ]))
}
