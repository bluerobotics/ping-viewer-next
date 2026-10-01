use std::collections::HashSet;
use std::net::SocketAddrV4;
use std::time::Duration;

use bluerobotics_ping::ping1d::Device as Ping1D;
use bluerobotics_ping::ping360::Device as Ping360;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tokio::time::sleep;
use tokio_serial::{SerialPort, SerialPortBuilderExt, SerialStream};
use tracing::{debug, error, info, trace, warn};
use udp_stream::UdpStream;

use crate::device::devices::{DeviceActor, DeviceType, PingAnswer, UpgradeResult};
use crate::device::fake::FakeStream;
use crate::device::manager::{DeviceProperties, ManagerError};

use super::{device_discovery, DeviceSelection, DeviceStatus, SourceSelection, SourceType};

pub struct DeviceFactory;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeviceDiscoveryInfo {
    pub source: SourceSelection,
    pub status: DeviceStatus,
    pub device_type: DeviceSelection,
    pub properties: Option<DeviceProperties>,
}

impl DeviceFactory {
    pub async fn create_device(
        source: SourceSelection,
        mut device_type: DeviceSelection,
    ) -> Result<DeviceDiscoveryInfo, ManagerError> {
        let port = match &source {
            SourceSelection::UdpStream(source_udp_struct) => {
                let socket_addr = SocketAddrV4::new(source_udp_struct.ip, source_udp_struct.port);

                let udp_stream = UdpStream::connect(socket_addr.into())
                    .await
                    .map_err(|err| ManagerError::DeviceSourceError(err.to_string()))?;
                SourceType::Udp(udp_stream)
            }
            SourceSelection::SerialStream(source_serial_struct) => {
                let mut serial_stream: SerialStream =
                    tokio_serial::new(&source_serial_struct.path, source_serial_struct.baudrate)
                        .open_native_async()
                        .map_err(|err| ManagerError::DeviceSourceError(err.to_string()))?;

                device_discovery::set_baudrate_pre_routine(
                    &mut serial_stream,
                    source_serial_struct.baudrate,
                )
                .await?;

                serial_stream
                    .clear(tokio_serial::ClearBuffer::All)
                    .map_err(|err| ManagerError::DeviceSourceError(err.to_string()))?;

                SourceType::Serial(serial_stream)
            }
            SourceSelection::FakeStream(_) => SourceType::Fake(FakeStream::new(device_type)),
        };

        let device = match port {
            SourceType::Udp(udp_port) => match device_type {
                DeviceSelection::Common | DeviceSelection::Auto => {
                    DeviceType::Common(bluerobotics_ping::common::Device::new(udp_port))
                }
                DeviceSelection::Ping1D => DeviceType::Ping1D(Ping1D::new(udp_port)),
                DeviceSelection::Ping360 => DeviceType::Ping360(Ping360::new(udp_port)),
            },
            SourceType::Serial(serial_port) => match device_type {
                DeviceSelection::Common | DeviceSelection::Auto => {
                    DeviceType::Common(bluerobotics_ping::common::Device::new(serial_port))
                }
                DeviceSelection::Ping1D => DeviceType::Ping1D(Ping1D::new(serial_port)),
                DeviceSelection::Ping360 => DeviceType::Ping360(Ping360::new(serial_port)),
            },
            SourceType::Fake(fake_port) => match device_type {
                DeviceSelection::Common | DeviceSelection::Auto => {
                    DeviceType::Common(bluerobotics_ping::common::Device::new(fake_port))
                }
                DeviceSelection::Ping1D => DeviceType::Ping1D(Ping1D::new(fake_port)),
                DeviceSelection::Ping360 => DeviceType::Ping360(Ping360::new(fake_port)),
            },
        };

        let (mut device, _handler) = DeviceActor::new(device, 1);

        if device_type == DeviceSelection::Auto {
            let mut retry_count = 0;
            let max_retries = 3;
            let retry_delay = Duration::from_millis(100);

            loop {
                match device.try_upgrade().await {
                    Ok(PingAnswer::UpgradeResult(result)) => {
                        match result {
                            UpgradeResult::Unknown => {
                                device_type = DeviceSelection::Common;
                            }
                            UpgradeResult::Ping1D => {
                                device_type = DeviceSelection::Ping1D;
                            }
                            UpgradeResult::Ping360 => {
                                device_type = DeviceSelection::Ping360;
                            }
                        }
                        break;
                    }
                    Err(error) => {
                        retry_count += 1;
                        if retry_count >= max_retries {
                            error!(
                                ?error, max_retries,
                                "Device creation error: Can't auto upgrade the DeviceType after max_retries attempts"                            );
                            return Err(ManagerError::DeviceError(error));
                        }

                        warn!(
                            ?error,
                            retry_count,
                            max_retries,
                            "Device creation error: Device upgrade attempt failed. Retrying...",
                        );

                        debug!("Force stopping device for discovery service next attempt");
                        match crate::device::manager::turnoff_device_continuous_mode(&source).await
                        {
                            Ok(()) => debug!("Force stopping device success"),
                            Err(error) => error!(
                                ?error,
                                "Force stopping device for discovery service next attempt error"
                            ),
                        };

                        sleep(retry_delay).await;
                        continue;
                    }
                    error => warn!(?error, "Device creation error"),
                }
            }
        }

        let device = DeviceDiscoveryInfo {
            source,
            status: DeviceStatus::Available,
            device_type,
            properties: None,
        };

        Ok(device)
    }
}

