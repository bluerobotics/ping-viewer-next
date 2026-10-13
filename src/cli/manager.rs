use clap;
use clap::Parser;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::OnceLock;
use std::{path::PathBuf, sync::LazyLock};

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

    /// Specifies the path in which the logs will be stored.
    #[arg(long, default_value = "./logs")]
    log_path: PathBuf,

    /// Specifies the path in which recordings will be stored.
    #[arg(long, default_value = "./recordings")]
    recordings_path: PathBuf,

    /// Specifies the path in which settings will be stored.
    #[arg(long, default_value = "./settings")]
    settings_path: PathBuf,

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

static MANAGER: LazyLock<Manager> = LazyLock::new(|| Manager::new());

impl Manager {
    fn new() -> Self {
        Self {
            clap_matches: Args::parse(),
        }
    }
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

pub fn is_reset() -> bool {
    MANAGER.clap_matches.reset
}

static BASE_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Sets the directory against which relative data paths (logs, recordings, settings) are resolved.
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

fn prepare_directory(path: PathBuf) -> std::io::Result<PathBuf> {
    let path = resolve_path(&path);
    std::fs::create_dir_all(&path)?;
    path.canonicalize()
}

pub fn log_directory() -> std::io::Result<PathBuf> {
    prepare_directory(MANAGER.clap_matches.log_path.clone())
}

pub fn recordings_directory() -> std::io::Result<PathBuf> {
    prepare_directory(MANAGER.clap_matches.recordings_path.clone())
}

pub fn settings_directory() -> std::io::Result<PathBuf> {
    prepare_directory(MANAGER.clap_matches.settings_path.clone())
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
