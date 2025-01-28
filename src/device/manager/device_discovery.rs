use std::{net::Ipv4Addr, time::Duration};

use tokio::{io::AsyncWriteExt, task::JoinSet, time::timeout};
use tokio_serial::{available_ports, SerialPort, SerialPortBuilderExt, SerialStream};
use tracing::{debug, error, info, trace, warn};

use crate::device::manager::ManagerError;

use super::{SourceSelection, SourceSerialStruct, SourceUdpStruct};
use regex::Regex;
use std::collections::HashMap;

#[cfg(feature = "blueos-extension")]
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq)]
pub struct DiscoveryResponse {
    pub device_name: String,
    pub manufacturer: String,
    pub mac_address: String,
    pub ip_address: Ipv4Addr,
}

#[cfg(feature = "blueos-extension")]
#[derive(Debug, Deserialize, Serialize)]
struct DriverStatus {
    udp_port: Option<u16>,
    mavlink_driver_enabled: bool,
}

#[cfg(feature = "blueos-extension")]
#[derive(Debug, Deserialize, Serialize)]
struct PingDevice {
    ping_type: String,
    device_id: u8,
    device_model: u8,
    device_revision: u8,
    firmware_version_major: u8,
    firmware_version_minor: u8,
    firmware_version_patch: u8,
    port: String,
    ethernet_discovery_info: Option<String>,
    driver_status: DriverStatus,
}

#[cfg(feature = "blueos-extension")]
pub struct BluePingDiscoveryResult {
    pub sources: Vec<SourceSelection>,
    pub used_ports: Vec<String>,
}

impl DiscoveryResponse {
    /// Decode Ping360 ASCII NetworkDiscovery message using regex
    fn from_response(response: &str) -> Option<Self> {
        let device_name_part = r"(?P<device_name>.+)";
        let manufacturer_part = r"(?P<manufacturer>.+)";
        let mac_address_part = r"MAC\sAddress:-\s(?P<mac_address>[A-Fa-f0-9\-]+)";
        let ip_address_part = r"IP\sAddress:-\s*0{0,2}(?<octet_1>[0-9]{1,3})\.0{0,2}(?<octet_2>[0-9]{1,3})\.0{0,2}(?<octet_3>[0-9]{1,3})\.0{0,2}(?<octet_4>[0-9]{1,3})";

        let regex_pattern = format!(
            r"(?x)
            ^{device_name_part}\r\n
            {manufacturer_part}\r\n
            {mac_address_part}\r\n
            {ip_address_part}\r\n$
            "
        );

        let re = match Regex::new(&regex_pattern) {
            Ok(regex) => regex,
            Err(err) => {
                warn!("auto_create: Failed to compile regex: {err}");
                return None;
            }
        };

        let captures = re.captures(response)?;

        let device_name = captures["device_name"].trim().to_string();
        let manufacturer = captures["manufacturer"].trim().to_string();
        let mac_address = captures["mac_address"].to_string();
        let ip_address_str = format!(
            "{}.{}.{}.{}",
            &captures["octet_1"], &captures["octet_2"], &captures["octet_3"], &captures["octet_4"],
        );

        let ip_address = match ip_address_str.parse::<Ipv4Addr>() {
            Ok(ip) => ip,
            Err(err) => {
                warn!(
                    "auto_create: network: Failed to parse IP address: {ip_address_str}, details: {err}"
                );
                return None;
            }
        };

        Some(DiscoveryResponse {
            device_name,
            manufacturer,
            mac_address,
            ip_address,
        })
    }
}

pub fn network_discovery() -> Option<Vec<SourceSelection>> {
    let socket = match std::net::UdpSocket::bind("0.0.0.0:0") {
        Ok(s) => s,
        Err(err) => {
            warn!("auto_create: network: Failed to bind to socket: {err}");
            return None;
        }
    };

    if let Err(err) = socket.set_broadcast(true) {
        warn!("auto_create: network: Failed to enable broadcast: {err}");
        return None;
    }

    let broadcast_addr = "255.255.255.255:30303";
    let discovery_message = "Discovery";

    if let Err(err) = socket.send_to(discovery_message.as_bytes(), broadcast_addr) {
        warn!("auto_create: network: Failed to send discovery message: {err}");
        return None;
    }

    if let Err(err) = socket.set_read_timeout(Some(std::time::Duration::from_secs(2))) {
        warn!("auto_create: network: Failed to set read timeout: {err}");
        return None;
    }

    let mut buf = [0; 1024];
    let mut responses = Vec::new();

    loop {
        match socket.recv_from(&mut buf) {
            Ok((size, src)) => {
                let response = match std::str::from_utf8(&buf[..size]) {
                    Ok(r) => r,
                    Err(err) => {
                        warn!("auto_create: network: Received invalid UTF-8 response from: {src}, details: {err}");
                        continue;
                    }
                };

                if let Some(discovery_response) = DiscoveryResponse::from_response(response) {
                    responses.push(discovery_response);
                } else {
                    warn!(
                        "auto_create: network: Failed to parse the discovery response from: {src}"
                    );
                }
            }
            Err(err) => {
                warn!("auto_create: network: Timeout or error receiving response: {err}");
                break;
            }
        }
    }

    if responses.is_empty() {
        warn!("auto_create: network: No valid discovery responses were collected.");
        return None;
    }

    let mut available_sources = Vec::new();
    for device in responses {
        let source = SourceSelection::UdpStream(SourceUdpStruct {
            ip: device.ip_address,
            port: 12345,
        });

        available_sources.push(source);
    }
    Some(available_sources)
}

