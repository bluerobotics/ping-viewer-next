fn resolve_stm32flash_path() -> Result<String, FirmwareUpdateError> {
    if let Ok(path) = which::which("stm32flash") {
        return Ok(path.to_string_lossy().to_string());
    }
    let fallback = std::env::current_dir()
        .map(|p| p.join("utils").join("stm32flash"))
        .map_err(|e| FirmwareUpdateError::Process(format!("cwd error: {e}")))?;
    if fallback.exists() {
        return Ok(fallback.to_string_lossy().to_string());
    }
    Err(FirmwareUpdateError::MissingTool)
}

fn resolve_ping360_bootloader_path() -> Result<String, FirmwareUpdateError> {
    if let Ok(path) = which::which("ping360-bootloader") {
        return Ok(path.to_string_lossy().to_string());
    }
    let fallback = std::env::current_dir()
        .map(|p| p.join("utils").join("ping360-bootloader"))
        .map_err(|e| FirmwareUpdateError::Process(format!("cwd error: {e}")))?;
    if fallback.exists() {
        return Ok(fallback.to_string_lossy().to_string());
    }
    Err(FirmwareUpdateError::MissingTool)
}
use paperclip::actix::Apiv2Schema;
use serde::{Deserialize, Serialize};

use crate::device::manager::SourceSelection;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone, Apiv2Schema)]
pub struct FirmwareUpdateRequest {
    pub mode: FirmwareUpdateMode,
    pub force: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, Apiv2Schema)]
pub enum FirmwareUpdateMode {
    AutoUpdate(crate::device::manager::UuidWrapper),
    ManualUpdate(ManualUpdate),
}
#[derive(Debug, Serialize, Deserialize, Clone, Apiv2Schema)]
pub struct ManualUpdate {
    pub path: String,
    pub device_kind: FirmwareDeviceKind,
}

#[derive(Debug, Serialize, Deserialize, Clone, Apiv2Schema)]
pub enum FirmwareDeviceKind {
    Ping1D,
    Ping2,
    Ping360,
}

