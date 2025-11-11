use std::collections::HashSet;
use std::net::SocketAddrV4;
use std::time::Duration;

use bluerobotics_ping::device::PingDevice;
use bluerobotics_ping::ping1d::Device as Ping1D;
use bluerobotics_ping::ping360::Device as Ping360;
use tokio::sync::broadcast;
use tokio::time::sleep;
use tokio_serial::{SerialPort, SerialPortBuilderExt, SerialStream};
use tracing::{debug, error, info, trace, warn};
use udp_stream::UdpStream;
use uuid::Uuid;

use crate::device::devices::{DeviceActor, DeviceType, PingAnswer, UpgradeResult};
use crate::device::manager::ManagerError;

use super::{
    device_discovery, CommonProperties, DeviceInfo, DeviceProperties, DeviceSelection,
    DeviceStatus, Ping1DProperties, Ping360Config, Ping360Properties, SourceSelection, SourceType,
};

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub struct DeviceFactory;

impl DeviceFactory {
    pub async fn create_device(
        source: SourceSelection,
        mut device_type: DeviceSelection,
    ) -> Result<DeviceInfo, ManagerError> {
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
                    Err(err) => {
                        retry_count += 1;
                        if retry_count >= max_retries {
                            error!(
                                "Device creation error: Can't auto upgrade the DeviceType after {} attempts, details: {err:?}",
                                max_retries
                            );
                            return Err(ManagerError::DeviceError(err));
                        }

                        warn!(
                            "Device creation error: Device upgrade attempt {} of {} failed: {err:?}. Retrying...",
                            retry_count, max_retries
                        );

                        debug!("Force stopping device for discovery service next attempt");
                        match crate::device::manager::turnoff_device_continuous_mode(&source).await {
                            Ok(()) => debug!("Force stopping device success"),
                            Err(err) =>
                                error!("Force stopping device for discovery service next attempt error. Error: {err:?}")
                        };

                        sleep(retry_delay).await;
                        continue;
                    }
                    e => warn!("Device creation error: Abnormal answer: {e:?}."),
                }
            }
        }

        let mut hasher = DefaultHasher::new();
        source.hash(&mut hasher);
        let id = Uuid::from_u128(hasher.finish() as u128);

        // Try to fetch properties for available device based on resolved type
        let properties: Option<DeviceProperties> = match device_type {
            DeviceSelection::Common => {
                // Common properties
                let common = match &device.device_type {
                    DeviceType::Common(dev) => {
                        let device_information = dev.device_information().await.map_err(|e| {
                            ManagerError::DeviceError(
                                crate::device::devices::DeviceError::PingError(e),
                            )
                        })?;
                        let protocol_version = dev.protocol_version().await.map_err(|e| {
                            ManagerError::DeviceError(
                                crate::device::devices::DeviceError::PingError(e),
                            )
                        })?;
                        CommonProperties {
                            device_information,
                            protocol_version,
                        }
                    }
                    // DeviceType::Ping1D(dev) => {
                    //     let device_information = dev.device_information().await.map_err(|e| {
                    //         ManagerError::DeviceError(
                    //             crate::device::devices::DeviceError::PingError(e),
                    //         )
                    //     })?;
                    //     let protocol_version = dev.protocol_version().await.map_err(|e| {
                    //         ManagerError::DeviceError(
                    //             crate::device::devices::DeviceError::PingError(e),
                    //         )
                    //     })?;
                    //     CommonProperties {
                    //         device_information,
                    //         protocol_version,
                    //     }
                    // }
                    // DeviceType::Ping360(dev) => {
                    //     let device_information = dev.device_information().await.map_err(|e| {
                    //         ManagerError::DeviceError(
                    //             crate::device::devices::DeviceError::PingError(e),
                    //         )
                    //     })?;
                    //     let protocol_version = dev.protocol_version().await.map_err(|e| {
                    //         ManagerError::DeviceError(
                    //             crate::device::devices::DeviceError::PingError(e),
                    //         )
                    //     })?;
                    //     CommonProperties {
                    //         device_information,
                    //         protocol_version,
                    //     }
                    // }
                    _ => unreachable!(),
                };
                Some(DeviceProperties::Common(common))
            }
            DeviceSelection::Ping1D => {
                let common = match &device.device_type {
                    DeviceType::Ping1D(dev) => {
                        let device_information = dev.device_information().await.map_err(|e| {
                            ManagerError::DeviceError(
                                crate::device::devices::DeviceError::PingError(e),
                            )
                        })?;
                        let protocol_version = dev.protocol_version().await.map_err(|e| {
                            ManagerError::DeviceError(
                                crate::device::devices::DeviceError::PingError(e),
                            )
                        })?;
                        CommonProperties {
                            device_information,
                            protocol_version,
                        }
                    }
                    // DeviceType::Common(dev) => {
                    //     let device_information = dev.device_information().await.map_err(|e| {
                    //         ManagerError::DeviceError(
                    //             crate::device::devices::DeviceError::PingError(e),
                    //         )
                    //     })?;
                    //     let protocol_version = dev.protocol_version().await.map_err(|e| {
                    //         ManagerError::DeviceError(
                    //             crate::device::devices::DeviceError::PingError(e),
                    //         )
                    //     })?;
                    //     CommonProperties {
                    //         device_information,
                    //         protocol_version,
                    //     }
                    // }
                    // DeviceType::Ping360(_) | DeviceType::Null => unreachable!(),
                    _ => unreachable!(),
                };
                Some(DeviceProperties::Ping1D(Ping1DProperties { common }))
            }
            DeviceSelection::Ping360 => match &device.device_type {
                DeviceType::Ping360(dev) => {
                    let device_information = dev.device_information().await.map_err(|e| {
                        ManagerError::DeviceError(crate::device::devices::DeviceError::PingError(e))
                    })?;
                    let protocol_version = dev.protocol_version().await.map_err(|e| {
                        ManagerError::DeviceError(crate::device::devices::DeviceError::PingError(e))
                    })?;

                    let device_data = dev.device_data().await.map_err(|e| {
                        ManagerError::DeviceError(crate::device::devices::DeviceError::PingError(e))
                    })?;

                    let auto_transmit = Ping360Config {
                        mode: device_data.mode,
                        gain_setting: device_data.gain_setting,
                        transmit_duration: device_data.transmit_duration,
                        sample_period: device_data.sample_period,
                        transmit_frequency: device_data.transmit_frequency,
                        number_of_samples: 1200,
                        start_angle: 0,
                        stop_angle: 399,
                        num_steps: 1,
                        delay: 0,
                    };

                    Some(DeviceProperties::Ping360(Ping360Properties {
                        common: CommonProperties {
                            device_information,
                            protocol_version,
                        },
                        continuous_mode_settings: std::sync::Arc::new(std::sync::RwLock::new(
                            auto_transmit,
                        )),
                    }))
                }
                // DeviceType::Common(dev) => {
                //     let device_information = dev.device_information().await.map_err(|e| {
                //         ManagerError::DeviceError(crate::device::devices::DeviceError::PingError(e))
                //     })?;
                //     let protocol_version = dev.protocol_version().await.map_err(|e| {
                //         ManagerError::DeviceError(crate::device::devices::DeviceError::PingError(e))
                //     })?;
                //     Some(DeviceProperties::Common(CommonProperties {
                //         device_information,
                //         protocol_version,
                //     }))
                // }
                // DeviceType::Ping1D(_) | DeviceType::Null => unreachable!(),
                _ => unreachable!(),
            },
            DeviceSelection::Auto => None,
        };

        let device = DeviceInfo {
            id,
            source,
            status: DeviceStatus::Available,
            device_type,
            properties,
        };

        Ok(device)
    }
}

