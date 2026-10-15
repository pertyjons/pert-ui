//! Retained MultiSelect option-source replacement through the public facade.
use pert_ui::*;
mod support;
use std::time::Duration;
use support::{find, press};
#[test]
fn replaced_option_source_reaches_an_open_multi_select_list()
-> Result<(), Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0)?)?;
    let value = ui.signal(Vec::<i32>::new());
    let old = ui.signal(vec![Choice::new(1, "Old")]);
    let next = ui.signal(vec![Choice::new(2, "New")]);
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([Element::new(
            "choice",
            MultiSelect::bound(&ui, value.clone(), old.read_only()),
        )]),
    ))?;
    ui.flush(Duration::ZERO)?;
    ui.focus(&find(&ui, &root, "anchor").unwrap())?;
    press(&mut ui, KeyCode::Enter);
    ui.reconcile_children(
        &root,
        vec![Element::new(
            "choice",
            MultiSelect::bound(&ui, value.clone(), next.read_only()),
        )],
    )?;
    ui.flush(Duration::ZERO)?;
    let labels = |ui: &Runtime| {
        ui.inspect()
            .components
            .into_iter()
            .filter(|c| c.visible && c.details.properties.contains_key("checked"))
            .map(|c| c.details.semantics.unwrap().name.unwrap())
            .collect::<Vec<_>>()
    };
    assert_eq!(labels(&ui), vec!["New"]);
    for name in ["Intermediate", "Latest"] {
        let source = ui.signal(vec![Choice::new(3, name)]);
        ui.reconcile_children(
            &root,
            vec![Element::new(
                "choice",
                MultiSelect::bound(&ui, value.clone(), source.read_only()),
            )],
        )?;
    }
    ui.flush(Duration::ZERO)?;
    assert_eq!(labels(&ui), vec!["Latest"]);
    assert_eq!(ui.flush(Duration::ZERO)?.components_updated, 0);
    Ok(())
}