#[derive(Debug, Serialize, Deserialize, Clone, Apiv2Schema)]
pub enum FirmwareUpdateResult {
    Started,
    SkippedAlreadyLatest,
    Running,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum FirmwareUpdateError {
    // #[error("stm32flash not found in PATH")]
    MissingTool,
    // #[error("invalid firmware path: {0}")]
    InvalidFirmwarePath(String),
    // #[error("unsupported device for this operation")]
    UnsupportedDevice,
    // #[error("io error: {0}")]
    Io(String),
    // #[error("process failed: {0}")]
    Process(String),
    // #[error("manager error: {0:?}")]
    // Manager(ManagerError),
}

impl From<std::io::Error> for FirmwareUpdateError {
    fn from(value: std::io::Error) -> Self {
        FirmwareUpdateError::Io(value.to_string())
    }
}

pub async fn update_ping360(
    source: &SourceSelection,
    firmware_path: Option<&str>,
    force: bool,
    device_id: Option<Uuid>,
) -> Result<FirmwareUpdateResult, FirmwareUpdateError> {
    let result = update_ping360_with_callback(source, firmware_path, force, device_id, None).await;
    match result {
        Ok((status, _)) => Ok(status),
        Err(e) => Err(e),
    }
}

pub async fn update_ping360_with_callback(
    source: &SourceSelection,
    firmware_path: Option<&str>,
    _force: bool,
    device_id: Option<Uuid>,
    on_completion: Option<Box<dyn FnOnce(bool) + Send + 'static>>,
) -> Result<(FirmwareUpdateResult, tokio::task::JoinHandle<()>), FirmwareUpdateError> {
    try_flash_ping360_with_options(
        source,
        firmware_path,
        device_id,
        Some(3),     // max_attempts
        Some(10000), // delay_between_attempts_ms (10 seconds)
        on_completion,
    )
    .await
}

pub async fn try_flash_ping360_with_options(
    source: &SourceSelection,
    firmware_path: Option<&str>,
    device_id: Option<Uuid>,
    max_attempts: Option<u8>,
    delay_between_attempts_ms: Option<u64>,
    on_completion: Option<Box<dyn FnOnce(bool) + Send + 'static>>,
) -> Result<(FirmwareUpdateResult, tokio::task::JoinHandle<()>), FirmwareUpdateError> {
    // Resolve ping360-bootloader path (PATH first, then ./utils/ping360-bootloader)
    let ping360_bootloader = resolve_ping360_bootloader_path()?;

    if let Some(path) = firmware_path {
        if !std::path::Path::new(path).exists() {
            return Err(FirmwareUpdateError::InvalidFirmwarePath(path.to_string()));
        }
    }

    let serial_path = match source {
        SourceSelection::SerialStream(s) => s.path.clone(),
        SourceSelection::UdpStream(_) => return Err(FirmwareUpdateError::UnsupportedDevice),
    };

    // Determine firmware path - use provided path or ensure default firmware is present
    let firmware_path_str = if let Some(path) = firmware_path {
        path.to_string()
    } else {
        let base = default_firmware_base_dir();
        ensure_ping360_firmware_present(&base)
            .await?
            .to_string_lossy()
            .to_string()
    };

    // Clone inputs to owned values for 'static lifetime inside spawned tasks
    let firmware_path_owned = firmware_path_str.to_string();
    let serial_path_owned = serial_path.to_string();

    let max_attempts_value: u8 = max_attempts.unwrap_or(3);
    let delay_ms_value: u64 = delay_between_attempts_ms.unwrap_or(10000);

    // Spawn ping360-bootloader in background with retry
    let handle = tokio::spawn(async move {
        let mut attempt: u8 = 1;

        loop {
            // Send 0% progress at the start of each attempt
            if attempt == 1 {
                broadcast_fw_progress(0.0, device_id);
            }

            let result = run_ping360_bootloader(
                ping360_bootloader.as_str(),
                &firmware_path_owned,
                &serial_path_owned,
                device_id,
            )
            .await;

            let should_retry = match result {
                Ok(_) => {
                    // Success - broadcast final 100% and call completion callback
                    broadcast_fw_progress(100.0, device_id);
                    if let Some(callback) = on_completion {
                        callback(true);
                    }
                    return;
                }
                Err(e) => {
                    // Check if this is a retryable error
                    // For Ping360, most errors are retryable (port busy, timeout, etc.)
                    let should_retry = attempt < max_attempts_value;

                    if should_retry {
                        broadcast_fw_error(
                            format!(
                                "Ping360 firmware update attempt {} failed, retrying in {}ms: {:?}",
                                attempt, delay_ms_value, e
                            ),
                            device_id,
                        );
                    } else {
                        broadcast_fw_error(
                            format!(
                                "Ping360 firmware update failed after {} attempts: {:?}",
                                max_attempts_value, e
                            ),
                            device_id,
                        );
                        if let Some(callback) = on_completion {
                            callback(false);
                        }
                        return;
                    }
                    true
                }
            };

            if should_retry && attempt < max_attempts_value {
                attempt += 1;
                tokio::time::sleep(Duration::from_millis(delay_ms_value)).await;
            } else {
                // Should not reach here, but just in case
                if let Some(callback) = on_completion {
                    callback(false);
                }
                return;
            }
        }
    });

    Ok((FirmwareUpdateResult::Started, handle))
}

pub async fn update_ping1d(
    source: &SourceSelection,
    firmware_path: Option<&str>,
    force: bool,
    device_id: Option<Uuid>,
) -> Result<FirmwareUpdateResult, FirmwareUpdateError> {
    let result = update_ping1d_with_callback(source, firmware_path, force, device_id, None).await;
    match result {
        Ok((status, _)) => Ok(status),
        Err(e) => Err(e),
    }
}

pub async fn update_ping1d_with_callback(
    source: &SourceSelection,
    firmware_path: Option<&str>,
    force: bool,
    device_id: Option<Uuid>,
    on_completion: Option<Box<dyn FnOnce(bool) + Send + 'static>>,
) -> Result<(FirmwareUpdateResult, tokio::task::JoinHandle<()>), FirmwareUpdateError> {
    // Resolve stm32flash binary (PATH first, then ./utils/stm32flash)
    let stm32flash_path = resolve_stm32flash_path()?;

    if let Some(path) = firmware_path {
        if !std::path::Path::new(path).exists() {
            return Err(FirmwareUpdateError::InvalidFirmwarePath(path.to_string()));
        }
    }

    let serial_path = match source {
        SourceSelection::SerialStream(s) => s.path.clone(),
        SourceSelection::UdpStream(_) => return Err(FirmwareUpdateError::UnsupportedDevice),
    };

    // If not forcing, attempt to query version and decide
    if !force {
        // We do not have version database here yet; keep placeholder: proceed for now
    }

    // Entering bootloader is handled by manager for DeviceUuid. For manual path, try flashing directly.

    // Build stm32flash argument generator (adds baud and verify/go)
    let firmware_arg = firmware_path.unwrap_or("");

    // Run two-phase flash: write/verify, then GO only on success
    try_flash_stm32flash_with_options(
        firmware_arg,
        &serial_path,
        device_id,
        Some(3),
        Some(10000),
        on_completion,
        stm32flash_path.as_str(),
    )
    .await
}

pub async fn try_flash_stm32flash_with_options(
    firmware_path: &str,
    serial_path: &str,
    device_id: Option<Uuid>,
    max_attempts: Option<u8>,
    delay_between_attempts_ms: Option<u64>,
    on_completion: Option<Box<dyn FnOnce(bool) + Send + 'static>>,
    stm32flash_cmd: &str,
) -> Result<(FirmwareUpdateResult, tokio::task::JoinHandle<()>), FirmwareUpdateError> {
    // Clone inputs to owned values for 'static lifetime inside spawned tasks
    let firmware_path_owned = firmware_path.to_string();
    let serial_path_owned = serial_path.to_string();
    let write_args = build_stm32flash_write_args(&firmware_path_owned, &serial_path_owned);

    let max_attempts_value: u8 = max_attempts.unwrap_or(3);
    let delay_ms_value: u64 = delay_between_attempts_ms.unwrap_or(5000);

    // Spawn write/verify in background with retry, then GO only on success
    let cmd_path = stm32flash_cmd.to_string();
    let handle = tokio::spawn(async move {
        let mut attempt: u8 = 1;

        loop {
            // Phase 1: write/verify
            let write_result =
                run_stm32flash_phase(&cmd_path, &write_args, "write", device_id).await;

            let should_retry = match write_result {
                FlashResult::Success => {
                    // Phase 2: GO only if write succeeded
                    let go_args = build_stm32flash_go_args(&serial_path_owned);
                    let go_result =
                        run_stm32flash_phase(&cmd_path, &go_args, "GO", device_id).await;
                    match go_result {
                        FlashResult::Success => {
                            broadcast_fw_progress(100.0, device_id);
                            // Call completion callback with success
                            if let Some(callback) = on_completion {
                                callback(true);
                            }
                            return;
                        }
                        FlashResult::PortBusy => {
                            // Port busy during GO phase
                            true
                        }
                        FlashResult::OtherError => {
                            // Other error during GO phase
                            true
                        }
                    }
                }
                FlashResult::PortBusy => {
                    // Port busy during write phase
                    true
                }
                FlashResult::OtherError => {
                    // Other error during write phase
                    true
                }
            };

            if should_retry && attempt < max_attempts_value {
                attempt += 1;

                // Use consistent 5000ms delay for all retries
                let delay_duration = delay_ms_value;

                // Debug: Log the delay duration and timing
                let start_time = std::time::Instant::now();
                broadcast_fw_error(
                    format!(
                        "Starting {}ms delay before retry (attempt {}/{})",
                        delay_duration, attempt, max_attempts_value
                    ),
                    device_id,
                );

                tokio::time::sleep(Duration::from_millis(delay_duration)).await;

                let elapsed = start_time.elapsed();
                broadcast_fw_error(
                    format!(
                        "Delay completed in {:.2}s (expected {:.2}s)",
                        elapsed.as_secs_f64(),
                        delay_duration as f64 / 1000.0
                    ),
                    device_id,
                );
                continue;
            } else {
                if should_retry {
                    broadcast_fw_error(
                        format!(
                            "firmware update failed after {} attempts",
                            max_attempts_value
                        ),
                        device_id,
                    );
                }
                // Call completion callback with failure
                if let Some(callback) = on_completion {
                    callback(false);
                }
                return;
            }
        }
    });

    Ok((FirmwareUpdateResult::Started, handle))
}

fn parse_stm32flash_progress(line: &str) -> Option<f32> {
    // Examples:
    // "Wrote and verified address 0x08020378 (...100.00%) Done."
    // "Writing at address 0x08004000 ( 21.25%)"
    if let Some(start) = line.rfind('(') {
        if let Some(end) = line[start..].find('%') {
            let slice = &line[start + 1..start + end];
            let digits: String = slice
                .chars()
                .filter(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            if let Ok(v) = digits.parse::<f32>() {
                return Some(v.clamp(0.0, 100.0));
            }
        }
    }
    None
}

fn broadcast_fw_progress(percent: f32, device: Option<Uuid>) {
    let mut msg = serde_json::json!({
        "type": "firmware_progress",
        "percent": percent,
    });

    // If we've reached 100%, mark as completed
    if percent >= 100.0 {
        msg["status"] = serde_json::json!("completed");
    }

    crate::server::protocols::v1::websocket::send_to_websockets(msg, device);
}

fn broadcast_fw_error(message: String, device: Option<Uuid>) {
    let msg = serde_json::json!({
        "type": "firmware_progress",
        "status": "error",
        "message": message,
    });
    crate::server::protocols::v1::websocket::send_to_websockets(msg, device);
}

async fn read_progress_stream<R>(stream: R, device_id: Option<Uuid>)
where
    R: tokio::io::AsyncRead + Unpin,
{
    let mut reader = BufReader::new(stream);
    let mut buf = [0u8; 1024];
    let mut acc: Vec<u8> = Vec::new();
    loop {
        match reader.read(&mut buf).await {
            Ok(0) => break,
            Ok(n) => {
                acc.extend_from_slice(&buf[..n]);
                // Split on CR or LF
                let mut idx = 0;
                while let Some(pos) = acc[idx..].iter().position(|&b| b == b'\n' || b == b'\r') {
                    let split_at = idx + pos;
                    let line = String::from_utf8_lossy(&acc[..split_at]).to_string();
                    // Remove up to and including delimiter
                    let remove_len = split_at + 1;
                    acc.drain(..remove_len);
                    idx = 0;
                    if let Some(pct) = parse_stm32flash_progress(&line) {
                        broadcast_fw_progress(pct, device_id);
                    }
                }
                // Also attempt parse on partial line to catch inline updates
                if !acc.is_empty() {
                    if let Ok(s) = std::str::from_utf8(&acc) {
                        if let Some(pct) = parse_stm32flash_progress(s) {
                            broadcast_fw_progress(pct, device_id);
                        }
                    }
                }
            }
            Err(_) => break,
        }
    }
}

fn build_stm32flash_write_args(firmware_path: &str, serial_port: &str) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();
    if !firmware_path.is_empty() {
        args.extend(["-w".to_string(), firmware_path.to_string()]);
    }
    args.push("-v".to_string());
    args.push(serial_port.to_string());
    args
}

fn build_stm32flash_go_args(serial_port: &str) -> Vec<String> {
    vec![
        "-g".to_string(),
        "0x08000000".to_string(),
        serial_port.to_string(),
    ]
}

#[derive(Debug)]
enum FlashResult {
    Success,
    PortBusy,
    OtherError,
}

async fn run_ping360_bootloader(
    bootloader_cmd: &str,
    firmware_path: &str,
    serial_path: &str,
    device_id: Option<Uuid>,
) -> Result<(), FirmwareUpdateError> {
    let mut cmd = tokio::process::Command::new(bootloader_cmd);
    cmd.args([serial_path, firmware_path, "--bootloader"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| {
        FirmwareUpdateError::Process(format!("Failed to spawn ping360-bootloader: {}", e))
    })?;

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    // Create readers for stdout and stderr
    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();

    // Track if we've seen success indicators
    let saw_config_write = std::sync::Arc::new(std::sync::Mutex::new(false));
    let saw_app_start = std::sync::Arc::new(std::sync::Mutex::new(false));

    let saw_config_write_clone = saw_config_write.clone();
    let saw_app_start_clone = saw_app_start.clone();
    let device_id_clone = device_id;

    // Spawn a task to read from streams
    let read_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                line = stdout_reader.next_line() => {
                    match line {
                        Ok(Some(line)) => {
                            // Parse progress from the line
                            if line.contains("writing application...") {
                                broadcast_fw_progress(25.0, device_id_clone);
                            } else if line.contains("verifying application...") {
                                broadcast_fw_progress(75.0, device_id_clone);
                            } else if line.contains("writing configuration...ok") {
                                broadcast_fw_progress(90.0, device_id_clone);
                                *saw_config_write_clone.lock().unwrap() = true;
                            } else if line.contains("starting application...ok") {
                                broadcast_fw_progress(100.0, device_id_clone);
                                *saw_app_start_clone.lock().unwrap() = true;
                            }
                        }
                        Ok(None) => break, // EOF
                        Err(_) => break,
                    }
                }
                line = stderr_reader.next_line() => {
                    match line {
                        Ok(Some(line)) => {
                            // Also check stderr for progress indicators (though unlikely)
                            if line.contains("writing configuration...ok") {
                                broadcast_fw_progress(90.0, device_id_clone);
                                *saw_config_write_clone.lock().unwrap() = true;
                            } else if line.contains("starting application...ok") {
                                broadcast_fw_progress(100.0, device_id_clone);
                                *saw_app_start_clone.lock().unwrap() = true;
                            }
                        }
                        Ok(None) => break, // EOF
                        Err(_) => break,
                    }
                }
            }
        }
    });

    // Wait for the process to complete
    let status = child.wait().await.map_err(|e| {
        FirmwareUpdateError::Process(format!("ping360-bootloader wait error: {}", e))
    })?;

    // Wait for the reading task to complete
    let _ = read_task.await;

    // Check the exit status
    if status.success() {
        Ok(())
    } else {
        let code = status.code().unwrap_or(0);
        // Check if flashing succeeded despite exit code 1
        if *saw_config_write.lock().unwrap() && *saw_app_start.lock().unwrap() {
            Ok(())
        } else {
            Err(FirmwareUpdateError::Process(format!(
                "ping360-bootloader failed with exit code {} for device {} on port {} with firmware {}",
                code, device_id.map(|id| id.to_string()).unwrap_or("unknown".to_string()), serial_path, firmware_path
            )))
        }
    }
}

