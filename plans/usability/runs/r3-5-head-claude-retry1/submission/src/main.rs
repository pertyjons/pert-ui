fn main() -> Result<(), pert_ui::AppError> {
    job_monitor::app(job_monitor::Env::real()).run()
}