// Discovery function that uses BlueOS's ping service to find current bridged devices
#[cfg(feature = "blueos-extension")]
pub async fn blueos_ping_discovery() -> Option<BluePingDiscoveryResult> {
    let client = reqwest::Client::new();
    let response = match client
        .get("http://localhost:9110/v1.0/sensors")
        .header("accept", "application/json")
        .timeout(Duration::from_millis(500))
        .send()
        .await
    {
        Ok(response) => response,
        Err(err) => {
            warn!("blue_ping_discovery: Failed to connect to Ping service: {err}");
            return None;
        }
    };

    let devices: Vec<PingDevice> = match response.json().await {
        Ok(devices) => devices,
        Err(err) => {
            warn!("blue_ping_discovery: Failed to parse response: {err}");
            return None;
        }
    };

    debug!("blue_ping_discovery: Found devices: {devices:?}");

    let mut available_sources = Vec::new();
    let mut used_ports = Vec::new();

    for device in devices {
        if !device.port.is_empty() {
            used_ports.push(device.port);
        }

        if let Some(udp_port) = device.driver_status.udp_port {
            available_sources.push(SourceSelection::UdpStream(SourceUdpStruct {
                ip: Ipv4Addr::new(127, 0, 0, 1),
                port: udp_port,
            }));
        }
    }

    Some(BluePingDiscoveryResult {
        sources: available_sources,
        used_ports,
    })
}

pub async fn serial_discovery(skip_ports: Option<&[String]>) -> Option<Vec<SourceSelection>> {
    match available_ports() {
        Ok(serial_ports) => {
            debug!("serial_discovery: Found {serial_ports:?}");

            let mut set: JoinSet<Result<SourceSelection, ManagerError>> = JoinSet::new();

            // Filter ports if skip_ports is provided
            let filtered_ports = serial_ports
                .into_iter()
                .filter(|port_info| match skip_ports {
                    Some(skip_list) => !skip_list.contains(&port_info.port_name),
                    None => true,
                });

            filtered_ports.for_each(|port_info| {
                let path = port_info.port_name.clone();
                set.spawn(async move {
                    let baud_rate = auto_detect_baudrate(path.clone()).await?;

                    Ok(SourceSelection::SerialStream(SourceSerialStruct {
                        path,
                        baudrate: baud_rate,
                    }))
                });
            });

            let mut available_sources = Vec::new();
            while let Some(result) = set.join_next().await {
                match result {
                    Ok(Ok(source)) => {
                        available_sources.push(source);
                    }
                    Ok(Err(e)) => {
                        error!("serial_discovery: Port detection error: {e:?}");
                    }
                    Err(e) => {
                        error!("serial_discovery: Task error: {e:?}");
                    }
                }
            }

            if available_sources.is_empty() {
                warn!("serial_discovery: No valid serial devices were found");
                None
            } else {
                info!("serial_discovery: Devices Available : {available_sources:?}");
                Some(available_sources)
            }
        }
        Err(err) => {
            warn!("Auto create: Unable to find available devices on serial ports, details: {err}");
            None
        }
    }
}