async fn run_stm32flash_phase(
    cmd_path: &str,
    args: &[String],
    phase: &str,
    device_id: Option<Uuid>,
) -> FlashResult {
    let mut cmd = tokio::process::Command::new(cmd_path);
    cmd.args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    match cmd.spawn() {
        Ok(mut child) => {
            let stdout = child.stdout.take();
            let stderr = child.stderr.take();

            if let Some(out) = stdout {
                let device_id_clone = device_id;
                tokio::spawn(async move {
                    read_progress_stream(out, device_id_clone).await;
                });
            }
            if let Some(err) = stderr {
                let device_id_clone = device_id;
                tokio::spawn(async move {
                    read_progress_stream(err, device_id_clone).await;
                });
            }

            match child.wait().await {
                Ok(status) => {
                    if status.success() {
                        FlashResult::Success
                    } else {
                        // Check if this is likely a port busy error
                        let code = status.code();
                        if code == Some(1) || code == Some(2) {
                            broadcast_fw_error(
                                format!("stm32flash {} failed - port may be busy (exit code {}): status {}", phase, code.unwrap_or(0), status),
                                device_id,
                            );
                            FlashResult::PortBusy
                        } else {
                            broadcast_fw_error(
                                format!("stm32flash {} failed: status {}", phase, status),
                                device_id,
                            );
                            FlashResult::OtherError
                        }
                    }
                }
                Err(e) => {
                    broadcast_fw_error(
                        format!("stm32flash {} wait error: {}", phase, e),
                        device_id,
                    );
                    FlashResult::OtherError
                }
            }
        }
        Err(e) => {
            broadcast_fw_error(
                format!("stm32flash {} spawn error: {}", phase, e),
                device_id,
            );
            FlashResult::OtherError
        }
    }
}

