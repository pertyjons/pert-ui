use pert_ui::AppError;

fn main() -> Result<(), AppError> {
    job_monitor::app(job_monitor::Environment::real(), |_| {}).run()
}
