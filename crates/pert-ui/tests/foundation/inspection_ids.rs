//! `Runtime::inspection_id` names a mounted component's snapshot entry.
use pert_ui::*;

fn entry_key(ui: &Runtime, id: u64) -> Option<String> {
    ui.inspect()
        .components
        .iter()
        .find(|c| c.id == id)
        .map(|c| c.key.clone())
}

#[test]
fn inspection_ids_name_one_incarnation() -> Result<(), Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    let shown = ui.signal(true);
    // Two descendants share a key; the id tells their entries apart.
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([
            Element::new("left", Row::new([Element::new("apply", Label::new("A"))])),
            Element::new(
                "right",
                Show::new(&shown, |_| {
                    Element::new("row", Row::new([Element::new("apply", Label::new("B"))]))
                }),
            ),
        ]),
    ))?;
    ui.flush(std::time::Duration::ZERO)?;
    let right = ui.find_descendant(&root, "right")?.ok_or("right")?;
    let apply = ui.find_descendant(&right, "apply")?.ok_or("apply")?;
    let id = ui.inspection_id(&apply)?;
    let snapshot = ui.inspect();
    let entry = snapshot
        .components
        .iter()
        .find(|c| c.id == id)
        .ok_or("entry")?;
    assert_eq!(entry.key, "apply");
    assert_eq!(
        entry.details.properties["text"],
        InspectValue::Text("B".into())
    );

    // Unmounting ends the id; mounting again under the same key gives a new one.
    shown.set(false)?;
    ui.flush(std::time::Duration::ZERO)?;
    assert_eq!(ui.inspection_id(&apply), Err(RuntimeError::Unmounted));
    assert_eq!(entry_key(&ui, id), None);
    shown.set(true)?;
    ui.flush(std::time::Duration::ZERO)?;
    let again = ui.find_descendant(&right, "apply")?.ok_or("apply")?;
    let new = ui.inspection_id(&again)?;
    assert_ne!(new, id);
    assert_eq!(entry_key(&ui, new).as_deref(), Some("apply"));

    // An id of another runtime is refused.
    let other = Runtime::new();
    assert_eq!(
        other.inspection_id(&again),
        Err(RuntimeError::ForeignRuntime)
    );
    Ok(())
}

/// Panics once mounted, after it took an id, so its mount is rolled back.
struct Panics;
impl Component for Panics {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn mounted(&mut self) {
        panic!("rolled back");
    }
}

#[test]
fn a_rolled_back_mount_does_not_reuse_inspection_ids() -> Result<(), Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    let first = ui.mount_tree(Element::new("first", Label::new("1")))?;
    let first = ui.inspection_id(&first)?;
    let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        ui.mount_tree(Element::new(
            "failed",
            Column::new([
                Element::new("ok", Label::new("ok")),
                Element::new("panics", Panics),
            ]),
        ))
    }));
    assert!(failed.is_err());
    let ids: Vec<u64> = ui.inspect().components.iter().map(|c| c.id).collect();
    assert_eq!(ids, vec![first]);
    let second = ui.mount_tree(Element::new("second", Label::new("2")))?;
    let second = ui.inspection_id(&second)?;
    // The failed tree's root took the id after `first`; it is not given out again.
    assert!(second > first + 1, "{second} after {first}");
    assert_eq!(entry_key(&ui, second).as_deref(), Some("second"));
    Ok(())
}
