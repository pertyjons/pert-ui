//! Author verification inputs for the frozen round 3 head.
//! Execution evidence is recorded separately; this is not a library example.
#![forbid(unsafe_code)]

#[cfg(test)]
mod tests {
    use pert_ui::{ChildReveal, prelude::*, testing::*};
    use std::{cell::RefCell, rc::Rc};

    type Check = Result<(), Box<dyn std::error::Error>>;

    fn runtime() -> Result<Runtime, RuntimeError> {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(720.0, 480.0, 1.0)?)?;
        Ok(ui)
    }

    fn descendants(ui: &Runtime, root: &WidgetId, key: &str) -> WidgetId {
        ui.find_descendant(root, key)
            .expect("mounted tree")
            .unwrap_or_else(|| panic!("missing {key}"))
    }

    #[test]
    fn p1_tab_address_children_and_find_agree_before_and_after_settlement() -> Check {
        let mut ui = runtime()?;
        let tabs = ui.mount_tree(Tabs::new([]).key("tabs"))?;
        Driver::new(&mut ui).settle()?;
        assert!(Driver::new(&mut ui).address_children(&tabs)?.is_empty());
        ui.reconcile_children(
            &tabs,
            vec![
                Tab::new(
                    "note",
                    "Note",
                    TextArea::new(ui.signal("text".to_owned())).key("editor"),
                )
                .into_element(),
            ],
        )?;
        let before = Driver::new(&mut ui).address_children(&tabs)?;
        let found_before = ui.find(&tabs, &["note"])?;
        println!("P1 before settle: addresses={before:?}; found={found_before:?}");
        assert_eq!(
            before
                .iter()
                .find(|child| child.segment == "note")
                .map(|child| child.id.clone()),
            found_before
        );
        Driver::new(&mut ui).settle()?;
        let after = Driver::new(&mut ui).address_children(&tabs)?;
        assert_eq!(
            after
                .iter()
                .map(|child| child.segment.as_str())
                .collect::<Vec<_>>(),
            ["note"]
        );
        assert!(ui.find(&tabs, &["note", "editor"])?.is_some());
        Ok(())
    }

    #[test]
    fn p2_a_deferred_dynamic_write_is_settled_before_tab_key_resolution() -> Check {
        let mut ui = runtime()?;
        let show = ui.signal(false);
        let text = ui.signal("retained".to_owned());
        let root = ui.mount_tree(
            Dynamic::new(&show, move |_, show| {
                let mut tabs = vec![Tab::new(
                    "old",
                    "Old",
                    TextArea::new(text.clone()).key("editor"),
                )];
                if *show {
                    tabs.push(Tab::new(
                        "new",
                        "New",
                        Label::new("New content").key("content"),
                    ));
                }
                Tabs::new(tabs).key("tabs")
            })
            .key("dynamic"),
        )?;
        Driver::new(&mut ui).settle()?;
        let tabs = descendants(&ui, &root, "tabs");
        let editor = ui
            .find(&tabs, &["old", "editor"])?
            .ok_or("missing old editor")?;
        let recorded = Rc::new(RefCell::new(None));
        let write = recorded.clone();
        let target = tabs.clone();
        ui.defer(move |ui| {
            show.set(true)?;
            *write.borrow_mut() = Some(ui.reveal_child(&target, "new"));
            Ok(())
        })?;
        Driver::new(&mut ui).settle()?;
        println!("P2 deferred reveal: {:?}", recorded.borrow());
        assert!(matches!(
            recorded.borrow().as_ref(),
            Some(Ok(ChildReveal::Changed))
        ));
        assert_eq!(ui.find(&tabs, &["old", "editor"])?, Some(editor));
        let semantics = Driver::new(&mut ui).semantics(&tabs)?;
        assert_eq!(
            semantics
                .items
                .iter()
                .find(|item| item.selected)
                .and_then(|item| item.name.as_deref()),
            Some("New")
        );
        Ok(())
    }

    #[test]
    fn p3_and_p5_menu_titles_are_structural_and_items_lack_pointer_bounds() -> Check {
        let mut ui = runtime()?;
        let bar = MenuBar::new(
            &ui,
            vec![MenuItem::submenu(
                "file",
                "File",
                vec![MenuItem::new("save", "Save", |_| Ok(()))],
            )],
        )?
        .content(Label::new("Content").key("main"));
        let root = ui.mount_tree(bar.key("bar"))?;
        Driver::new(&mut ui).settle()?;
        let addresses = Driver::new(&mut ui).address_children(&root)?;
        assert_eq!(
            addresses
                .iter()
                .map(|child| child.segment.as_str())
                .collect::<Vec<_>>(),
            ["main"]
        );
        assert!(ui.find(&root, &["title/file"])?.is_none());
        let title = descendants(&ui, &root, "title/file");
        Driver::new(&mut ui).click(&title)?;
        let items = descendants(&ui, &title, "items");
        assert!(matches!(
            Driver::new(&mut ui).click_item(&items, 0),
            Err(TestError::ItemWithoutBounds { .. })
        ));
        Ok(())
    }

    #[test]
    fn p3_tab_title_items_have_no_pointer_bounds_on_the_frozen_head() -> Check {
        let mut ui = runtime()?;
        let root = ui.mount_tree(
            Tabs::new([
                Tab::new("a", "A", Label::new("A body").key("body")),
                Tab::new("b", "B", Label::new("B body").key("body")),
            ])
            .key("tabs"),
        )?;
        Driver::new(&mut ui).settle()?;
        assert!(matches!(
            Driver::new(&mut ui).click_item(&root, 1),
            Err(TestError::ItemWithoutBounds { .. })
        ));
        Ok(())
    }

    #[test]
    fn p3_context_menu_rows_also_have_no_pointer_bounds() -> Check {
        let mut ui = runtime()?;
        let context = ContextMenu::new(
            &ui,
            Button::new("Target", |_| Ok(())).key("target"),
            vec![MenuItem::new("run", "Run", |_| Ok(()))],
        )?;
        let root = ui.mount_tree(Column::new([context.key("context")]).key("root"))?;
        Driver::new(&mut ui).settle()?;
        let target = descendants(&ui, &root, "target");
        Driver::new(&mut ui).click(&target)?;
        Driver::new(&mut ui).key(KeyCode::ContextMenu)?;
        let items = descendants(&ui, &root, "items");
        assert!(matches!(
            Driver::new(&mut ui).click_item(&items, 0),
            Err(TestError::ItemWithoutBounds { .. })
        ));
        Ok(())
    }

    #[test]
    fn p4_dialog_content_has_the_dialog_address_segment() -> Check {
        let mut ui = runtime()?;
        let controller = DialogController::new(&ui);
        let root = ui.mount_tree(
            Dialog::new(
                controller.clone(),
                Button::new("Open", |_| Ok(())).key("anchor"),
                Button::new("Confirm", |_| Ok(())).key("content"),
            )
            .key("owner"),
        )?;
        Driver::new(&mut ui).settle()?;
        controller.open()?;
        Driver::new(&mut ui).settle()?;
        assert!(ui.find(&root, &["content"])?.is_none());
        assert!(ui.find(&root, &["dialog", "content"])?.is_some());
        let addresses = Driver::new(&mut ui).address_children(&root)?;
        println!("P4 dialog addresses: {addresses:?}");
        assert!(addresses.iter().any(|child| child.segment == "dialog"));
        Ok(())
    }

    #[test]
    fn p4_alert_dialog_uses_the_same_dialog_segment() -> Check {
        let mut ui = runtime()?;
        let controller = DialogController::new(&ui);
        let root = ui.mount_tree(
            AlertDialog::new(
                controller.clone(),
                Button::new("Open", |_| Ok(())).key("anchor"),
                "Confirm",
                "Continue?",
                "Continue",
            )
            .key("owner"),
        )?;
        Driver::new(&mut ui).settle()?;
        controller.open()?;
        Driver::new(&mut ui).settle()?;
        assert!(ui.find(&root, &["confirmation"])?.is_none());
        assert!(ui.find(&root, &["dialog", "confirmation"])?.is_some());
        Ok(())
    }

    #[test]
    fn p6_select_semantics_need_logical_discovery_on_the_frozen_head() -> Check {
        let mut ui = runtime()?;
        let value = ui.signal(1_u8);
        let select = Select::new(&ui, &value, [Choice::new(1, "One"), Choice::new(2, "Two")])?;
        let root = ui.mount_tree(Column::new([select.key("format")]).key("root"))?;
        Driver::new(&mut ui).settle()?;
        let owner = ui.find(&root, &["format"])?.ok_or("missing Select")?;
        assert!(matches!(
            Driver::new(&mut ui).semantics(&owner),
            Err(TestError::NoSemantics { .. })
        ));
        assert!(Driver::new(&mut ui).address_children(&owner)?.is_empty());
        assert!(ui.find(&owner, &["control"])?.is_none());
        let control = descendants(&ui, &owner, "select");
        let semantics = Driver::new(&mut ui).semantics(&control)?;
        assert_eq!(semantics.role, SemanticRole::ComboBox);
        assert!(matches!(
            semantics.value,
            Some(SemanticValue::Choice {
                selected: Some(0),
                ..
            })
        ));
        Ok(())
    }
}