async fn auto_detect_baudrate(path: String) -> Result<u32, ManagerError> {
    const BAUDRATE_CHECK_MESSAGES: usize = 10;
    const TOTAL_CHECK_TIMEOUT_MS: u64 = 2000;

    let baud_rates = [
        2500000, 2000000, 1843200, 921600, 460800, 230400, 115200, 9600,
    ];
    let mut baudrate_results: HashMap<u32, BaudrateCheckResult> = HashMap::new();

    for &rate in &baud_rates {
        debug!("auto_detect_baudrate: Testing baud rate: {}", rate);

        let mut serial_stream = match tokio_serial::new(path.clone(), rate).open_native_async() {
            Ok(stream) => stream,
            Err(err) => {
                warn!(
                    "auto_detect_baudrate: Failed to open port at {}: {}",
                    rate, err
                );
                continue;
            }
        };

        #[cfg(unix)]
        if let Err(e) = serial_stream.set_exclusive(false) {
            warn!(
                "auto_detect_baudrate: Failed to set non-exclusive mode: {}",
                e
            );
            continue;
        }

        if let Err(e) = set_baudrate_pre_routine(&mut serial_stream, rate).await {
            warn!(
                "auto_detect_baudrate: Failed baudrate initialization at {}: {:?}",
                rate, e
            );
            continue;
        }

        let temp_device = bluerobotics_ping::common::Device::new(serial_stream);

        match timeout(
            Duration::from_millis(TOTAL_CHECK_TIMEOUT_MS),
            test_baudrate_quality(&temp_device, BAUDRATE_CHECK_MESSAGES),
        )
        .await
        {
            Ok(Ok(result)) => {
                if result.parser_errors == 0 && result.messages_received == BAUDRATE_CHECK_MESSAGES
                {
                    info!("auto_detect_baudrate: Found optimal baudrate: {}", rate);
                    return Ok(rate);
                }
                if result.messages_received > 0 {
                    baudrate_results.insert(rate, result);
                }
            }
            Ok(Err(e)) => {
                debug!(
                    "auto_detect_baudrate: Failed quality check at {}: {:?}",
                    rate, e
                );
            }
            Err(_) => {
                debug!(
                    "auto_detect_baudrate: Timeout during baudrate check at {}",
                    rate
                );
            }
        }
    }

    select_best_baudrate(baudrate_results).ok_or_else(|| {
        ManagerError::Other(
            "auto_detect_baudrate: Failed to auto-detect baud rate: No successful communication at any baudrate"
                .to_string(),
        )
    })
}

async fn test_baudrate_quality(
    device: &bluerobotics_ping::common::Device,
    num_checks: usize,
) -> Result<BaudrateCheckResult, ManagerError> {
    let mut result = BaudrateCheckResult {
        messages_received: 0,
        parser_errors: 0,
    };

    for _ in 0..num_checks {
        match timeout(Duration::from_millis(300), device.device_information()).await {
            Ok(Ok(_)) => {
                result.messages_received += 1;
            }
            Ok(Err(e)) => {
                debug!("test_baudrate_quality: test_baudrate_quality: {e:?}");
                result.parser_errors += 1;
            }
            Err(e) => {
                debug!("test_baudrate_quality: test_baudrate_quality: {e:?}");
                result.parser_errors += 1;
            }
        }
    }

    Ok(result)
}

#[derive(Debug, Clone, PartialEq)]
struct BaudrateCheckResult {
    messages_received: usize,
    parser_errors: usize,
}

/// Selects the optimal baudrate from test results based on the following criteria:
/// 1. Highest number of successful messages received
/// 2. If tied on messages, lowest number of parser/timeout errors
/// 3. If tied on both messages and errors, highest baudrate is preferred
fn select_best_baudrate(results: HashMap<u32, BaudrateCheckResult>) -> Option<u32> {
    trace!("Starting baudrate selection with {} results", results.len());

    if results.is_empty() {
        trace!("select_best_baudrate: No baudrate results available");
        return None;
    }

    results.iter().for_each(|(rate, result)| {
        trace!(
            "select_best_baudrate: Initial baudrate {rate}: {} successful messages, {} parser errors",
            result.messages_received,
            result.parser_errors
        );
    });

    // Helper to log comparison results
    fn log_comparison(
        rate1: u32,
        result1: &BaudrateCheckResult,
        rate2: u32,
        result2: &BaudrateCheckResult,
        reason: &str,
        selected: u32,
    ) {
        trace!(
            "select_best_baudrate: Comparing baudrates:\n\
            {rate1} (messages: {}, errors: {}) vs\n\
            {rate2} (messages: {}, errors: {})\n\
            Selected {selected} based on {reason}",
            result1.messages_received,
            result1.parser_errors,
            result2.messages_received,
            result2.parser_errors,
        );
    }

    let selected = results
        .into_iter()
        .filter(|(_, result)| result.messages_received > 0)
        .max_by(|(rate1, result1), (rate2, result2)| {
            match result1.messages_received.cmp(&result2.messages_received) {
                std::cmp::Ordering::Equal => {
                    match result2.parser_errors.cmp(&result1.parser_errors) {
                        std::cmp::Ordering::Equal => {
                            trace!(
                                "select_best_baudrate: Equal messages and errors, selecting higher baudrate {} over {}",
                                rate1, rate2
                            );
                            rate1.cmp(rate2)
                        }
                        ordering => {
                            log_comparison(
                                *rate1, result1, *rate2, result2,
                                "error count",
                                if ordering == std::cmp::Ordering::Greater { *rate1 } else { *rate2 }
                            );
                            ordering
                        }
                    }
                }
                ordering => {
                    log_comparison(
                        *rate1, result1, *rate2, result2,
                        "message count",
                        if ordering == std::cmp::Ordering::Greater { *rate1 } else { *rate2 }
                    );
                    ordering
                }
            }
        })
        .map(|(rate, result)| {
            trace!(
                "select_best_baudrate: Final selection: baudrate {} with {} successful messages and {} errors",
                rate, result.messages_received, result.parser_errors
            );
            rate
        });

    if selected.is_none() {
        trace!("select_best_baudrate: No suitable baudrate found after comparison");
    }

    selected
}

