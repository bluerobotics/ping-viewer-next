use std::str::FromStr;

use crate::cli;

use tracing::{metadata::LevelFilter, *};
use tracing_log::LogTracer;
use tracing_subscriber::{fmt, layer::SubscriberExt, EnvFilter, Layer};

// Start logger, should be done inside main
pub fn init() {
    // Redirect all logs from libs using "Log"
    LogTracer::init_with_filter(tracing::log::LevelFilter::Trace).expect("Failed to set logger");

    let level = std::env::var("RUST_LOG").unwrap_or_else(|_| {
        if cli::manager::is_verbose() {
            LevelFilter::DEBUG.to_string()
        } else {
            LevelFilter::INFO.to_string()
        }
    });

    let console_env_filter = EnvFilter::from_str(&level).expect("logger : Invalid debugging value");

    let console_layer = fmt::Layer::new()
        .with_writer(std::io::stdout)
        .with_ansi(true)
        .with_file(true)
        .with_line_number(true)
        .with_span_events(fmt::format::FmtSpan::NONE)
        .with_target(false)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_filter(console_env_filter);

    // Configure the file log
    let file_env_filter = if cli::manager::is_tracing() {
        EnvFilter::new(LevelFilter::TRACE.to_string())
    } else {
        EnvFilter::new(LevelFilter::DEBUG.to_string())
    };

    let dir = cli::manager::log_path();
    let file_appender = tracing_appender::rolling::hourly(dir, "ping-viewer.", ".log");
    let file_layer = fmt::Layer::new()
        .with_writer(file_appender)
        .with_ansi(false)
        .with_file(true)
        .with_line_number(true)
        .with_span_events(fmt::format::FmtSpan::NONE)
        .with_target(false)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_filter(file_env_filter);

    let subscriber = tracing_subscriber::registry()
        .with(console_layer)
        .with(file_layer);

    // Configure the default subscriber
    match (
        cli::manager::is_tracy(),
        cli::manager::is_log_all_dependencies(),
    ) {
        (true, false) => {
            let lib_name = env!("CARGO_PKG_NAME").replace('-', "_");
            let subscriber = subscriber.with(EnvFilter::new(format!(
                "{lib_name}={level},lib{lib_name}={level}"
            )));
            let tracy_layer = tracing_tracy::TracyLayer::default();
            let subscriber = subscriber.with(tracy_layer);
            tracing::subscriber::set_global_default(subscriber)
                .expect("Unable to set a global subscriber");
        }
        (false, false) => {
            let lib_name = env!("CARGO_PKG_NAME").replace('-', "_");
            let subscriber = subscriber.with(EnvFilter::new(format!(
                "{lib_name}={level},lib{lib_name}={level}"
            )));
            tracing::subscriber::set_global_default(subscriber)
                .expect("Unable to set a global subscriber");
        }
        (true, true) => {
            let subscriber = subscriber.with(EnvFilter::new(level.to_string()));
            let tracy_layer = tracing_tracy::TracyLayer::default();
            let subscriber = subscriber.with(tracy_layer);
            tracing::subscriber::set_global_default(subscriber)
                .expect("Unable to set a global subscriber");
        }
        (false, true) => {
            let subscriber = subscriber.with(EnvFilter::new(level.to_string()));
            tracing::subscriber::set_global_default(subscriber)
                .expect("Unable to set a global subscriber");
        }
    };

    info!(
        "{}, version: {}-{}, build date: {}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        env!("VERGEN_GIT_SHA"),
        env!("VERGEN_BUILD_DATE")
    );

    info!(
        "Build dependencies details: {}",
        env!("VERGEN_CARGO_DEPENDENCIES"),
    );

    info!(
        "Starting at {}",
        chrono::Local::now().format("%Y-%m-%dT%H:%M:%S"),
    );
    debug!("Command line call: {}", cli::manager::command_line_string());
    debug!(
        "Command line input struct call: {}",
        cli::manager::command_line()
    );
}