pub const PING1D_REV1_URL: &str = "https://raw.githubusercontent.com/bluerobotics/ping-firmware/master/ping1d/Ping-V3.29_auto.hex";
pub const PING1D_REV2_URL: &str = "https://raw.githubusercontent.com/bluerobotics/ping-firmware/master/ping2/Ping2-V1.1.0_auto.hex";
pub const PING360_URL: &str = "https://raw.githubusercontent.com/bluerobotics/ping-firmware/master/ping360/Ping360-V3.3.8_auto.hex";

pub const PING360_BOOTLOADER_PATH: &str = "firmwares/utils/ping360-bootloader";

pub fn default_firmware_base_dir() -> PathBuf {
    PathBuf::from("firmwares")
}

pub fn expected_local_firmware_path(revision: u8, base: &Path) -> (PathBuf, &'static str) {
    if revision == 2 {
        (
            base.join("ping2").join("Ping2-V1.1.0_auto.hex"),
            PING1D_REV2_URL,
        )
    } else {
        (
            base.join("ping1d").join("Ping-V3.29_auto.hex"),
            PING1D_REV1_URL,
        )
    }
}

pub fn expected_local_ping360_firmware_path(base: &Path) -> (PathBuf, &'static str) {
    (
        base.join("ping360").join("Ping360-V3.3.8_auto.hex"),
        PING360_URL,
    )
}

