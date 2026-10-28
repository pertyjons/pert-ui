//! The prelude's closure is sufficient: each group of prelude components is
//! configured with every name of its vocabulary, in a constructor, builder,
//! callback pattern or match, from `use pert_ui::prelude::*` alone.
//!
//! Each child module imports nothing else from `pert_ui`; its compiling is the
//! evidence. The program clause's key names are written the same way.

/// Layout containers and their sizes, spacing, alignment and scroll reports.
mod layout {
    use pert_ui::prelude::*;
    use std::time::Duration;

    #[test]
    fn containers_are_configured_from_the_prelude() -> Result<(), Box<dyn std::error::Error>> {
        let mut ui = Runtime::new();
        let limits = SizeLimits::new(Size::new(10.0, 10.0)?, Constraints::new(None, Some(40.0))?)?;
        let boxed = Container::new(Label::new("Boxed").key("label"))
            .background(Color::SURFACE)
            .border_role(ColorRole::Primary)
            .padding(Spacing::Theme(MetricRole::GroupPadding))
            .cursor(CursorIcon::Move)
            .size_limits(limits);
        let grid = Grid::new(
            [Track::fixed(Length::new(40.0)?), Track::weighted(1.0)?],
            [Label::new("a").key("a"), Label::new("b").key("b")],
        )?
        .gap(Spacing::Fixed(Length::from(4)));
        let column = Column::new([boxed.key("box"), grid.key("grid")])
            .align(Align::Stretch)
            .justify(Justify::Start);
        let scrolled = ScrollArea::new(column.key("column"))
            .axes(ScrollAxes::Both)
            .on_scroll(|_, _, origin| {
                let _user = matches!(origin, ChangeOrigin::User);
                Ok(())
            });
        ui.mount_tree(scrolled.key("scroll"))?;
        ui.flush(Duration::ZERO)?;
        Ok(())
    }
}

/// Text, labels and editors.
mod text {
    use pert_ui::prelude::*;
    use std::time::Duration;

    #[test]
    fn text_and_editors_are_configured_from_the_prelude() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut ui = Runtime::new();
        let rich = RichText::new()
            .span(TextSpan::new("a").color_role(ColorRole::Primary))
            .tinted_icon(Icon::Settings, TextColor::Inherit);
        let format = NumberFormat::new(
            "-0.00",
            |value: f64| format!("{value:.2}"),
            |text: &str| text.parse().map_err(|_| NumberInputError::InvalidNumber),
        );
        let amount = DecimalInput::new(ui.signal(1.0))
            .format(format)
            .normalize(|value| {
                if value < 0.0 {
                    Err(InputRejection::new("Must not be negative"))
                } else {
                    Ok(value)
                }
            })?
            .style(TextStyle::Monospace);
        let search = SearchField::new(ui.signal(String::new())).on_key(|_, key| {
            let _submit = key == SearchKey::Submit;
            Ok(())
        });
        let content = Column::new([
            Label::new(rich).key("rich"),
            Label::new("Turned")
                .turned(QuarterTurn::Left)
                .overflow(TextOverflow::Ellipsis)
                .key("turned"),
            Link::new("More", |_| Ok(()))
                .underline(LinkUnderline::Hover)
                .key("link"),
            amount.key("amount"),
            search.key("search"),
        ]);
        ui.mount_tree(content.key("text"))?;
        ui.flush(Duration::ZERO)?;
        Ok(())
    }
}

/// Choice and value controls.
mod choice {
    use pert_ui::prelude::*;
    use std::time::Duration;

    struct Rows;
    impl ListItems for Rows {
        fn len(&self) -> usize {
            1
        }
        fn key(&self, index: usize) -> Option<String> {
            (index == 0).then(|| "row".to_owned())
        }
        fn build(&self, index: usize) -> Option<Element> {
            (index == 0).then(|| Label::new("Row").key("row"))
        }
    }

    #[test]
    fn choice_controls_are_configured_from_the_prelude() -> Result<(), Box<dyn std::error::Error>> {
        let mut ui = Runtime::new();
        let picked = ui.signal(vec!["a"]);
        let multi = MultiSelect::new(&ui, picked, [Choice::new("a", "A"), Choice::new("b", "B")])?
            .commit(MultiSelectCommit::OnClose);
        let rows = ui.signal(ListSource::new(Rows));
        let strip = VirtualList::new(rows, 24.0)?.orientation(Orientation::Horizontal);
        ui.mount_tree(Column::new([multi.key("multi"), strip.key("strip")]).key("choice"))?;
        ui.flush(Duration::ZERO)?;
        Ok(())
    }
}

