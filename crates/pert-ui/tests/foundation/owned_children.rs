//! Public lifecycle admission for components retaining constructor-generated parts.
use pert_ui::*;

struct Owner {
    children: Vec<Element>,
    retain: bool,
}
impl Owner {
    fn new(children: Vec<Element>, retain: bool) -> Self {
        Self { children, retain }
    }
}
impl Component for Owner {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn retain_initial_children(&self) -> bool {
        self.retain
    }
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
}

struct Refused;
impl Component for Refused {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn check_description(&self) -> Result<(), RuntimeError> {
        Err(RuntimeError::Exhausted)
    }
}

#[test]
fn compatible_owned_descriptions_preserve_bindings_and_metadata_before_first_update() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0).unwrap())
        .unwrap();
    let binding = ui.signal("original".to_owned());
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([Element::new(
                "owner",
                Owner::new(
                    vec![
                        Element::new("field", TextField::new(binding.clone()))
                            .with_label("Original label"),
                    ],
                    true,
                ),
            )]),
        ))
        .unwrap();
    let owner = ui.find_descendant(&root, "owner").unwrap().unwrap();
    let field = ui.find_descendant(&owner, "field").unwrap().unwrap();
    ui.set_visible(&owner, false).unwrap();
    // Neither a different child type, invalid children, nor a different incoming
    // policy may replace the mounted owner's children or their metadata.
    ui.reconcile_children(
        &root,
        vec![Element::new(
            "owner",
            Owner::new(
                vec![
                    Element::new("field", Refused).with_label("Replacement label"),
                    Element::new("field", Label::new("duplicate")),
                ],
                false,
            ),
        )],
    )
    .unwrap();
    assert_eq!(
        ui.children(&owner).unwrap().as_slice(),
        std::slice::from_ref(&field)
    );
    let component = ui
        .inspect()
        .components
        .into_iter()
        .find(|c| c.key == "field")
        .unwrap();
    assert_eq!(
        component.details.semantics.unwrap().name.as_deref(),
        Some("Original label")
    );
    ui.set_visible(&owner, true).unwrap();
    ui.flush(std::time::Duration::ZERO).unwrap();
    ui.focus(&field).unwrap();
    ui.text_input("!").unwrap();
    assert_eq!(binding.get().unwrap(), "!original");
}

#[test]
fn new_owned_components_receive_initial_children_and_new_keys_replace_ownership() {
    let mut ui = Runtime::new();
    let root = ui
        .mount_tree(Element::new("root", Column::new([])))
        .unwrap();
    ui.reconcile_children(
        &root,
        vec![Element::new(
            "first",
            Owner::new(vec![Element::new("part", Label::new("First"))], true),
        )],
    )
    .unwrap();
    let first = ui.find_descendant(&root, "first").unwrap().unwrap();
    let part = ui.find_descendant(&first, "part").unwrap().unwrap();
    ui.reconcile_children(
        &root,
        vec![Element::new(
            "second",
            Owner::new(vec![Element::new("part", Label::new("Second"))], true),
        )],
    )
    .unwrap();
    let second = ui.find_descendant(&root, "second").unwrap().unwrap();
    let next = ui.find_descendant(&second, "part").unwrap().unwrap();
    assert_ne!(next, part);
    assert_eq!(ui.key(&part), Err(RuntimeError::Unmounted));
    // An explicit owner operation is still permitted.
    ui.reconcile_children(
        &second,
        vec![Element::new("replacement", Label::new("Direct"))],
    )
    .unwrap();
    let replacement = ui.find_descendant(&second, "replacement").unwrap().unwrap();
    ui.reconcile_children(
        &root,
        vec![Element::new("second", Owner::new(vec![], true))],
    )
    .unwrap();
    assert_eq!(ui.children(&second).unwrap(), [replacement]);
}

#[test]
fn initial_owned_children_and_ordinary_descriptions_still_require_admission() {
    let mut ui = Runtime::new();
    assert_eq!(
        ui.mount_tree(Element::new(
            "bad",
            Owner::new(vec![Element::new("child", Refused),], true)
        )),
        Err(RuntimeError::Exhausted)
    );
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([Element::new(
                "owner",
                Owner::new(vec![Element::new("part", Label::new("First"))], false),
            )]),
        ))
        .unwrap();
    let owner = ui.find_descendant(&root, "owner").unwrap().unwrap();
    let before = ui.children(&owner).unwrap();
    assert_eq!(
        ui.reconcile_children(
            &root,
            vec![Element::new(
                "owner",
                Owner::new(vec![Element::new("part", Refused),], false)
            )]
        ),
        Err(RuntimeError::Exhausted)
    );
    assert_eq!(ui.children(&owner).unwrap(), before);
    assert_eq!(
        ui.reconcile_children(&root, vec![Element::new("owner", Column::new([]))]),
        Err(RuntimeError::ChildTypeChanged("owner".into()))
    );
    assert_eq!(ui.children(&owner).unwrap(), before);
}

#[test]
fn nested_owned_parts_mount_under_hidden_parents_and_survive_recursive_redescription() {
    let mut ui = Runtime::new();
    let root = ui
        .mount_tree(Element::new("root", Column::new([])))
        .unwrap();
    ui.set_visible(&root, false).unwrap();
    ui.reconcile_children(
        &root,
        vec![Element::new(
            "container",
            Column::new([Element::new(
                "owner",
                Owner::new(vec![Element::new("part", Label::new("Original"))], true),
            )]),
        )],
    )
    .unwrap();
    let owner = ui.find_descendant(&root, "owner").unwrap().unwrap();
    let part = ui.find_descendant(&owner, "part").unwrap().unwrap();
    assert!(
        ui.inspect()
            .components
            .iter()
            .filter(|c| c.key == "owner" || c.key == "part")
            .all(|c| !c.visible)
    );
    ui.reconcile_children(
        &root,
        vec![Element::new(
            "container",
            Column::new([Element::new(
                "owner",
                Owner::new(vec![Element::new("part", Refused)], true),
            )]),
        )],
    )
    .unwrap();
    assert_eq!(
        ui.find_descendant(&root, "part").unwrap(),
        Some(part.clone())
    );
    assert_eq!(
        ui.reconcile_children(
            &root,
            vec![Element::new(
                "container",
                Column::new([Element::new("owner", Column::new([])),])
            )]
        ),
        Err(RuntimeError::ChildTypeChanged("owner".into()))
    );
    assert_eq!(ui.find_descendant(&root, "part").unwrap(), Some(part));
}
