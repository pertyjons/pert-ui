//! The Data & media category: virtual lists, collections, drag and drop,
//! images and vector shapes. The live pages are in `crate::live`.
mod drag_and_drop;
pub(crate) mod files;
use crate::gallery::{label, page, section, stage};
use crate::samples::{ReceiptReadout, VectorShapes};
pub(crate) use drag_and_drop::drag_and_drop;
pub(crate) use files::file_browser;
use pert_ui::*;

/// One hundred thousand rows, of which only the ones near the viewport exist.
struct Catalog {
    count: usize,
    status: Signal<String>,
}
impl ListItems for Catalog {
    fn len(&self) -> usize {
        self.count
    }
    fn key(&self, index: usize) -> Option<String> {
        (index < self.count).then(|| format!("entry-{index}"))
    }
    fn build(&self, index: usize) -> Option<Element> {
        let key = self.key(index)?;
        let status = self.status.clone();
        Some(Element::new(
            key,
            Row::new([
                label("name", format!("Entry {}", index + 1)),
                Element::new(
                    "pick",
                    Button::new("Pick", move |_| {
                        status
                            .set(format!("Picked entry {}", index + 1))
                            .map(|_| ())
                    }),
                ),
            ]),
        ))
    }
    fn index_of(&self, key: &str) -> Option<usize> {
        key.strip_prefix("entry-")?
            .parse()
            .ok()
            .filter(|index| *index < self.count)
    }
}

/// Describes a row reveal's progress for the virtual list's readout.
fn describe_row_reveal(status: &ListRevealStatus) -> String {
    match status {
        ListRevealStatus::Pending => "Row reveal: waiting for the list".to_owned(),
        ListRevealStatus::Mounted(_) => "Row reveal: found and mounted".to_owned(),
        ListRevealStatus::NotFound => "Row reveal: no row has that key".to_owned(),
        ListRevealStatus::Superseded => "Row reveal: replaced by a newer jump".to_owned(),
        _ => "Row reveal: cancelled".to_owned(),
    }
}

pub(crate) fn virtual_list(ui: &Runtime, status: &Signal<String>) -> Result<Element, RuntimeError> {
    const ROWS: usize = 100_000;
    let catalog = ui.signal(ListSource::new(Catalog {
        count: ROWS,
        status: status.clone(),
    }));
    let controller = ListController::new(ui);
    let receipt = ui.signal(None::<ListRevealReceipt>);
    let reveal = |key: &str, text: &str, row: String| {
        let (controller, receipt) = (controller.clone(), receipt.clone());
        Element::new(
            key,
            Button::new(text, move |_| {
                // The receipt reports whether the row was found and mounted,
                // which the readout below shows when the list has served it.
                let started = controller.reveal(row.clone())?;
                receipt.set(Some(started)).map(|_| ())
            }),
        )
    };
    let entry = |number: usize| format!("entry-{}", number - 1);
    Ok(page([section(
        "virtual",
        "VirtualList with 100,000 rows",
        [
            label(
                "help",
                "Only the rows near the viewport are mounted, plus a few of overscan; scroll by wheel, keyboard or the bar, Tab through the mounted buttons, or jump to a row by key. A focused row stays mounted while scrolled away. The line below the buttons reports each jump's outcome: found and mounted, or no row with that key.",
            ),
            Element::new(
                "jumps",
                Wrap::new([
                    reveal("jump-first", "Reveal entry 1", entry(1)),
                    reveal("jump-middle", "Reveal entry 50,000", entry(50_000)),
                    reveal("jump-last", "Reveal entry 100,000", entry(ROWS)),
                    reveal(
                        "jump-missing",
                        "Reveal a missing entry",
                        "entry-none".into(),
                    ),
                ]),
            ),
            Element::new(
                "jump-status",
                ReceiptReadout::new(
                    receipt.read_only(),
                    ListRevealReceipt::status_binding,
                    describe_row_reveal,
                    "No row revealed yet.",
                ),
            ),
            stage(
                "virtual-stage",
                240,
                Element::new(
                    "catalog",
                    VirtualList::new(catalog.read_only(), 36.0)?.controller(controller.clone()),
                )
                .with_label("Catalog"),
            ),
        ],
    )]))
}