/// Disclosure and tabs.
mod disclosure {
    use pert_ui::prelude::*;
    use std::time::Duration;

    #[test]
    fn disclosure_and_tabs_are_configured_from_the_prelude()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut ui = Runtime::new();
        let accordion = Accordion::new([Section::new(
            "general",
            "General",
            Label::new("Body").key("body"),
        )])
        .mode(ExpandMode::Single)
        .on_change(|_, change: &ExpansionChange| {
            let _ = (&change.expanded, change.origin);
            Ok(())
        });
        let tabs = Tabs::new([Tab::new("one", "One", Label::new("1").key("one"))]).on_select(
            |_, change: &SelectionChange| {
                let _ = (&change.selected, &change.previous);
                Ok(())
            },
        );
        ui.mount_tree(Column::new([accordion.key("accordion"), tabs.key("tabs")]).key("root"))?;
        ui.flush(Duration::ZERO)?;
        Ok(())
    }
}

/// Forms.
mod forms {
    use pert_ui::prelude::*;
    use std::time::Duration;

    #[test]
    fn forms_are_configured_from_the_prelude() -> Result<(), Box<dyn std::error::Error>> {
        let mut ui = Runtime::new();
        let fields = FormFields::new(&ui);
        let name: FieldHandle = fields.field("name")?;
        name.set(FieldStatus::error("Required"))?;
        let summary: ReadSignal<FormSummary> = fields.summary();
        let grid = PropertyGrid::new([Property::new(
            "title",
            "Title",
            TextField::new(ui.signal(String::new())).key("title"),
        )
        .status_signal(name.status())]);
        let form = Form::new([
            Field::new(TextField::new(ui.signal(String::new())).key("name"))
                .label("Name")
                .form(&name)
                .key("name-field"),
        ]);
        ui.mount_tree(Column::new([grid.key("grid"), form.key("form")]).key("root"))?;
        ui.flush(Duration::ZERO)?;
        assert_eq!(summary.get()?.first_error(), Some("name"));
        Ok(())
    }
}

/// Surfaces: dialogs, menus, popovers, toasts and the quick pick.
mod surfaces {
    use pert_ui::prelude::*;
    use std::time::Duration;

    #[test]
    fn surfaces_are_configured_from_the_prelude() -> Result<(), Box<dyn std::error::Error>> {
        let mut ui = Runtime::new();
        let dialog = DialogController::new(&ui);
        let status: ReadSignal<DialogStatus> = dialog.status();
        let menu = ContextMenu::new(
            &ui,
            Label::new("Target").key("target"),
            vec![MenuItem::new("copy", "Copy", |_| Ok(()))],
        )?;
        let _menu: ContextMenuController = menu.controller();
        let picks = QuickPickController::new(&ui);
        let done = ui.signal(false);
        let pick = QuickPick::new(
            picks.clone(),
            menu.key("menu"),
            move |_, item: &PickItem| {
                Ok(if item.key == "later" {
                    PickChoice::Pending(PickPending::new(done.clone(), |done: &bool| {
                        done.then_some(Ok(()))
                    }))
                } else {
                    PickChoice::Done
                })
            },
        )
        .matcher(PickMatcher::fuzzy());
        let shown = ui.signal(false);
        let popover = Popover::new(
            shown,
            Label::new("Anchor").key("anchor"),
            Label::new("Content").key("content"),
        )
        .on_dismiss(|_, reason| {
            let _escape = reason == SurfaceDismissal::Escape;
            Ok(())
        });
        let toasts = ToastController::new(&ui);
        let content = Column::new([
            AlertDialog::new(
                dialog.clone(),
                Label::new("Owner").key("owner"),
                "Title",
                "Message",
                "Accept",
            )
            .key("dialog"),
            pick.key("pick"),
            popover.key("popover"),
        ]);
        ui.mount_tree(ToastHost::new(toasts.clone(), content.key("content")).key("toasts"))?;
        ui.flush(Duration::ZERO)?;
        picks.open(vec![
            PickItem::new("now", "Now"),
            PickItem::new("later", "Later"),
        ])?;
        let _open = matches!(picks.status()?, PickStatus::Open | PickStatus::Pending);
        let receipt: ToastReceipt = toasts.push(Toast::new("Saved"))?;
        let _shown = matches!(
            receipt.status(),
            ToastStatus::Pending | ToastStatus::Visible
        );
        let _cancelled = matches!(
            status.get()?,
            DialogStatus::Finished(DialogOutcome::Cancelled)
        );
        Ok(())
    }
}

