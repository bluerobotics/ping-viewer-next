use std::net::Ipv4Addr;

use tokio_serial::available_ports;
use tracing::warn;

use super::{SourceSelection, SourceSerialStruct, SourceUdpStruct};

#[derive(Debug, PartialEq)]
pub struct DiscoveryResponse {
    pub device_name: String,
    pub manufacturer: String,
    pub mac_address: String,
    pub ip_address: Ipv4Addr,
}

use regex::Regex;

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

pub fn serial_discovery() -> Option<Vec<SourceSelection>> {
    match available_ports() {
        Ok(serial_ports) => {
            let mut available_sources = Vec::new();
            for port_info in serial_ports {
                let source = SourceSelection::SerialStream(SourceSerialStruct {
                    path: port_info.port_name.clone(),
                    baudrate: 115200,
                });
                available_sources.push(source);
            }
            Some(available_sources)
        }
        Err(err) => {
            warn!("Auto create: Unable to find available devices on serial ports, details: {err}");
            None
        }
    }
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