pub struct DeviceDiscoveryManager {
    tx: broadcast::Sender<DeviceDiscoveryInfo>,
    handle: Option<tokio::task::JoinHandle<()>>,
    known_sources_rx: broadcast::Receiver<Vec<SourceSelection>>,
}

impl DeviceDiscoveryManager {
    pub fn new(
        known_sources_rx: broadcast::Receiver<Vec<SourceSelection>>,
    ) -> (Self, broadcast::Receiver<DeviceDiscoveryInfo>) {
        let (tx, rx) = broadcast::channel(10);
        (
            Self {
                tx,
                handle: None,
                known_sources_rx,
            },
            rx,
        )
    }

    pub fn start_discovery(&mut self) {
        let tx = self.tx.clone();
        let mut known_devices_rx = self.known_sources_rx.resubscribe();

        let handle = tokio::spawn(async move {
            let mut device_keys = HashSet::new();

            loop {
                #[cfg_attr(feature = "blueos-extension", allow(unused_variables))]
                let known_sources = match known_devices_rx.try_recv() {
                    Ok(device_sources) => {
                        device_keys.clear();
                        for source in &device_sources {
                            let key = get_device_key(source);
                            device_keys.insert(key);
                        }
                        device_sources
                    }
                    Err(tokio::sync::broadcast::error::TryRecvError::Empty) => {
                        vec![]
                    }
                    Err(error) => {
                        error!(?error, "Error receiving known devices update");
                        continue;
                    }
                };

                let mut available_sources = Vec::new();

                #[cfg(feature = "blueos-extension")]
                if let Some(discovery_result) = device_discovery::blueos_ping_discovery().await {
                    for source in discovery_result.sources {
                        let key = get_device_key(&source);
                        if !device_keys.contains(&key) {
                            available_sources.push(source);
                        }
                    }
                }

                if let Some(result) = device_discovery::network_discovery().await {
                    for source in result {
                        let key = get_device_key(&source);
                        if !device_keys.contains(&key) {
                            available_sources.push(source);
                        }
                    }
                }

                #[cfg(not(feature = "blueos-extension"))]
                let used_ports: Vec<String> = known_sources
                    .iter()
                    .filter_map(|source| {
                        if let SourceSelection::SerialStream(serial) = &source {
                            Some(serial.path.clone())
                        } else {
                            None
                        }
                    })
                    .collect();

                // Add serial devices, skipping used ports
                #[cfg(not(feature = "blueos-extension"))]
                if let Some(result) = device_discovery::serial_discovery(Some(&used_ports)).await {
                    for source in result {
                        let key = get_device_key(&source);
                        if !device_keys.contains(&key) {
                            available_sources.push(source);
                        }
                    }
                }

                // Process discovered sources
                for source in available_sources {
                    let key = get_device_key(&source);
                    trace!(key, "Attempting to create device for source");

                    match DeviceFactory::create_device(source.clone(), DeviceSelection::Auto).await
                    {
                        Ok(device_discovery_info) => {
                            trace!(key, ?device_discovery_info, "Created new device");
                            let _ = tx.send(device_discovery_info);
                        }
                        Err(error) => {
                            error!(key, ?error, "Failed to create device");
                        }
                    }
                }

                tokio::time::sleep(Duration::from_secs(30)).await;
            }
        });

        self.handle = Some(handle);
    }

    pub fn stop_discovery(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
    }
}

fn get_device_key(source: &SourceSelection) -> String {
    match source {
        SourceSelection::SerialStream(serial) => serial.path.clone(),
        SourceSelection::UdpStream(udp) => format!("{}:{}", udp.ip, udp.port),
        SourceSelection::FakeStream(fake) => format!("Fake #{}", fake.fake_id),
    }
}

impl Drop for DeviceDiscoveryManager {
    fn drop(&mut self) {
        self.stop_discovery();
    }
}

pub struct DiscoveryComponent {
    manager: DeviceDiscoveryManager,
    rx: broadcast::Receiver<DeviceDiscoveryInfo>,
    known_sources_tx: broadcast::Sender<Vec<SourceSelection>>,
}

impl Default for DiscoveryComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl DiscoveryComponent {
    pub fn new() -> Self {
        let (known_sources_tx, known_sources_rx) = broadcast::channel(1);
        let (manager, rx) = DeviceDiscoveryManager::new(known_sources_rx);

        Self {
            manager,
            rx,
            known_sources_tx,
        }
    }

    pub fn start_discovery(&mut self) {
        self.manager.start_discovery();
        info!("DeviceDiscovery service is running");
    }

    pub fn stop_discovery(&mut self) {
        self.manager.stop_discovery();
        info!("DeviceDiscovery service is stopped");
    }

    pub fn broadcast_known_sources(&self, sources: Vec<SourceSelection>) {
        let _ = self.known_sources_tx.send(sources);
    }

    pub fn get_discovery_rx(&self) -> broadcast::Receiver<DeviceDiscoveryInfo> {
        self.rx.resubscribe()
    }
}