/// Collections: lists, filters, tables and trees.
mod collections {
    use pert_ui::prelude::*;
    use std::time::Duration;

    struct Rows;
    impl ListItems for Rows {
        fn len(&self) -> usize {
            2
        }
        fn key(&self, index: usize) -> Option<String> {
            (index < 2).then(|| index.to_string())
        }
        fn build(&self, index: usize) -> Option<Element> {
            (index < 2).then(|| Label::new(format!("Row {index}")).key("label"))
        }
        fn text(&self, index: usize) -> Option<String> {
            (index < 2).then(|| format!("Row {index}"))
        }
    }

    impl TableItems for Rows {
        fn len(&self) -> usize {
            2
        }
        fn key(&self, index: usize) -> Option<String> {
            (index < 2).then(|| index.to_string())
        }
        fn cell(&self, index: usize, _column: &str) -> Option<Element> {
            (index < 2).then(|| Label::new(format!("Cell {index}")).key("cell"))
        }
    }

    impl TreeItems for Rows {
        fn roots(&self) -> Vec<String> {
            vec!["root".to_owned()]
        }
        fn children(&self, _key: &str) -> Vec<String> {
            Vec::new()
        }
        fn build(&self, key: &str) -> Option<Element> {
            Some(Label::new(key.to_owned()).key("label"))
        }
    }

    #[test]
    fn collections_are_configured_from_the_prelude() -> Result<(), Box<dyn std::error::Error>> {
        let mut ui = Runtime::new();
        let source = ui.signal(ListSource::new(Rows));
        let selection = ListSelection::new(&ui, SelectionMode::Multiple);
        let filter = ListFilter::new(&ui, source)?.selection(selection.clone());
        let outcome: FilterOutcome = filter.filter("Row", &FilterMatcher::substring())?;
        let counts: FilterCounts = outcome.counts;
        assert_eq!(counts.total, 2);
        let controller = ListController::new(&ui);
        let list = VirtualList::new(filter.items(), 24.0)?
            .selection(selection)
            .controller(controller.clone())
            .drag_operations(DropOperations::MOVE)
            .placement(|_keys, point: &ListDropPoint| {
                let _ = point.index();
                Some(ListDropPlacement::new(
                    ListDropPosition::End,
                    ListDropMark::Line {
                        boundary: 0,
                        indent: 0.0,
                    },
                ))
            })
            .on_selection_change(|_, change: &ListSelectionChange| {
                let _ = (&change.active, change.origin);
                Ok(())
            })
            .reorderable(|_, drop: &ListDrop<'_>| {
                Ok(drop.operation() == DropOperation::Move && !drop.items().is_empty())
            });
        let filtered = CollectionFilter::new(&ui, FilterTarget::List(filter), list.key("list"))
            .hidden_active(HiddenActive::Clear);

        let table_selection = ListSelection::new(&ui, SelectionMode::Single);
        let table_controller = TableController::new(
            &ui,
            table_selection,
            [TableColumn::new("name", "Name").sortable(true)],
            TableSource::new(Rows),
        )?;
        table_controller.set_sort(Some(TableSort {
            column: "name".to_owned(),
            direction: SortDirection::Descending,
        }))?;
        let table = Table::new(&table_controller, 24.0)?
            .on_sort(|_, sort: &TableSort| {
                let _ = sort.direction == SortDirection::Ascending;
                Ok(())
            })
            .cell_editor(|edit: &mut CellEdit<'_>| {
                Ok(Label::new(edit.row().to_owned()).key("editor"))
            })
            // An edit-end callback matches what the prelude holds.
            .on_edit_end(|_, end: &CellEditEnd| {
                match end.reason() {
                    CellEditEndReason::Completed(EditorCompletion {
                        trigger:
                            EditorTrigger::Tab {
                                direction: FocusDirection::Backward,
                            },
                        outcome: EditorOutcome::Saved,
                        ..
                    }) => {}
                    CellEditEndReason::Hidden | CellEditEndReason::RowLeft => {}
                    _ => {}
                }
                let _ = (end.row(), end.column());
                Ok(())
            });
        let tree_controller = TreeController::new(
            &ui,
            ListSelection::new(&ui, SelectionMode::Single),
            TreeSource::new(Rows),
        )?;
        let tree = Tree::new(&tree_controller, 24.0)?;
        ui.mount_tree(
            Column::new([
                filtered.key("filtered"),
                table.key("table"),
                tree.key("tree"),
            ])
            .key("root"),
        )?;
        ui.flush(Duration::ZERO)?;
        let receipt: ListRevealReceipt = controller.reveal("1")?;
        let _found = !matches!(receipt.status(), ListRevealStatus::NotFound);
        Ok(())
    }
}

