use clap;
use clap::Parser;
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(version = env!("CARGO_PKG_VERSION"), author = env!("CARGO_PKG_AUTHORS"), about = env!("CARGO_PKG_DESCRIPTION"))]
struct Args {
    /// Call AutoCreate on DeviceManager during application startup.
    #[arg(long, default_value = "false")]
    enable_auto_create: bool,

    /// Deletes settings file before starting.
    #[arg(long)]
    reset: bool,

    /// Sets the address for the REST API server
    #[arg(long, value_name = "IP>:<PORT", default_value = "0.0.0.0:8080")]
    rest_server: String,

    /// Turns all log categories up to Debug, for more information check RUST_LOG env variable.
    #[arg(short, long)]
    verbose: bool,

    /// Specifies the path in witch the logs will be stored.
    #[arg(long, default_value = "./logs")]
    log_path: Option<String>,

    /// Turns all log categories up to Trace to the log file, for more information check RUST_LOG env variable.
    #[arg(long)]
    enable_tracing_level_log_file: bool,

    /// Filter to show only own crate related logs
    #[arg(long, default_value = "true")]
    log_current_crate_only: bool,

    /// Turns on the Tracy tool integration.
    #[arg(long)]
    enable_tracy: bool,
}

#[derive(Debug)]
struct Manager {
    clap_matches: Args,
}

lazy_static! {
    static ref MANAGER: Arc<Manager> = Arc::new(Manager::new());
}

impl Manager {
    fn new() -> Self {
        Self {
            clap_matches: Args::parse(),
        }
    }
}

// Construct our manager, should be done inside main
pub fn init() {
    MANAGER.as_ref();
}

// Check if the verbosity parameter was used
pub fn is_verbose() -> bool {
    MANAGER.clap_matches.verbose
}

pub fn is_tracing() -> bool {
    MANAGER.clap_matches.enable_tracing_level_log_file
}

pub fn is_tracy() -> bool {
    MANAGER.clap_matches.enable_tracy
}

pub fn log_current_crate_only() -> bool {
    MANAGER.clap_matches.log_current_crate_only
}

pub fn is_enable_auto_create() -> bool {
    MANAGER.clap_matches.enable_auto_create
}

pub fn log_path() -> String {
    let log_path =
        MANAGER.clap_matches.log_path.clone().expect(
            "Clap arg \"log-path\" should always be \"Some(_)\" because of the default value.",
        );

    shellexpand::full(&log_path)
        .expect("Failed to expand path")
        .to_string()
}

// Return the desired address for the REST API
pub fn server_address() -> String {
    MANAGER.clap_matches.rest_server.clone()
}

// Return the command line used to start this application
pub fn command_line_string() -> String {
    std::env::args().collect::<Vec<String>>().join(" ")
}

// Return a clone of current Args struct
pub fn command_line() -> String {
    format!("{:#?}", MANAGER.clap_matches)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_arguments() {
        assert!(!is_verbose());
    }
}
