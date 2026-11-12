
// ---------------------------------------------------------------------------
// Phase 1 probe adapter, appended by plans/rendering/phase1-probe/run.sh to a
// scratch copy of the gallery's `pages/layout/transforms.rs`, so it can reach
// the private `card`; `pages/layout.rs` gets a re-export. The repository's
// gallery does not change.

/// The canvas of the gallery's `--zoom-benchmark`: sixty-four cards of four
/// controls each in a `ZoomView`, keyed `canvas`.
pub(crate) fn probe_zoom_canvas(ui: &Runtime) -> Result<Element, Box<dyn std::error::Error>> {
    const COLUMNS: usize = 8;
    const ROWS: usize = 8;
    let status = ui.signal(String::new());
    let mut cards = Vec::new();
    for row in 0..ROWS {
        for column in 0..COLUMNS {
            let index = row * COLUMNS + column;
            let origin = Point::new(40.0 + 280.0 * column as f32, 40.0 + 200.0 * row as f32)?;
            let title = format!("Module {index}");
            cards.push((origin, card(ui, &format!("card-{index}"), &title, &status)));
        }
    }
    Ok(Element::new("canvas", ZoomView::new(cards)))
}
