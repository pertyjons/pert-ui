use notes_editor::{app, ctrl};
use pert_ui::{prelude::*, testing::*};
use std::{fs, path::PathBuf};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
fn find(d: &Driver<'_>, root: &WidgetId, key: &str) -> WidgetId {
    d.runtime()
        .find_descendant(root, key)
        .unwrap()
        .unwrap_or_else(|| panic!("missing {key}"))
}
fn click(d: &mut Driver<'_>, root: &WidgetId, key: &str) -> Result {
    let id = find(d, root, key);
    d.click(&id)?;
    Ok(())
}
fn named(d: &Driver<'_>, root: &WidgetId, key: &str) -> String {
    d.semantics(&find(d, root, key))
        .unwrap()
        .name
        .unwrap_or_default()
}
fn editor(d: &Driver<'_>, tabs: &WidgetId, name: &str) -> WidgetId {
    let tab = d.find(tabs, &[name]).unwrap();
    find(d, &tab, "editor")
}
fn titles(d: &Driver<'_>, tabs: &WidgetId) -> Vec<String> {
    d.semantics(tabs)
        .unwrap()
        .items
        .into_iter()
        .filter_map(|i| i.name)
        .collect()
}
fn chord(d: &mut Driver<'_>, c: char) -> Result {
    let code = match c {
        'n' => KeyCode::KeyN,
        's' => KeyCode::KeyS,
        'w' => KeyCode::KeyW,
        _ => unreachable!(),
    };
    d.chord(ctrl(c), code)?;
    Ok(())
}
#[test]
fn acceptance() -> Result {
    let folder = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/headless-notes");
    if folder.exists() {
        fs::remove_dir_all(&folder)?;
    }
    fs::create_dir_all(&folder)?;
    fs::write(folder.join("Beta.txt"), "second note")?;
    fs::write(folder.join("Alpha.txt"), "first note")?;
    fs::write(folder.join("ignore.md"), "not a note")?;
    let mut app = app(folder.clone()).headless()?;
    let root = app.state().root.clone();
    let mut d = Driver::new(app.runtime_mut());
    d.settle()?;
    let list = find(&d, &root, "files");
    let tabs = find(&d, &root, "tabs");
    let rows = d.address_children(&list)?;
    assert_eq!(
        rows.iter().map(|r| r.segment.as_str()).collect::<Vec<_>>(),
        ["Alpha.txt", "Beta.txt"]
    );
    let button = find(&d, &rows[0].id, "open");
    d.click(&button)?;
    assert_eq!(titles(&d, &tabs), ["Alpha.txt"]);
    d.runtime_mut().focus(&list)?;
    d.key(KeyCode::Home)?;
    d.key(KeyCode::ArrowDown)?;
    d.key(KeyCode::Enter)?;
    assert_eq!(titles(&d, &tabs), ["Alpha.txt", "Beta.txt"]);
    let alpha = d.find(&list, &["Alpha.txt"])?;
    click(&mut d, &alpha, "open")?;
    assert_eq!(titles(&d, &tabs).len(), 2);
    assert!(
        d.semantics(&tabs)?.items[0].selected,
        "reopening selects the existing tab"
    );
    let edit = editor(&d, &tabs, "Alpha.txt");
    d.replace_text(&edit, "one two\nthree")?;
    assert_eq!(titles(&d, &tabs), ["Alpha.txt *", "Beta.txt"]);
    let alpha_tab = d.find(&tabs, &["Alpha.txt"])?;
    assert_eq!(
        named(&d, &alpha_tab, "status"),
        "Alpha.txt: 2 lines, 3 words"
    );
    chord(&mut d, 's')?;
    assert_eq!(
        fs::read_to_string(folder.join("Alpha.txt"))?,
        "one two\nthree"
    );
    assert_eq!(titles(&d, &tabs), ["Alpha.txt", "Beta.txt"]);
    d.replace_text(&edit, "saved through menu")?;
    // F10 reaches File; Down opens it, Down selects Save after New note.
    d.key(KeyCode::F10)?;
    d.key(KeyCode::ArrowDown)?;
    d.key(KeyCode::ArrowDown)?;
    d.key(KeyCode::Enter)?;
    assert_eq!(
        fs::read_to_string(folder.join("Alpha.txt"))?,
        "saved through menu"
    );
    assert_eq!(titles(&d, &tabs), ["Alpha.txt", "Beta.txt"]);
    d.replace_text(&edit, "An edited note\nwith a second line.")?;
    // Required CPU capture: two open tabs, one modified.
    d.runtime().set_capture_support(true);
    let pending = d
        .runtime()
        .request_capture(CaptureRequest::new(CaptureTarget::window(
            &d.runtime().primary_window(),
        )));
    d.settle()?;
    CpuRenderer::new().serve_captures(d.runtime_mut());
    let capture = pending.try_reply()?.value?;
    fs::write(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("notes.png"),
        capture.image.encode_png(),
    )?;
    // New note validation stays in the dialog and leaves the directory unchanged.
    chord(&mut d, 'n')?;
    let form = find(&d, &root, "new-form");
    for (name, explanation) in [
        ("", "1 to 64"),
        ("bad/name", "must not contain"),
        ("Alpha", "already exists"),
        (".hidden", "must not start"),
        (&"a".repeat(65), "1 to 64"),
    ] {
        let field = find(&d, &form, "name");
        d.replace_text(&field, name)?;
        click(&mut d, &form, "create")?;
        assert!(named(&d, &form, "error").contains(explanation));
        assert_eq!(fs::read_dir(&folder)?.count(), 3);
    }
    let field = find(&d, &form, "name");
    d.replace_text(&field, "Gamma")?;
    click(&mut d, &form, "create")?;
    assert_eq!(fs::read_to_string(folder.join("Gamma.txt"))?, "");
    assert_eq!(
        d.address_children(&list)?
            .iter()
            .map(|r| r.segment.as_str())
            .collect::<Vec<_>>(),
        ["Alpha.txt", "Beta.txt", "Gamma.txt"]
    );
    assert_eq!(titles(&d, &tabs), ["Alpha.txt *", "Beta.txt", "Gamma.txt"]);
    // Unmodified close has no modal; the next editing action succeeds.
    chord(&mut d, 'w')?;
    assert_eq!(titles(&d, &tabs), ["Alpha.txt *", "Beta.txt"]);
    // Select Alpha via its actual title.
    d.runtime_mut().focus(&tabs)?;
    d.key(KeyCode::Home)?;
    chord(&mut d, 'w')?;
    let close_form = find(&d, &root, "close-form");
    click(&mut d, &close_form, "cancel")?;
    assert_eq!(titles(&d, &tabs), ["Alpha.txt *", "Beta.txt"]);
    assert_eq!(
        d.semantics(&edit)?.value,
        Some(SemanticValue::Text(
            "An edited note\nwith a second line.".into()
        ))
    );
    chord(&mut d, 'w')?;
    click(&mut d, &close_form, "discard")?;
    assert_eq!(titles(&d, &tabs), ["Beta.txt"]);
    assert_eq!(
        fs::read_to_string(folder.join("Alpha.txt"))?,
        "saved through menu"
    );
    let beta_edit = editor(&d, &tabs, "Beta.txt");
    d.replace_text(&beta_edit, "save on close")?;
    chord(&mut d, 'w')?;
    click(&mut d, &close_form, "save")?;
    assert!(titles(&d, &tabs).is_empty());
    assert!(
        d.runtime().find_descendant(&tabs, "status")?.is_none(),
        "no status without an open tab"
    );
    assert_eq!(
        fs::read_to_string(folder.join("Beta.txt"))?,
        "save on close"
    );
    // Reopen, fail a save and fail Save-and-close; preserve text and marker.
    let beta = d.find(&list, &["Beta.txt"])?;
    click(&mut d, &beta, "open")?;
    let beta_edit = editor(&d, &tabs, "Beta.txt");
    d.replace_text(&beta_edit, "cannot save")?;
    let moved = folder.with_extension("moved");
    if moved.exists() {
        fs::remove_dir_all(&moved)?;
    }
    fs::rename(&folder, &moved)?;
    chord(&mut d, 's')?;
    assert!(named(&d, &root, "notification").contains("Save failed"));
    assert_eq!(titles(&d, &tabs), ["Beta.txt *"]);
    chord(&mut d, 'w')?;
    click(&mut d, &close_form, "save")?;
    assert_eq!(titles(&d, &tabs), ["Beta.txt *"]);
    click(&mut d, &close_form, "cancel")?;
    assert_eq!(
        d.semantics(&beta_edit)?.value,
        Some(SemanticValue::Text("cannot save".into()))
    );
    fs::rename(&moved, &folder)?;
    Ok(())
}
