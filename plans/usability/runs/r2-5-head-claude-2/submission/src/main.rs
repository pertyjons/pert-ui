use job_monitor::{Deps, app};
use pert_ui::prelude::*;

fn main() -> Result<(), AppError> {
    let (app, _built) = app(Deps::real());
    app.size(Size::new(820.0, 480.0)?)?.run()
}