/// Files.
mod files {
    use pert_ui::prelude::*;
    use std::{cell::RefCell, rc::Rc, time::Duration};

    #[test]
    fn a_file_browser_is_served_from_the_prelude() -> Result<(), Box<dyn std::error::Error>> {
        let mut ui = Runtime::new();
        let requests: Rc<RefCell<Vec<DirectoryRequest>>> = Rc::default();
        let log = Rc::clone(&requests);
        let controller = FileBrowserController::new(
            &ui,
            ListSelection::new(&ui, SelectionMode::Single),
            ["/data"],
            move |request| log.borrow_mut().push(request),
        )?;
        ui.mount_tree(FileBrowser::new(&controller, 24.0)?.key("files"))?;
        ui.flush(Duration::ZERO)?;
        let pending: Vec<DirectoryRequest> = requests.borrow_mut().drain(..).collect();
        for request in pending {
            let response: DirectoryResponse = request.respond(Ok(vec![
                DirectoryEntry::new("/data/a.txt", EntryKind::File),
                DirectoryEntry::new("/data/link", EntryKind::Symlink(LinkTarget::Broken)),
                DirectoryEntry::unreadable(
                    "/data/secret",
                    ListingError::new(ListingErrorKind::PermissionDenied, "denied"),
                ),
            ]));
            let delivery = controller.deliver(response)?;
            let _applied = matches!(delivery, ListingDelivery::Applied);
        }
        ui.flush(Duration::ZERO)?;
        Ok(())
    }
}

/// Navigation: breadcrumbs, history and toolbars.
mod navigation {
    use pert_ui::prelude::*;
    use std::{rc::Rc, time::Duration};

    #[test]
    fn navigation_is_configured_from_the_prelude() -> Result<(), Box<dyn std::error::Error>> {
        let mut ui = Runtime::new();
        let history: NavigationHistory<String> = NavigationHistory::new(&ui)?;
        history.visit("home".to_owned(), "Home")?;
        let snapshot: Rc<HistorySnapshot<String>> = history.state().get()?;
        let id: HistoryEntryId = snapshot.current().ok_or("an entry")?.id();
        let navigator =
            HistoryNavigator::new(&ui, history.clone(), |_, entry: &HistoryEntry<String>| {
                let _ = entry.value();
                Ok(())
            })?;
        let buttons =
            HistoryButtons::new(history.clone(), Label::new("Page").key("page"), |_, _| {
                Ok(())
            });
        let crumbs = Breadcrumb::new([BreadcrumbSegment::new("root", "Root")], |_, _| Ok(()))?;
        let toolbar = Toolbar::new(
            &ui,
            [ToolGroup::new(
                "edit",
                [Button::new("Cut", |_| Ok(())).key("cut")],
            )],
        )?
        .overflow(ToolbarOverflow::Collapse);
        ui.mount_tree(
            Column::new([
                navigator.key("navigator"),
                buttons.key("buttons"),
                crumbs.key("crumbs"),
                toolbar.key("toolbar"),
            ])
            .key("root"),
        )?;
        ui.flush(Duration::ZERO)?;
        assert!(history.go_to(id)?.is_some());
        Ok(())
    }
}

/// Media and progress.
mod media {
    use pert_ui::prelude::*;
    use std::time::Duration;

