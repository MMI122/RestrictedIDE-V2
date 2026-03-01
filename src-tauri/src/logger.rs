/// Initialize the application logger.
/// Uses env_logger with a default filter of "info" for release, "debug" for dev.
pub fn init() {
    let default_level = if cfg!(debug_assertions) { "debug" } else { "info" };

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(default_level))
        .format_timestamp_secs()
        .init();
}
