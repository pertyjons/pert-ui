
// ---------------------------------------------------------------------------
// Phase 1 probe adapter, appended by plans/rendering/phase1-probe/run.sh to a
// scratch copy of the diagram editor's `main.rs` whose `main` was removed and
// whose `crate::` paths were rewritten to `crate::diagram::`. It lives in the
// example's root module so it can reach the private `app` and `editor`
// modules; the repository's example does not change.

/// The diagram editor's application, as `app::build` returns it.
pub(crate) struct ProbeDiagram {
    app: std::rc::Rc<app::DiagramApp>,
}

/// Builds the editor with its files in `dir`, which should be a fresh
/// directory.
pub(crate) fn probe_build(
    ui: &mut pert_ui::Runtime,
    dir: std::path::PathBuf,
) -> Result<ProbeDiagram, Box<dyn std::error::Error>> {
    Ok(ProbeDiagram {
        app: app::build(ui, dir)?,
    })
}

impl ProbeDiagram {
    /// The canvas component, whose `child_transform` pans and zooms.
    pub(crate) fn canvas(&self) -> pert_ui::WidgetId {
        self.app.canvas.clone()
    }

    /// Zooms by `factor` around `anchor` in the canvas's local pixels, as
    /// the editor's wheel and zoom commands do.
    pub(crate) fn zoom_by(&self, factor: f32, anchor: pert_ui::Point) -> Result<(), pert_ui::RuntimeError> {
        self.app.editor.zoom_by(factor, anchor)
    }

    /// Moves the view's offset by `dx`, `dy` local pixels at its scale.
    pub(crate) fn pan_by(&self, dx: f32, dy: f32) -> Result<(), Box<dyn std::error::Error>> {
        let view = self.app.editor.view.get()?;
        let offset = view.offset();
        let moved = pert_ui::Point::new(offset.x() + dx, offset.y() + dy)?;
        self.app
            .editor
            .view
            .set(pert_ui::ViewTransform::new(view.scale(), moved))?;
        Ok(())
    }

    /// The view's current scale.
    pub(crate) fn scale(&self) -> Result<f32, pert_ui::RuntimeError> {
        Ok(self.app.editor.view.get()?.scale().get())
    }
}
