use clap;
use clap::Parser;
use lazy_static::lazy_static;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

#[derive(Parser, Debug)]
#[command(version, author, about)]
struct Args {
    /// Call AutoCreate on DeviceManager during application startup.
    #[arg(long)]
    enable_auto_create: bool,

    /// Deletes settings file before starting.
    #[arg(long)]
    reset: bool,

    /// Sets the address for the REST API server
    #[arg(long, value_name = "IP>:<PORT", default_value = "0.0.0.0:4936")]
    rest_server: SocketAddr,

    /// Sets the address of the Zenoh router used to receive vehicle data
    #[arg(long, value_name = "IP>:<PORT", default_value = "127.0.0.1:7447")]
    zenoh_server: SocketAddr,

    /// Turns all log categories up to Debug, for more information check RUST_LOG env variable.
    #[arg(short, long)]
    verbose: bool,

    /// Specifies the path in witch the logs will be stored.
    #[arg(long, default_value = "./logs")]
    log_path: PathBuf,

    /// Turns all log categories up to Trace to the log file, for more information check RUST_LOG env variable.
    #[arg(long)]
    enable_tracing_level_log_file: bool,

    /// Filter to show only own crate related logs
    #[arg(long)]
    log_include_all_dependencies: bool,

    /// Turns on the Tracy tool integration.
    #[arg(long)]
    enable_tracy: bool,

    /// Turns on the debug mode.
    #[arg(long)]
    debug: bool,
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

pub fn is_debug() -> bool {
    MANAGER.clap_matches.debug
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

pub fn is_log_all_dependencies() -> bool {
    MANAGER.clap_matches.log_include_all_dependencies
}

pub fn is_enable_auto_create() -> bool {
    MANAGER.clap_matches.enable_auto_create
}

static BASE_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Sets the directory against which relative data paths (logs, recordings) are resolved.
pub fn set_base_dir(base_dir: impl Into<PathBuf>) {
    BASE_DIR
        .set(base_dir.into())
        .expect("base directory should only be set once");
}

fn resolve_path(path: impl AsRef<Path>) -> PathBuf {
    match BASE_DIR.get() {
        Some(base_dir) => base_dir.join(path),
        None => path.as_ref().to_path_buf(),
    }
}

pub fn log_path() -> PathBuf {
    let log_path = resolve_path(&MANAGER.clap_matches.log_path);
    std::fs::create_dir_all(&log_path).expect("Failed to create log path");
    log_path
        .canonicalize()
        .expect("Failed to canonicalize log path")
}

pub fn recordings_path() -> PathBuf {
    resolve_path("recordings")
}

// Return the desired address for the REST API
pub fn server_address() -> SocketAddr {
    MANAGER.clap_matches.rest_server.clone()
}

// Return the desired address for the Zenoh router
pub fn zenoh_server_address() -> SocketAddr {
    MANAGER.clap_matches.zenoh_server.clone()
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