// A grid, a tree and a table: three consumers of the one virtualized list and the
// one keyed selection. None of them virtualizes or navigates by itself.
struct Swatches(usize);
impl ListItems for Swatches {
    fn len(&self) -> usize {
        self.0
    }
    fn key(&self, index: usize) -> Option<String> {
        (index < self.0).then(|| format!("swatch-{index}"))
    }
    fn build(&self, index: usize) -> Option<Element> {
        Some(Element::new(
            self.key(index)?,
            Container::new(Element::new(
                "name",
                Label::new(format!("Item {}", index + 1)),
            ))
            .padding(Length::from(8)),
        ))
    }
    // What the item shows, so a filter matches what the user reads.
    fn text(&self, index: usize) -> Option<String> {
        (index < self.0).then(|| format!("Item {}", index + 1))
    }
    fn index_of(&self, key: &str) -> Option<usize> {
        key.strip_prefix("swatch-")?
            .parse()
            .ok()
            .filter(|index| *index < self.0)
    }
}

/// A hierarchy generated from its keys: every folder has ten folders and ten
/// files, six levels deep, so it is far too large to build and is only ever asked
/// for the children of nodes the user expands.
struct Folders;
impl Folders {
    const DEPTH: usize = 6;
    fn is_folder(key: &str) -> bool {
        key.rsplit('/')
            .next()
            .is_some_and(|name| name.starts_with("folder"))
            && key.split('/').count() < Self::DEPTH
    }
}
impl TreeItems for Folders {
    fn roots(&self) -> Vec<String> {
        (1..=10).map(|index| format!("folder-{index}")).collect()
    }
    fn children(&self, key: &str) -> Vec<String> {
        if !Self::is_folder(key) {
            return Vec::new();
        }
        (1..=10)
            .map(|index| format!("{key}/folder-{index}"))
            .chain((1..=10).map(|index| format!("{key}/file-{index}.txt")))
            .collect()
    }
    fn can_expand(&self, key: &str) -> bool {
        Self::is_folder(key)
    }
    fn build(&self, key: &str) -> Option<Element> {
        Some(Element::new(key, Label::new(key.rsplit('/').next()?)))
    }
    fn text(&self, key: &str) -> Option<String> {
        key.rsplit('/').next().map(str::to_owned)
    }
    // The hierarchy is generated, so the gallery treats its first levels as
    // the part already listed; a filter searches those and never generates
    // the millions of nodes below.
    fn known_children(&self, key: &str) -> Option<Vec<String>> {
        (key.split('/').count() < Self::FILTERED).then(|| self.children(key))
    }
}
impl Folders {
    /// How many levels a filter searches.
    const FILTERED: usize = 3;
}

/// Notes of one to five sentences, so their wrapped heights differ.
struct Notes(usize);
impl ListItems for Notes {
    fn len(&self) -> usize {
        self.0
    }
    fn key(&self, index: usize) -> Option<String> {
        (index < self.0).then(|| format!("note-{index}"))
    }
    fn build(&self, index: usize) -> Option<Element> {
        let sentences = 1 + index % 5;
        let text = format!(
            "Note {}. {}",
            index + 1,
            "A sentence long enough to wrap in a narrow list. ".repeat(sentences)
        );
        Some(Element::new(
            self.key(index)?,
            Container::new(Element::new("text", Label::new(text))).padding(Length::from(6)),
        ))
    }
    fn index_of(&self, key: &str) -> Option<usize> {
        key.strip_prefix("note-")?
            .parse()
            .ok()
            .filter(|index| *index < self.0)
    }
}

/// 100,000 rows in whatever order the gallery, playing the application, chose.
struct Measurements {
    count: u32,
    descending: bool,
}
impl Measurements {
    fn number(&self, index: usize) -> Option<u32> {
        let index = u32::try_from(index)
            .ok()
            .filter(|index| *index < self.count)?;
        Some(if self.descending {
            self.count - index
        } else {
            index + 1
        })
    }
}
impl TableItems for Measurements {
    fn len(&self) -> usize {
        self.count as usize
    }
    fn key(&self, index: usize) -> Option<String> {
        self.number(index).map(|number| format!("sample-{number}"))
    }
    fn cell(&self, index: usize, column: &str) -> Option<Element> {
        let text = self.text(index, column)?;
        Some(Element::new("text", Label::new(text)))
    }
    // The same text the cell shows, so content search finds rows that are
    // not mounted by what they would show.
    fn text(&self, index: usize, column: &str) -> Option<String> {
        let number = self.number(index)?;
        Some(match column {
            "sample" => format!("Sample {number}"),
            "value" => format!("{:.3}", f64::from(number).sqrt()),
            "note" => (if number % 7 == 0 { "every seventh" } else { "" }).to_owned(),
            "square" => (u64::from(number) * u64::from(number)).to_string(),
            "cube" => (u64::from(number).pow(3)).to_string(),
            "parity" => (if number % 2 == 0 { "even" } else { "odd" }).to_owned(),
            _ => return None,
        })
    }
    fn index_of(&self, key: &str) -> Option<usize> {
        let number: u32 = key.strip_prefix("sample-")?.parse().ok()?;
        (1..=self.count).contains(&number).then(|| {
            (if self.descending {
                self.count - number
            } else {
                number - 1
            }) as usize
        })
    }
}