    #[test]
    fn media_and_progress_are_configured_from_the_prelude() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut ui = Runtime::new();
        let pixels = RasterImage::from_srgba8(1, 1, vec![255, 0, 0, 255])?;
        let image = Image::new(pixels)
            .fit(ImageFit::Contain)
            .sampling(ImageSampling::Nearest);
        let progress = ProgressBar::new(ProgressStatus::of(1, 2));
        // A running task's fraction carries into the status it ends in, and a
        // match reads it back, without naming the fraction's type.
        let running = ProgressStatus::of(3, 8);
        let ended = ProgressStatus::Failed(running.fraction());
        let reached = match ended {
            ProgressStatus::Failed(reached) => reached.map(|r| r.get()),
            _ => None,
        };
        assert_eq!(reached, Some(0.375));
        let status = ui.signal(ProgressStatus::Indeterminate);
        let busy = ui.signal(true);
        let field_status = ui.signal(FieldStatus::Normal);
        let grid = PropertyGrid::new([Property::new(
            "title",
            "Title",
            TextField::new(ui.signal(String::new())).key("title"),
        )
        .status(FieldStatus::warning("Long"))]);
        ui.mount_tree(
            Column::new([
                image.key("image"),
                progress.key("progress"),
                ProgressBar::new(ended).key("ended"),
                ProgressBar::bound(&status).key("bound"),
                Spinner::new(true).key("spinner"),
                Spinner::bound(&busy).key("bound-spinner"),
                InlineReadout::new("Ready").key("readout"),
                Field::new(TextField::new(ui.signal(String::new())).key("name"))
                    .status(FieldStatus::error("Required"))
                    .key("fixed-field"),
                Field::new(TextField::new(ui.signal(String::new())).key("other"))
                    .status_signal(&field_status)
                    .key("bound-field"),
                grid.key("grid"),
            ])
            .key("root"),
        )?;
        ui.flush(Duration::ZERO)?;
        Ok(())
    }
}

/// Zoomable content.
mod zoom {
    use pert_ui::prelude::*;
    use std::time::Duration;

    #[test]
    fn zoomable_content_is_configured_from_the_prelude() -> Result<(), Box<dyn std::error::Error>> {
        let mut ui = Runtime::new();
        let zoom = ZoomController::new(&ui);
        zoom.set(ViewTransform::new(
            ViewScale::new(2.0)?,
            Point::new(0.0, 0.0)?,
        ))?;
        let view = ZoomView::new([(Point::new(10.0, 10.0)?, Label::new("Node").key("node"))])
            .controller(&zoom)
            .scale_limits(ScaleLimits::new(ViewScale::new(0.5)?, ViewScale::new(4.0)?))
            .wheel(WheelGesture::Zoom);
        ui.mount_tree(view.key("zoom"))?;
        ui.flush(Duration::ZERO)?;
        Ok(())
    }
}

/// The program clause: a shortcut's chord and scope, and the text command a
/// rename begins.
mod commands {
    use pert_ui::prelude::*;
    use std::time::Duration;

    #[test]
    fn a_shortcut_and_a_rename_are_written_from_the_prelude()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut ui = Runtime::new();
        let chord = KeyChord::new(CommandKey::Named(KeyCode::Delete), Modifiers::PRIMARY);
        ui.register_command("Delete", CommandScope::Application, vec![chord], |_, _| {
            Ok(())
        })?;
        let title = ui.mount_tree(InlineText::new(ui.signal("Title".to_owned())).key("title"))?;
        ui.flush(Duration::ZERO)?;
        ui.text_command(&title, TextCommand::BeginEdit)?;
        Ok(())
    }
}

/// A theme switch: the mode it writes is in the prelude, the theme services
/// stay at the crate root and `ui.theme()` returns the manager unnamed.
mod theme {
    use pert_ui::prelude::*;

    #[test]
    fn a_theme_switch_writes_the_mode_from_the_prelude() -> Result<(), Box<dyn std::error::Error>> {
        let ui = Runtime::new();
        ui.theme().set_mode(ThemeMode::Light)?;
        ui.theme().set_mode(ThemeMode::Dark)?;
        assert_eq!(ui.theme().mode()?, ThemeMode::Dark);
        let next = match ui.theme().mode()? {
            ThemeMode::Dark => ThemeMode::Light,
            ThemeMode::Light => ThemeMode::Dark,
        };
        assert_eq!(next, ThemeMode::Light);
        Ok(())
    }
}
