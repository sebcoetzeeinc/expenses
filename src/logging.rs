use tracing::Level;
use tracing_subscriber::{
    filter::Targets,
    fmt::{
        self,
        format::{Format, Full},
        time::SystemTime,
    },
    prelude::*,
};

fn build_base_log_format() -> Format<Full, SystemTime> {
    return fmt::format()
        .with_level(true)
        .with_ansi(false)
        .with_file(true)
        .with_target(true)
        .with_thread_names(true);
}

pub fn setup_logging(base_log_dir: &str) {
    let stdout_layer =
        tracing_subscriber::fmt::layer().event_format(build_base_log_format().with_ansi(true));

    let filter = Targets::new()
        .with_target("sqlx", Level::INFO)
        .with_target("hyper_util", Level::INFO)
        .with_target("reqwest", Level::INFO)
        .with_default(Level::DEBUG);

    let subscriber = tracing_subscriber::registry()
        .with(filter)
        .with(stdout_layer);

    if base_log_dir != "" {
        let log_file_layer = tracing_subscriber::fmt::layer()
            .event_format(build_base_log_format())
            .with_writer(tracing_appender::rolling::daily(
                base_log_dir,
                "expenses.log",
            ));
        let json_file_layer = tracing_subscriber::fmt::layer()
            .event_format(build_base_log_format().json())
            .with_writer(tracing_appender::rolling::daily(
                format!("{}/structured", base_log_dir),
                "expenses.log",
            ));
        subscriber.with(log_file_layer).with(json_file_layer).init();
    } else {
        subscriber.init();
    }
}