pub async fn set_baudrate_pre_routine(
    port: &mut SerialStream,
    baud_rate: u32,
) -> Result<(), ManagerError> {
    timeout(Duration::from_millis(100), async {
        port.set_baud_rate(baud_rate).map_err(|e| {
            ManagerError::Other(format!("Failed to set baud rate {}: {}", baud_rate, e))
        })?;
        tokio::time::sleep(Duration::from_millis(10)).await;

        port.set_break()
            .map_err(|e| ManagerError::Other(format!("Failed to set BREAK: {}", e)))?;
        tokio::time::sleep(Duration::from_millis(10)).await;

        port.clear_break()
            .map_err(|e| ManagerError::Other(format!("Failed to clear BREAK: {}", e)))?;
        tokio::time::sleep(Duration::from_millis(10)).await;

        port.write_all(b"U")
            .await
            .map_err(|e| ManagerError::Other(format!("Failed to write 'U': {}", e)))?;
        tokio::time::sleep(Duration::from_millis(10)).await;

        port.flush()
            .await
            .map_err(|e| ManagerError::Other(format!("Failed to flush: {}", e)))?;
        tokio::time::sleep(Duration::from_millis(10)).await;

        port.clear(tokio_serial::ClearBuffer::All)
            .map_err(|err| ManagerError::DeviceSourceError(err.to_string()))?;

        Ok(())
    })
    .await
    .map_err(|_| ManagerError::Other("set_baudrate_pre_routine: Operation timed out".to_string()))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discovery_response_parsing() {
        let response = "SONAR PING360\r\n\
                        Blue Robotics\r\n\
                        MAC Address:- 54-10-EC-79-7D-D1\r\n\
                        IP Address:- 192.168.000.197\r\n";

        let expected = DiscoveryResponse {
            device_name: "SONAR PING360".to_string(),
            manufacturer: "Blue Robotics".to_string(),
            mac_address: "54-10-EC-79-7D-D1".to_string(),
            ip_address: Ipv4Addr::new(192, 168, 0, 197),
        };

        let parsed = DiscoveryResponse::from_response(response);
        assert_eq!(parsed, Some(expected));
    }

    #[test]
    fn test_invalid_response_parsing() {
        let invalid_response = "INVALID RESPONSE FORMAT";

        let parsed = DiscoveryResponse::from_response(invalid_response);
        assert!(parsed.is_none());
    }

    #[test]
    fn test_multiple_discovery_responses() {
        let response_1 = "SONAR PING360\r\n\
                          Blue Robotics\r\n\
                          MAC Address:- 54-10-EC-79-7D-D1\r\n\
                          IP Address:- 192.168.000.197\r\n";

        let response_2 = "SONAR PING360\r\n\
                          Blue Robotics\r\n\
                          MAC Address:- 54-10-EC-79-7D-D2\r\n\
                          IP Address:- 192.168.000.198\r\n";

        let expected_1 = DiscoveryResponse {
            device_name: "SONAR PING360".to_string(),
            manufacturer: "Blue Robotics".to_string(),
            mac_address: "54-10-EC-79-7D-D1".to_string(),
            ip_address: Ipv4Addr::new(192, 168, 0, 197),
        };

        let expected_2 = DiscoveryResponse {
            device_name: "SONAR PING360".to_string(),
            manufacturer: "Blue Robotics".to_string(),
            mac_address: "54-10-EC-79-7D-D2".to_string(),
            ip_address: Ipv4Addr::new(192, 168, 0, 198),
        };

        let responses = vec![response_1, response_2];

        let mut parsed_responses = Vec::new();
        for response in responses {
            if let Some(parsed) = DiscoveryResponse::from_response(response) {
                parsed_responses.push(parsed);
            }
        }

        assert_eq!(parsed_responses, vec![expected_1, expected_2]);
    }
}