pub struct DeviceDiscoveryManager {
    tx: broadcast::Sender<DeviceInfo>,
    handle: Option<tokio::task::JoinHandle<()>>,
    known_devices_rx: broadcast::Receiver<Vec<DeviceInfo>>,
}

impl DeviceDiscoveryManager {
    pub fn new(
        known_devices_rx: broadcast::Receiver<Vec<DeviceInfo>>,
    ) -> (Self, broadcast::Receiver<DeviceInfo>) {
        let (tx, rx) = broadcast::channel(10);
        (
            Self {
                tx,
                handle: None,
                known_devices_rx,
            },
            rx,
        )
    }

    pub fn start_discovery(&mut self) {
        let tx = self.tx.clone();
        let mut known_devices_rx = self.known_devices_rx.resubscribe();

        let handle = tokio::spawn(async move {
            let mut known_devices = Vec::new();
            let mut device_keys = HashSet::new();

            loop {
                match known_devices_rx.try_recv() {
                    Ok(devices) => {
                        known_devices = devices;
                        device_keys.clear();
                        for device in &known_devices {
                            let key = get_device_key(&device.source);
                            device_keys.insert(key);
                        }
                    }
                    Err(tokio::sync::broadcast::error::TryRecvError::Empty) => {}
                    Err(e) => {
                        error!("Error receiving known devices update: {e}");
                        continue;
                    }
                }

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

                if let Some(result) = device_discovery::network_discovery() {
                    for source in result {
                        let key = get_device_key(&source);
                        if !device_keys.contains(&key) {
                            available_sources.push(source);
                        }
                    }
                }

                #[cfg(not(feature = "blueos-extension"))]
                let used_ports: Vec<String> = known_devices
                    .iter()
                    .filter_map(|device| {
                        if let SourceSelection::SerialStream(serial) = &device.source {
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
                    trace!("Attempting to create device for source: {}", key);

                    match DeviceFactory::create_device(source.clone(), DeviceSelection::Auto).await
                    {
                        Ok(device_info) => {
                            trace!("Created new device: {} -> {:?}", key, device_info);
                            let _ = tx.send(device_info);
                        }
                        Err(err) => {
                            error!("Failed to create device {}: {:?}", key, err);
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
    }
}

impl Drop for DeviceDiscoveryManager {
    fn drop(&mut self) {
        self.stop_discovery();
    }
}

pub struct DiscoveryComponent {
    manager: DeviceDiscoveryManager,
    rx: broadcast::Receiver<DeviceInfo>,
    known_devices_tx: broadcast::Sender<Vec<DeviceInfo>>,
}

impl Default for DiscoveryComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl DiscoveryComponent {
    pub fn new() -> Self {
        let (known_devices_tx, known_devices_rx) = broadcast::channel(1);
        let (manager, rx) = DeviceDiscoveryManager::new(known_devices_rx);

        Self {
            manager,
            rx,
            known_devices_tx,
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

    pub fn broadcast_known_devices(&self, device_ids: &[DeviceInfo]) {
        let _ = self.known_devices_tx.send(device_ids.to_owned());
    }

    pub fn get_discovery_rx(&self) -> broadcast::Receiver<DeviceInfo> {
        self.rx.resubscribe()
    }
}
