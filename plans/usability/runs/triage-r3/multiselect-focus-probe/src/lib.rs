#[cfg(test)]
mod tests {
    use pert_ui::*;
    use pert_ui::testing::Driver;
    type Check = Result<(), Box<dyn std::error::Error>>;
    fn focus(ui: &Runtime) -> String {
        ui.focused().map(|id| format!("{}#{}", ui.key(&id).unwrap(), ui.inspection_id(&id).unwrap())).unwrap_or_default()
    }
    fn highlight(ui: &Runtime) -> Vec<String> {
        ui.inspect().components.into_iter().filter(|c| c.visible && c.details.properties.get("highlighted")==Some(&InspectValue::Bool(true)))
            .filter_map(|c| c.details.semantics.and_then(|s| s.name)).collect()
    }
    #[test]
    fn click_then_keys_on_short_and_overflowing_lists() -> Check {
        for (count, remove_pointer) in [(6,false),(20,false),(6,true),(20,true)] {
            let mut ui=Runtime::new(); ui.set_viewport(Viewport::new(480.0,420.0,1.0)?)?;
            let value=ui.signal(Vec::<String>::new());let writes=ui.signal(Vec::<Vec<String>>::new());let report=writes.clone();
            let choices=(0..count).map(|n| Choice::new(format!("o{n:02}"),format!("O{n:02}")));
            let root=ui.mount_tree(Element::new("form",Column::new([
                Element::new("chooser",MultiSelect::new(&ui,&value,choices)?.on_change(move |_,chosen| report.update(|log|log.push(chosen.to_vec())))),
                Element::new("after",Button::new("After", |_| Ok(()))),
            ])))?;
            ui.flush(ui.now())?;
            let chooser=ui.find(&root,&["chooser"])?.ok_or("chooser")?;
            let trigger=ui.inspect().components.into_iter().find(|c|c.details.semantics.as_ref().is_some_and(|s|s.role==SemanticRole::ComboBox)).ok_or("ComboBox")?;
            let trigger=ui.find_descendant(&chooser,&trigger.key)?.ok_or("trigger")?;
            ui.focus(&trigger)?;Driver::new(&mut ui).key(KeyCode::ArrowDown)?;
            let search=Driver::new(&mut ui).exposed_part(&chooser,"search")?;
            let row=ui.inspect().components.into_iter().find(|c|c.visible && c.details.properties.get("text")==Some(&InspectValue::Text("O02".into())) && c.details.properties.contains_key("checked")).ok_or("row")?;
            let row=ui.find_descendant(&root,&row.key)?.ok_or("mounted row")?;
            println!("FOCUS|count={count} opened focus={} highlight={:?}",focus(&ui),highlight(&ui));
            Driver::new(&mut ui).click(&row)?;
            println!("FOCUS|count={count} clicked focus={} highlight={:?} value={:?} writes={:?}",focus(&ui),highlight(&ui),value.get()?,writes.get()?);
            if remove_pointer {
                ui.pointer_input(PointerEvent::new(Point::new(0.0,0.0)?,PointerKind::Leave))?;
            }
            println!("FOCUS|control pointer_left={remove_pointer} count={count}");
            Driver::new(&mut ui).key(KeyCode::ArrowDown)?;
            println!("FOCUS|count={count} Down focus={} highlight={:?}",focus(&ui),highlight(&ui));
            Driver::new(&mut ui).key(KeyCode::Enter)?;
            let before=Driver::new(&mut ui).semantics(&search)?.value;
            let typed=ui.text_input("z");ui.flush(ui.now())?;
            let after=Driver::new(&mut ui).semantics(&search)?.value;
            println!("FOCUS|count={count} Enter+type focus={} highlight={:?} value={:?} writes={:?} text_result={typed:?} search_before={before:?} search_after={after:?}",focus(&ui),highlight(&ui),value.get()?,writes.get()?);
        }
        Ok(())
    }
}