pub async fn ensure_firmware_present(
    revision: u8,
    base: &Path,
) -> Result<PathBuf, FirmwareUpdateError> {
    let (path, url) = expected_local_firmware_path(revision, base);

    if path.exists() {
        return Ok(path);
    }

    // Create directories if missing
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| FirmwareUpdateError::Io(e.to_string()))?;
    }

    // Try to download from GitHub raw URL
    let body = reqwest::get(url)
        .await
        .map_err(|e| FirmwareUpdateError::Process(format!("download: {e}")))?
        .bytes()
        .await
        .map_err(|e| FirmwareUpdateError::Process(format!("download bytes: {e}")))?;
    tokio::fs::write(&path, &body)
        .await
        .map_err(|e| FirmwareUpdateError::Io(e.to_string()))?;
    Ok(path)
}

pub async fn ensure_ping360_firmware_present(base: &Path) -> Result<PathBuf, FirmwareUpdateError> {
    let (path, url) = expected_local_ping360_firmware_path(base);

    if path.exists() {
        return Ok(path);
    }

    // Create directories if missing
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| FirmwareUpdateError::Io(e.to_string()))?;
    }

    // Try to download from GitHub raw URL
    let body = reqwest::get(url)
        .await
        .map_err(|e| FirmwareUpdateError::Process(format!("download: {e}")))?
        .bytes()
        .await
        .map_err(|e| FirmwareUpdateError::Process(format!("download bytes: {e}")))?;
    tokio::fs::write(&path, &body)
        .await
        .map_err(|e| FirmwareUpdateError::Io(e.to_string()))?;
    Ok(path)
}