pub(crate) fn collections(ui: &Runtime) -> Result<Element, RuntimeError> {
    const ROWS: u32 = 100_000;
    // A selection's active key is an `Option<String>`; mapping it to text
    // gives an ordinary bound label that follows the selection.
    let picked = |key: &str, selection: &ListSelection| -> Result<Element, RuntimeError> {
        let text = selection.active().map(|active| match active {
            Some(key) => format!("Active item: {key}"),
            None => "Active item: none".to_owned(),
        })?;
        Ok(Element::new(key, Label::new(text)))
    };
    let grid_selection = ListSelection::new(ui, SelectionMode::Multiple);
    let swatches = ui.signal(ListSource::new(Swatches(10_000)));
    let grid_filter = ListFilter::new(ui, swatches.read_only())?.selection(grid_selection.clone());
    let grid = VirtualList::new(grid_filter.items(), 48.0)?
        .lanes(5)?
        .selection(grid_selection.clone());
    // A filter applies once typing pauses, since each query reads every row.
    let pause = std::time::Duration::from_millis(200);
    let tree_state = TreeController::new(
        ui,
        TreeSource::new(Folders),
        ListSelection::new(ui, SelectionMode::Single),
    )?;
    let table_state = TableController::new(
        ui,
        [
            TableColumn::new("sample", "Sample")
                .width(Length::new(160.0)?)
                .resizable(true)
                .sortable(true),
            TableColumn::new("value", "Square root")
                .width(Length::new(160.0)?)
                .resizable(true)
                .movable(true),
            TableColumn::new("square", "Square")
                .width(Length::new(160.0)?)
                .resizable(true)
                .movable(true),
            TableColumn::new("cube", "Cube")
                .width(Length::new(200.0)?)
                .resizable(true)
                .movable(true),
            TableColumn::new("parity", "Parity")
                .width(Length::new(120.0)?)
                .resizable(true)
                .movable(true),
            TableColumn::new("note", "Note").width(Length::new(200.0)?),
        ],
        TableSource::new(Measurements {
            count: ROWS,
            descending: false,
        }),
        ListSelection::new(ui, SelectionMode::Single),
    )?;
    let sorted = table_state.clone();
    let cell = ui.signal("Press Enter on a cell.".to_owned());
    let opened = cell.clone();
    let notes = VirtualList::estimated(ui.signal(ListSource::new(Notes(5_000))).read_only(), 40.0)?;
    Ok(page([
        section(
            "grid-section",
            "Grid: a list with five lanes, 10,000 items, multiple selection",
            [
                label(
                    "grid-help",
                    "Click an item, then use the arrows: Left and Right move by item, Up and Down by line, Page and Home/End further. Shift extends a range, Control moves without selecting and Space toggles. The filter hides items whose name does not match; selected items stay selected while hidden, and Down in the filter moves into the grid.",
                ),
                stage(
                    "grid-stage",
                    240,
                    Element::new(
                        "grid-filter",
                        CollectionFilter::new(
                            ui,
                            grid_filter,
                            Element::new("collections-grid", grid).with_label("Grid"),
                        )
                        .name("Filter items")
                        .debounce(pause),
                    ),
                ),
                picked("grid-active", &grid_selection)?,
            ],
        ),
        section(
            "tree-section",
            "Tree: folders asked for their children only when expanded",
            [
                label(
                    "tree-help",
                    "Click an arrow to expand, or use the keyboard: Right expands and then enters a folder, Left collapses and then leaves it. The hierarchy has over two million nodes and none of them exists until its parent opens. Type in the filter to show the matching folders and files of the first three levels with the folders above them; your own expansion returns when the filter clears.",
                ),
                stage(
                    "tree-stage",
                    260,
                    Element::new(
                        "tree-filter",
                        CollectionFilter::new(
                            ui,
                            tree_state.clone(),
                            Element::new("collections-tree", Tree::new(&tree_state, 26.0)?)
                                .with_label("Tree"),
                        )
                        .name("Filter folders and files")
                        .debounce(pause),
                    ),
                ),
                picked("tree-active", &tree_state.selection())?,
            ],
        ),
        section(
            "table-section",
            "Table: 100,000 rows under a header that stays",
            [
                label(
                    "table-help",
                    "Click Sample to sort; the gallery reorders its own data and hands it back. Drag a column edge to resize it. Drag the titles from Square root to Parity to reorder those columns, or press Alt with Left or Right on a cell; Sample and Note stay where they are. The columns need more than the table's width, so header and rows scroll together: use a horizontal wheel, the bar along the bottom, or move the active cell with Left and Right, which scrolls its column into view. Click a cell or use the arrows, then press Enter.",
                ),
                stage(
                    "table-stage",
                    280,
                    Element::new(
                        "table-filter",
                        CollectionFilter::new(
                            ui,
                            table_state.clone(),
                            Element::new(
                                "collections-table",
                                Table::new(&table_state, 28.0)?
                                    .on_sort(move |_, sort| {
                                        sorted.set_source(TableSource::new(Measurements {
                                            count: ROWS,
                                            descending: sort.direction == SortDirection::Descending,
                                        }))
                                    })
                                    .on_activate(move |_, row, column| {
                                        opened
                                            .set(format!(
                                                "Activated {row}, column {}.",
                                                column.unwrap_or("none")
                                            ))
                                            .map(|_| ())
                                    }),
                            )
                            .with_label("Measurements"),
                        )
                        .name("Filter samples")
                        .debounce(pause),
                    ),
                ),
                picked("table-active", &table_state.selection())?,
                Element::new("table-cell", Label::new(cell.read_only())),
            ],
        ),
        section(
            "notes-section",
            "Rows of their own height: 5,000 wrapped notes over an estimate",
            [
                label(
                    "notes-help",
                    "Each row is as tall as its wrapped text. Rows are estimated at 40 pixels until they are mounted and measured, so the scrollbar grows more exact as you scroll. Jump to the end and scroll upwards: rows measured above the view correct the offset, so what is on screen does not jump.",
                ),
                stage(
                    "notes-stage",
                    220,
                    Element::new("collections-notes", notes).with_label("Notes"),
                ),
            ],
        ),
    ]))
}

/// A translucent PNG, decoded once when the page is built and again on a worker
/// thread on request: decoding never happens in `paint`.
const BADGE: &[u8] = include_bytes!("../../assets/badge.png");
const MARK: &[u8] = include_bytes!("../../assets/mark.svg");
const DIAL: &[u8] = include_bytes!("../../assets/dial.svg");

pub(crate) fn images(ui: &Runtime) -> Result<Element, AppError> {
    // A captioned image in a fixed box, so the three fits can be compared.
    let framed = |key: &str, caption: &str, image: Image| {
        Element::new(
            key,
            Column::new([
                Element::new(
                    "frame",
                    Container::new(Element::new("image", image))
                        .background_role(ColorRole::SurfaceContainer),
                ),
                label("caption", caption),
            ])
            .gap(MetricRole::ItemGap),
        )
    };
    let box_size = Size::from([160, 96]);
    let fits = match images::decode(BADGE) {
        Ok(badge) => Element::new(
            "fits",
            Wrap::new([
                framed(
                    "contain",
                    "Contain: all of it, centered",
                    Image::new(badge.clone())
                        .size(box_size)
                        .description("A soft-edged color disc"),
                ),
                framed(
                    "cover",
                    "Cover: fills the box, clipped",
                    Image::new(badge.clone())
                        .size(box_size)
                        .fit(ImageFit::Cover),
                ),
                framed(
                    "fill",
                    "Fill: stretched",
                    Image::new(badge).size(box_size).fit(ImageFit::Fill),
                ),
            ])
            .gap(MetricRole::SectionGap),
        ),
        Err(error) => label("fits", format!("The gallery's PNG did not decode: {error}")),
    };
    // Pixel art built in code, which needs no decoder feature at all.
    let checker: Vec<u8> = (0..64)
        .flat_map(|i| {
            if (i % 8 + i / 8) % 2 == 0 {
                [230, 120, 20, 255]
            } else {
                [20, 90, 200, 255]
            }
        })
        .collect();
    let sampling = match RasterImage::from_srgba8(8, 8, checker) {
        Ok(checker) => Element::new(
            "sampling",
            Wrap::new([
                framed(
                    "nearest",
                    "Nearest: hard pixel edges",
                    Image::new(checker.clone())
                        .size(Size::from([96, 96]))
                        .sampling(ImageSampling::Nearest),
                ),
                framed(
                    "linear",
                    "Linear: filtered",
                    Image::new(checker).size(Size::from([96, 96])),
                ),
            ])
            .gap(MetricRole::SectionGap),
        ),
        Err(error) => label("sampling", format!("The checker image is invalid: {error}")),
    };
    // Decoding off the UI thread: a worker decodes, and a mailbox hands the
    // owned result to the UI thread, whose handler writes the binding.
    let decoded = ui.signal(None::<RasterImage>);
    let decode_status = ui.signal("Not decoded yet.".to_owned());
    let (target, status) = (decoded.clone(), decode_status.clone());
    let capacity = Capacity::new(1)?;
    let mailbox = ui.mailbox(
        capacity,
        move |result: Result<RasterImage, images::ImageError>| {
            match result {
                Ok(image) => {
                    target.set(Some(image))?;
                    status.set("Decoded on a worker thread.".to_owned())?;
                }
                Err(error) => {
                    status.set(format!("Decoding failed: {error}"))?;
                }
            }
            Ok(())
        },
    )?;
    let busy = decode_status.clone();
    // The button owns the mailbox, so the mailbox lives as long as the page.
    let decode = Button::new("Decode on a worker", move |_| {
        let sender = mailbox.sender();
        std::thread::spawn(move || {
            // Full means a result is already waiting and Closed that the page
            // is gone; either way this result is not needed.
            let _ = sender.send(images::decode(BADGE));
        });
        busy.set("Decoding…".to_owned()).map(|_| ())
    });
    // SVG: one monochrome mark rasterized at three sizes and tinted by the
    // theme, and one drawing that keeps the colors it declares.
    let icon = |key: &str, extent: u16, role: ColorRole| match Svg::tinted(MARK, role) {
        Ok(mark) => Element::new(
            key,
            mark.size(Size::from([extent, extent]))
                .description("A waveform mark"),
        ),
        Err(error) => label(key, format!("The mark did not parse: {error}")),
    };
    let drawings = Element::new(
        "drawings",
        Row::new([
            icon("mark-small", 16, ColorRole::OnSurface),
            icon("mark-medium", 24, ColorRole::Primary),
            icon("mark-large", 48, ColorRole::Success),
            match Svg::new(DIAL) {
                Ok(dial) => Element::new(
                    "dial",
                    dial.size(Size::from([48, 48]))
                        .description("A dial showing a quarter past ten"),
                ),
                Err(error) => label("dial", format!("The dial did not parse: {error}")),
            },
        ])
        .gap(MetricRole::SectionGap),
    );
    let formats: Vec<_> = images::ImageFormat::enabled()
        .map(|format| format!("{format:?}"))
        .collect();
    Ok(page([
        section(
            "fits-section",
            "A decoded PNG with transparency, in three fits",
            [
                label(
                    "fits-help",
                    "The same 96x64 image in a 160x96 box. Its soft edge is premultiplied, so it blends without a dark fringe in both themes.",
                ),
                fits,
            ],
        ),
        section(
            "sampling-section",
            "An 8x8 image magnified twelve times",
            [sampling],
        ),
        section(
            "worker-section",
            "Decoded on a worker thread",
            [
                label(
                    "worker-help",
                    "The button decodes the same PNG on a new thread. A mailbox carries the image to the UI thread, where its handler sets the binding this image follows.",
                ),
                Element::new("decode-on-worker", decode),
                Element::new("worker-status", Label::new(decode_status.read_only())),
                Element::new(
                    "worker-image",
                    Image::bound(decoded.read_only()).size(Size::from([96, 64])),
                ),
            ],
        ),
        section(
            "svg-section",
            "SVG drawings",
            [
                label(
                    "svg-help",
                    "The same monochrome mark at three sizes, each rasterized for the pixels it covers and tinted by a theme color, beside a drawing that keeps its own colors. Switch theme or density and the tint follows. SVG text is not supported: convert it to paths.",
                ),
                drawings,
            ],
        ),
        section(
            "formats-section",
            "Decoders in this build",
            [label(
                "formats",
                format!(
                    "Enabled by Cargo features: {}. Every other format reports which feature it needs.",
                    formats.join(", ")
                ),
            )],
        ),
    ]))
}

pub(crate) fn vector_shapes() -> Element {
    page([section(
        "shapes-section",
        "Paths, strokes and ellipses",
        [
            label(
                "shapes-help",
                "Stars filled with the nonzero and even-odd rules; butt, square and round caps over their endpoints; miter, bevel and round joins; a filled and a stroked ellipse, a rounded rectangle and a sampled trace. Every shape is one antialiased coverage mask, rasterized when the page paints.",
            ),
            Element::new("vector-shapes", VectorShapes),
        ],
    )])
}
