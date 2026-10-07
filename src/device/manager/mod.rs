/// Specially for DeviceManager to retrieve checks and structures from Devices stored in it's hashmap collection
pub mod continuous_mode;
/// Specially for auto creation methods, from UDP or serial port
pub mod device_discovery;
/// Specially for continuous_mode methods, startup, shutdown, handle and errors routines for each device type
pub mod device_handle;
/// Specially for DeviceManager, allow discovery service to run on background
pub mod discovery_service;
/// Specially for DeviceManager, handles device reconnections with a stable identifier (slot).
pub mod slots;

use paperclip::actix::Apiv2Schema;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt::Display,
    hash::Hash,
    net::{Ipv4Addr, SocketAddrV4, UdpSocket},
    sync::{atomic::AtomicU16, Arc, RwLock},
    time::{Duration, Instant},
};
use tokio::{
    sync::{broadcast::Receiver, mpsc, oneshot},
    time::sleep,
};

use tokio_serial::{SerialPort, SerialPortBuilderExt, SerialStream};
use tracing::{debug, error, info, trace, warn};
use udp_stream::UdpStream;

use crate::device::manager::slots::{DeviceConnection, DeviceIdentity, SlotRegistry};

use super::{
    devices::{DeviceActor, DeviceActorHandler, DeviceType, PingAnswer},
    fake::FakeStream,
};
use bluerobotics_ping::{
    common::{DeviceInformationStruct, ProtocolVersionStruct},
    device::{Ping1D, Ping360},
    message::ProtocolMessage,
};
use discovery_service::DiscoveryComponent;
#[derive(Debug)]
pub struct Device {
    pub slot: u8,
    pub source: SourceSelection,
    pub handler: Option<super::devices::DeviceActorHandler>,
    pub actor: Option<tokio::task::JoinHandle<DeviceActor>>,
    pub broadcast: Option<tokio::task::JoinHandle<()>>,
    pub status: DeviceStatus,
    pub device_type: DeviceSelection,
    pub properties: Option<DeviceProperties>,
    pub recover_attempts: u32,
    // Next instant to attempt recovery at
    pub next_recover_at: Option<Instant>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum DeviceProperties {
    Common(CommonProperties),
    Ping1D(Ping1DProperties),
    Ping360(Ping360Properties),
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Apiv2Schema)]
pub struct Ping360Config {
    pub mode: u8,
    pub gain_setting: u8,
    pub transmit_duration: u16,
    pub sample_period: u16,
    pub transmit_frequency: u16,
    pub number_of_samples: u16,
    pub start_angle: u16,
    pub stop_angle: u16,
    pub num_steps: u8,
    pub delay: u8,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CommonProperties {
    pub device_information: DeviceInformationStruct,
    pub protocol_version: ProtocolVersionStruct,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Ping1DProperties {
    pub common: CommonProperties,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Ping360Properties {
    pub common: CommonProperties,
    pub continuous_mode_settings: Arc<RwLock<Ping360Config>>,
}

impl Ping360Properties {
    fn supports_auto_transmit(&self) -> bool {
        self.common.device_information.firmware_version_major > 3
            || (self.common.device_information.firmware_version_major == 3
                && self.common.device_information.firmware_version_minor >= 3)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeviceInfo {
    pub slot: u8,
    pub source: SourceSelection,
    pub status: DeviceStatus,
    pub device_type: DeviceSelection,
    pub properties: Option<DeviceProperties>,
}

impl Device {
    pub fn info(&self) -> DeviceInfo {
        DeviceInfo {
            slot: self.slot,
            source: self.source.clone(),
            status: self.status.clone(),
            device_type: self.device_type,
            properties: self.properties.clone(),
        }
    }

    fn reset_recover_backoff(&mut self) {
        self.recover_attempts = 0;
        self.next_recover_at = None;
    }

    fn schedule_recover_backoff(&mut self) {
        self.recover_attempts = self.recover_attempts.saturating_add(1);
        self.next_recover_at = Some(Instant::now() + Duration::from_secs(10));
    }

    fn due_for_recover(&self) -> bool {
        self.next_recover_at
            .map(|at| Instant::now() >= at)
            .unwrap_or(true)
    }

    fn mark_error(&mut self, reason: impl Into<String>) {
        self.status = DeviceStatus::Error(reason.into());
        if self.next_recover_at.is_none() {
            self.next_recover_at = Some(Instant::now() + Duration::from_secs(10));
        }
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        let info = self.info();
        trace!(?info, "Removing Device from DeviceManager",);
        if let Some(handle) = self.actor.take() {
            handle.abort();
        }
        if let Some(broadcast_handle) = &self.broadcast {
            trace!(device_id = %self.device_type, slot = self.slot, "Device broadcast handle closed");
            broadcast_handle.abort();
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Apiv2Schema)]
pub enum DeviceSelection {
    Common,
    Ping1D,
    Ping360,
    Auto,
}

impl Display for DeviceSelection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            DeviceSelection::Common => "Common",
            DeviceSelection::Ping1D => "Ping1D",
            DeviceSelection::Ping360 => "Ping360",
            DeviceSelection::Auto => "Auto",
        })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Hash, Apiv2Schema, PartialEq, Eq)]
pub enum SourceSelection {
    UdpStream(SourceUdpStruct),
    SerialStream(SourceSerialStruct),
    FakeStream(SourceFakeStruct),
}

enum SourceType {
    Udp(UdpStream),
    Serial(SerialStream),
    Fake(FakeStream),
}

#[derive(Clone, Debug, Deserialize, Serialize, Hash, Apiv2Schema, PartialEq, Eq)]
pub struct SourceUdpStruct {
    pub ip: Ipv4Addr,
    pub port: u16,
    pub mac_address: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, Hash, Apiv2Schema, PartialEq, Eq)]
pub struct SourceSerialStruct {
    pub path: String,
    pub baudrate: u32,
}

// Generate distinct IDs for each fake device
fn get_fake_source_id() -> u16 {
    static FAKE_ID: AtomicU16 = AtomicU16::new(1);
    FAKE_ID.fetch_add(1, std::sync::atomic::Ordering::AcqRel)
}

#[derive(Clone, Debug, Deserialize, Serialize, Hash, Apiv2Schema, PartialEq, Eq)]
pub struct SourceFakeStruct {
    #[serde(skip, default = "get_fake_source_id")]
    fake_id: u16,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeviceStatus {
    Available,
    Running,
    Error(String),
    ContinuousMode,
}

pub struct DeviceManager {
    receiver: mpsc::Receiver<ManagerActorRequest>,
    pub device: HashMap<SourceSelection, Device>,
    slot_registry: SlotRegistry,
    discovery_service: DiscoveryComponent,
    pub manager_handler: ManagerActorHandler,
}

#[derive(Debug)]
pub struct ManagerActorRequest {
    pub request: Request,
    pub respond_to: oneshot::Sender<Result<Answer, ManagerError>>,
}
#[derive(Clone)]
pub struct ManagerActorHandler {
    pub sender: mpsc::Sender<ManagerActorRequest>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Apiv2Schema)]
pub enum Answer {
    DeviceMessage(DeviceAnswer),
    #[serde(skip)]
    InnerDeviceHandler(DeviceActorHandler),
    DeviceInfo(Vec<DeviceInfo>),
    DeviceConfig(ModifyDeviceResult),
    #[serde(skip)]
    Shutdown,
}

#[derive(Debug, Serialize, Deserialize, Clone, thiserror::Error)]
pub enum ManagerError {
    #[error("Device {0}/{1} is not registered")]
    DeviceNotExist(DeviceSelection, u8),
    #[error("Device {0}/{1} is already registered")]
    DeviceAlreadyExist(DeviceSelection, u8),
    #[error("Device {1}/{2} is {0}")]
    DeviceStatus(DeviceStatus, DeviceSelection, u8),
    #[error("{0}")]
    DeviceError(super::devices::DeviceError),
    #[error("Could not open the device source: {0}")]
    DeviceSourceError(String),
    #[error("No devices available")]
    NoDevices,
    #[error("Internal request channel failed: {0}")]
    TokioMpsc(String),
    #[error("Request is not implemented: {0:?}")]
    NotImplemented(Request),
    #[error("{0}")]
    Other(String),
}

impl std::fmt::Display for DeviceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeviceStatus::Available => write!(f, "available"),
            DeviceStatus::Running => write!(f, "running"),
            DeviceStatus::ContinuousMode => write!(f, "streaming"),
            DeviceStatus::Error(reason) => write!(f, "in error state ({reason})"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeviceAnswer {
    #[serde(flatten)]
    pub answer: crate::device::devices::PingAnswer,
    pub device_type: DeviceSelection,
    pub slot: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, Apiv2Schema)]
#[serde(tag = "command", content = "payload")]
pub enum Request {
    AutoCreate,
    Create(CreateStruct),
    Delete(DeviceSlotWrapper),
    List,
    Info(DeviceSlotWrapper),
    Search,
    Ping(DeviceRequestStruct),
    GetDeviceHandler(DeviceSlotWrapper),
    ModifyDevice(ModifyDevice),
    EnableContinuousMode(DeviceSlotWrapper),
    DisableContinuousMode(DeviceSlotWrapper),
    #[serde(skip)]
    Shutdown,
    #[serde(skip)]
    SpecialTurnOffContinuousMode(DeviceSlotWrapper),
}

#[derive(Debug, Clone, Serialize, Deserialize, Apiv2Schema)]
pub enum ModifyDeviceCommand {
    SetIp(Ipv4Addr),
    SetPing360Config(Ping360Config),
    GetPing360Config,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ModifyDeviceResult {
    ConfigAcknowledge(ModifyDevice),
    Ping360Config(Ping360Config),
}

#[derive(Debug, Clone, Serialize, Deserialize, Apiv2Schema)]
pub struct ModifyDevice {
    pub device_type: DeviceSelection,
    pub slot: u8,
    pub modify: ModifyDeviceCommand,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Apiv2Schema)]
pub struct DeviceSlotWrapper {
    pub device_type: DeviceSelection,
    pub slot: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, Apiv2Schema)]
pub struct CreateStruct {
    pub source: SourceSelection,
    pub device_selection: DeviceSelection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceRequestStruct {
    pub device_type: DeviceSelection,
    pub slot: u8,
    pub device_request: crate::device::devices::PingRequest,
}

impl DeviceManager {
    async fn handle_message(&mut self, actor_request: ManagerActorRequest) {
        trace!(?actor_request, "DeviceManager: Received a request");
        match actor_request.request {
            Request::AutoCreate => {
                let result = self.auto_create().await;
                if let Err(error) = actor_request.respond_to.send(result) {
                    error!(
                        ?error,
                        "DeviceManager: Failed to return AutoCreate response"
                    );
                }
            }
            Request::Create(request) => {
                let result = self.create(request.source, request.device_selection).await;
                if let Err(error) = actor_request.respond_to.send(result) {
                    error!(?error, "DeviceManager: Failed to return Create response");
                }
            }
            Request::SpecialTurnOffContinuousMode(DeviceSlotWrapper { device_type, slot }) => {
                let result = self
                    .turnoff_device_on_continuous_mode(device_type, slot)
                    .await;
                if let Err(error) = actor_request.respond_to.send(result) {
                    error!(
                        ?error,
                        "DeviceManager: Failed to return SpecialTurnOffContinuousMode response"
                    );
                }
            }
            Request::Delete(DeviceSlotWrapper { device_type, slot }) => {
                let result = self.delete(device_type, slot).await;
                if let Err(error) = actor_request.respond_to.send(result) {
                    error!(?error, "DeviceManager: Failed to return Delete response");
                }
            }
            Request::List => {
                let result = self.list().await;
                if let Err(error) = actor_request.respond_to.send(result) {
                    error!(?error, "DeviceManager: Failed to return List response");
                }
            }
            Request::Info(DeviceSlotWrapper { device_type, slot }) => {
                let result = self.info(device_type, slot).await;
                if let Err(error) = actor_request.respond_to.send(result) {
                    error!(?error, "DeviceManager: Failed to return Info response");
                }
            }
            Request::EnableContinuousMode(DeviceSlotWrapper { device_type, slot }) => {
                let result = self.continuous_mode(device_type, slot).await;
                if let Err(error) = actor_request.respond_to.send(result) {
                    error!(
                        ?error,
                        "DeviceManager: Failed to return EnableContinuousMode response"
                    );
                }
            }
            Request::DisableContinuousMode(DeviceSlotWrapper { device_type, slot }) => {
                let result = self.continuous_mode_off(device_type, slot).await;
                if let Err(error) = actor_request.respond_to.send(result) {
                    error!(
                        ?error,
                        "DeviceManager: Failed to return DisableContinuousMode response"
                    );
                }
            }
            Request::GetDeviceHandler(DeviceSlotWrapper { device_type, slot }) => {
                let answer = self.get_device_handler(device_type, slot).await;
                if let Err(error) = actor_request.respond_to.send(answer) {
                    error!(
                        ?error,
                        "DeviceManager: Failed to return GetDeviceHandler response"
                    );
                }
            }
            Request::ModifyDevice(request) => {
                let answer = self.modify_device(request).await;
                if let Err(error) = actor_request.respond_to.send(answer) {
                    error!(
                        ?error,
                        "DeviceManager: Failed to return ModifyDevice response"
                    );
                }
            }
            Request::Search | Request::Ping(_) => {
                if let Err(error) = actor_request
                    .respond_to
                    .send(Err(ManagerError::NotImplemented(actor_request.request)))
                {
                    warn!(?error, "DeviceManager: Failed to return response");
                }
            }
            Request::Shutdown => {
                self.shutdown().await;
                if let Err(e) = actor_request.respond_to.send(Ok(Answer::Shutdown)) {
                    warn!("DeviceManager: Failed to return response: {e:?}");
                }
            }
        }
    }

    pub fn new(size: usize) -> (Self, ManagerActorHandler) {
        let (sender, receiver) = mpsc::channel(size);

        let actor_handler = ManagerActorHandler { sender };
        let actor = DeviceManager {
            receiver,
            device: HashMap::new(),
            slot_registry: SlotRegistry::new(),
            discovery_service: DiscoveryComponent::new(),
            manager_handler: actor_handler.clone(),
        };

        trace!("DeviceManager and handler successfully created: Success");
        (actor, actor_handler)
    }

    pub fn get_device_manager_handler(&self) -> ManagerActorHandler {
        self.manager_handler.clone()
    }

    pub async fn run(mut self) {
        info!("DeviceManager is running");

        self.discovery_service.start_discovery();

        if let Ok(sources) = self.sources().await {
            self.discovery_service.broadcast_known_sources(sources);
        }

        let mut discovery_rx = self.discovery_service.get_discovery_rx();

        let mut status_check_interval = tokio::time::interval(std::time::Duration::from_secs(10));

        loop {
            tokio::select! {
                Some(msg) = self.receiver.recv() => {
                    self.handle_message(msg).await;
                }
                Ok(device_discovery_info) = discovery_rx.recv() => {
                    let slot = self.alloc_slot(&DeviceIdentity {
                        device_type: device_discovery_info.device_type,
                        connection: DeviceConnection::from(&device_discovery_info.source),
                    }).await;
                    let device_info = DeviceInfo {
                        slot,
                        source: device_discovery_info.source,
                        status: device_discovery_info.status,
                        device_type: device_discovery_info.device_type,
                        properties: device_discovery_info.properties
                    };
                    match self.register_device(device_info.clone()).await {
                        Ok(_) => {
                            info!(
                                device_type = ?device_discovery_info.device_type,
                                slot,
                                "New device registered"
                            );

                            if crate::cli::manager::is_enable_auto_create() {
                                if let Err(error) = self.auto_create_device(device_info.device_type, device_info.slot).await {
                                    error!(device_type = ?device_info.device_type, slot, ?error, "Failed to auto create discovered device");
                                    if let Ok(device) = self.get_mut_device(device_info.device_type, device_info.slot) {
                                        device.schedule_recover_backoff();
                                    }
                                }
                            }
                        }
                        Err(error) => {
                            error!(?error, "Failed to register discovered device");
                            if self.get_device(device_info.device_type, slot).is_err() {
                                self.slot_registry.release(device_info.device_type, slot);
                            }
                        }
                    }
                }
                _ = status_check_interval.tick() => {
                    debug!("Running scheduled device status check");
                    self.update_devices_status().await;
                }
                else => break,
            }
        }

        error!("DeviceManager has stopped, please check your application");
    }

    pub async fn update_devices_status(&mut self) {
        let device_info = match self.list().await {
            Ok(Answer::DeviceInfo(answer)) => answer,
            _ => return,
        };

        let auto_create = crate::cli::manager::is_enable_auto_create();
        let mut recover_ids = Vec::new();
        let mut available_ids = Vec::new();

        for device in &device_info {
            match &device.status {
                DeviceStatus::Error(_) => {
                    if self
                        .get_device(device.device_type, device.slot)
                        .map(|entry| entry.due_for_recover())
                        .unwrap_or(false)
                    {
                        recover_ids.push((device.device_type, device.slot));
                    }
                }
                DeviceStatus::Available
                    if auto_create
                        && self
                            .get_device(device.device_type, device.slot)
                            .map(|entry| entry.actor.is_none() && entry.due_for_recover())
                            .unwrap_or(false) =>
                {
                    available_ids.push((device.device_type, device.slot));
                }
                _ => {}
            }
        }

        for (device_type, slot) in recover_ids {
            if let Err(error) = self.recover_device(device_type, slot).await {
                error!(
                    %device_type,
                    slot,
                    ?error,
                    "Auto-heal failed"
                );
            }
        }

        for (device_type, slot) in available_ids {
            match self.auto_create_device(device_type, slot).await {
                Ok(_) => {
                    if let Ok(device) = self.get_mut_device(device_type, slot) {
                        device.reset_recover_backoff();
                    }
                }
                Err(error) => {
                    error!(
                        %device_type,
                        slot,
                        ?error,
                        "Failed to auto create available device"
                    );
                    if let Ok(device) = self.get_mut_device(device_type, slot) {
                        device.schedule_recover_backoff();
                    }
                }
            }
        }

        let device_info = match self.list().await {
            Ok(Answer::DeviceInfo(answer)) => answer,
            _ => return,
        };

        for device in device_info {
            if matches!(
                device.status,
                DeviceStatus::Error(_) | DeviceStatus::Available
            ) {
                continue;
            }

            debug!(
                device_type = %device.device_type,
                slot = device.slot,
                "Device Manager is checking"
            );

            let receiver = match self.get_subscriber(device.device_type, device.slot).await {
                Ok(receiver) => receiver,
                Err(error) => {
                    error!(
                        device_type = %device.device_type,
                        slot = device.slot,
                        ?error,
                        "Device connection error, can't take subscriber"
                    );
                    if let Ok(entry) = self.get_mut_device(device.device_type, device.slot) {
                        entry.mark_error(format!("Could not subscribe to the device: {error}"));
                    }
                    continue;
                }
            };

            let device_entry = match self.get_mut_device(device.device_type, device.slot) {
                Ok(entry) => entry,
                Err(error) => {
                    error!(
                        ?error,
                        device_type = %device.device_type,
                        slot = device.slot,
                        "Device Manager can't get device"
                    );
                    continue;
                }
            };

            if let Some(handle) = &device_entry.actor {
                if handle.is_finished() {
                    error!(
                        device_type = %device.device_type,
                        slot = device.slot,
                        "Device Actor main task finished, marking device with error",
                    );
                    device_entry.mark_error("Device Actor main task finished");
                    continue;
                }
            }

            match &device_entry.status {
                DeviceStatus::ContinuousMode => {
                    DeviceManager::check_continuous_mode_device(device_entry, receiver).await;
                }
                DeviceStatus::Running => {
                    DeviceManager::check_running_device(device_entry).await;
                }
                status => {
                    error!(
                        ?status,
                        device_type = %device.device_type,
                        slot = device.slot,
                        "Device Manager found an unhandled device status"
                    );
                    continue;
                }
            }
        }
    }

    async fn check_continuous_mode_device(
        device_entry: &mut Device,
        mut receiver: Receiver<ProtocolMessage>,
    ) {
        let Some(broadcast) = &device_entry.broadcast else {
            error!(
                device_type = %device_entry.device_type,
                slot = device_entry.slot,
                "Device actor broadcast service finished, marking device with error"
            );
            device_entry.mark_error("Device actor broadcast service finished");
            return;
        };

        if broadcast.is_finished() {
            error!(
                device_type = %device_entry.device_type,
                slot = device_entry.slot,
                "Device actor broadcast service finished, marking device with error"
            );
            device_entry.mark_error("Device actor broadcast service finished");
            return;
        }

        match &device_entry.device_type {
            DeviceSelection::Common | DeviceSelection::Ping1D | DeviceSelection::Ping360 => {
                match tokio::time::timeout(std::time::Duration::from_secs(15), receiver.recv())
                    .await
                {
                    Err(_) => {
                        error!(
                            device_type = %device_entry.device_type,
                            slot = device_entry.slot,
                            "Device connection timeout, marking with error",
                        );
                        device_entry.mark_error("Device connection timeout");
                    }
                    Ok(Err(error)) => match error {
                        tokio::sync::broadcast::error::RecvError::Lagged(_) => error!(
                            device_type = %device_entry.device_type,
                            slot = device_entry.slot,
                            ?error, "Device connection error",
                        ),
                        tokio::sync::broadcast::error::RecvError::Closed => {
                            error!(
                                device_type = %device_entry.device_type,
                                slot = device_entry.slot,
                                ?error, "Device connection error, marking with error",
                            );
                            device_entry.mark_error("Device connection error");
                        }
                    },
                    Ok(Ok(_)) => {
                        debug!(
                            device_type = %device_entry.device_type,
                            slot = device_entry.slot,
                            "Device still responsive"
                        );
                    }
                }
            }
            device_selection => {
                error!(
                    device_type = %device_entry.device_type,
                    slot = device_entry.slot,
                    "Device connection error, Cannot check health of {device_selection:?}"
                );
            }
        }
    }

    async fn check_running_device(device_entry: &mut Device) {
        let Some(handler) = &device_entry.handler else {
            error!(
                device_type = %device_entry.device_type,
                slot = device_entry.slot,
                "Device handler missing, marking with error",
            );
            device_entry.mark_error("Device handler missing");
            return;
        };

        let handler_clone = handler.clone();

        match tokio::time::timeout(
            std::time::Duration::from_millis(2000),
            handler_clone.send(super::devices::PingRequest::Common(
                super::devices::PingCommonRequest::DeviceInformation,
            )),
        )
        .await
        {
            Err(_) => {
                error!(
                    device_type = %device_entry.device_type,
                    slot = device_entry.slot,
                    "Device connection timeout, marking with error",
                );
                device_entry.mark_error("Device connection timeout");
            }
            Ok(Err(error)) => {
                error!(
                    device_type = %device_entry.device_type,
                    slot = device_entry.slot,
                    ?error,
                    "Device connection error, marking with error",
                );
                device_entry.mark_error("Device connection error");
            }
            Ok(Ok(_)) => {
                debug!(
                    device_type = %device_entry.device_type,
                    slot = device_entry.slot,
                    "Device still responsive"
                );
            }
        }
    }

    pub async fn create(
        &mut self,
        source: SourceSelection,
        mut device_selection: DeviceSelection,
    ) -> Result<Answer, ManagerError> {
        if let Some(device) = self.device.get(&source) {
            trace!(
                ?source,
                "Device creation error: Device already exist for provided SourceSelection"
            );
            return Err(ManagerError::DeviceAlreadyExist(
                device.device_type,
                device.slot,
            ));
        }

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

                #[cfg(unix)]
                serial_stream
                    .set_exclusive(true)
                    .map_err(|err| ManagerError::DeviceSourceError(err.to_string()))?;

                SourceType::Serial(serial_stream)
            }
            SourceSelection::FakeStream(_) => SourceType::Fake(FakeStream::new(device_selection)),
        };

        let device = match port {
            SourceType::Udp(udp_port) => match device_selection {
                DeviceSelection::Common | DeviceSelection::Auto => {
                    crate::device::devices::DeviceType::Common(
                        bluerobotics_ping::common::Device::new(udp_port),
                    )
                }
                DeviceSelection::Ping1D => {
                    crate::device::devices::DeviceType::Ping1D(Ping1D::new(udp_port))
                }
                DeviceSelection::Ping360 => {
                    crate::device::devices::DeviceType::Ping360(Ping360::new(udp_port))
                }
            },
            SourceType::Serial(serial_port) => match device_selection {
                DeviceSelection::Common | DeviceSelection::Auto => {
                    crate::device::devices::DeviceType::Common(
                        bluerobotics_ping::common::Device::new(serial_port),
                    )
                }
                DeviceSelection::Ping1D => {
                    crate::device::devices::DeviceType::Ping1D(Ping1D::new(serial_port))
                }
                DeviceSelection::Ping360 => {
                    crate::device::devices::DeviceType::Ping360(Ping360::new(serial_port))
                }
            },
            SourceType::Fake(fake_port) => match device_selection {
                DeviceSelection::Common | DeviceSelection::Auto => {
                    crate::device::devices::DeviceType::Common(
                        bluerobotics_ping::common::Device::new(fake_port),
                    )
                }
                DeviceSelection::Ping1D => {
                    crate::device::devices::DeviceType::Ping1D(Ping1D::new(fake_port))
                }
                DeviceSelection::Ping360 => {
                    crate::device::devices::DeviceType::Ping360(Ping360::new(fake_port))
                }
            },
        };

        let (mut device, handler) = super::devices::DeviceActor::new(device, 10);

        if device_selection == DeviceSelection::Auto {
            let mut retry_count = 0;
            let max_retries = 3;
            let retry_delay = Duration::from_millis(100);

            loop {
                match device.try_upgrade().await {
                    Ok(super::devices::PingAnswer::UpgradeResult(result)) => {
                        match result {
                            super::devices::UpgradeResult::Unknown => {
                                device_selection = DeviceSelection::Common;
                            }
                            super::devices::UpgradeResult::Ping1D => {
                                device_selection = DeviceSelection::Ping1D;
                            }
                            super::devices::UpgradeResult::Ping360 => {
                                device_selection = DeviceSelection::Ping360;
                            }
                        }
                        break;
                    }
                    Err(error) => {
                        retry_count += 1;
                        if retry_count >= max_retries {
                            error!(
                                ?error,
                                "Device creation error: Can't auto upgrade the DeviceType after {} attempts",
                                max_retries
                            );
                            return Err(ManagerError::DeviceError(error));
                        }

                        warn!(
                            ?error,
                            "Device creation error: Device upgrade attempt {} of {} failed. Retrying...",
                            retry_count, max_retries
                        );

                        sleep(retry_delay).await;
                        continue;
                    }
                    error => warn!(?error, "Device creation error: Abnormal answer"),
                }
            }
        }

        let actor = tokio::spawn(async move { device.run().await });

        let slot = self
            .alloc_slot(&DeviceIdentity {
                device_type: device_selection,
                connection: DeviceConnection::from(&source),
            })
            .await;

        let device = Device {
            slot,
            source: source.clone(),
            handler: Some(handler),
            actor: Some(actor),
            status: DeviceStatus::Running,
            broadcast: None,
            device_type: device_selection,
            properties: None,
            recover_attempts: 0,
            next_recover_at: None,
        };

        let Device {
            device_type, slot, ..
        } = device;

        self.device.insert(source.clone(), device);

        trace!(?source, "Updating device properties");
        self.update_device_properties(device_type, slot).await?;

        trace!(?source, "Device broadcast enable by default");
        let device_info = self.continuous_mode(device_type, slot).await?;

        info!(?device_info, "New device created and available");
        Ok(device_info)
    }

    pub async fn auto_create(&mut self) -> Result<Answer, ManagerError> {
        let mut results = Vec::new();
        let mut has_errors = false;

        let available_device_info: Vec<(u8, SourceSelection, DeviceSelection)> = self
            .device
            .values()
            .filter_map(|device| {
                if device.status == DeviceStatus::Available {
                    Some((device.slot, device.source.clone(), device.device_type))
                } else {
                    None
                }
            })
            .collect();

        if available_device_info.is_empty() {
            warn!("Auto create: No available devices found");
            return Ok(Answer::DeviceInfo(vec![]));
        }

        for (slot, source, device_type) in available_device_info {
            match self.create_device_helper(device_type, slot, source).await {
                Ok(device_info) => {
                    trace!(%device_type, slot, "Successfully created device");
                    results.push(device_info);
                }
                Err(error) => {
                    error!(%device_type, slot, ?error, "Failed to create device");
                    has_errors = true;
                    continue;
                }
            }
        }

        if !results.is_empty() {
            Ok(Answer::DeviceInfo(results))
        } else if has_errors {
            Err(ManagerError::Other(
                "Failed to create any devices".to_string(),
            ))
        } else {
            Ok(Answer::DeviceInfo(vec![]))
        }
    }

    pub async fn auto_create_device(
        &mut self,
        device_type: DeviceSelection,
        slot: u8,
    ) -> Result<Answer, ManagerError> {
        let device = self.get_device(device_type, slot)?;

        self.check_device_status(device, &[DeviceStatus::Available])?;

        let source = device.source.clone();

        let device_info = Box::pin(self.create_device_helper(device_type, slot, source)).await?;

        Ok(Answer::DeviceInfo(vec![device_info]))
    }

    async fn create_device_helper(
        &mut self,
        device_type: DeviceSelection,
        slot: u8,
        source: SourceSelection,
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
            SourceSelection::FakeStream(_) => SourceType::Fake(FakeStream::new(device_type)),
        };

        let device_type_inner = match port {
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

        let (device_actor, handler) = super::devices::DeviceActor::new(device_type_inner, 10);
        let actor = tokio::spawn(async move { device_actor.run().await });

        let device = self.get_mut_device(device_type, slot)?;
        device.handler = Some(handler.clone());
        device.actor = Some(actor);
        device.status = DeviceStatus::Running;

        match self.continuous_mode(device_type, slot).await {
            Ok(_) => {
                trace!(%device_type, slot, "Successfully enabled continuous mode");
            }
            Err(error) => {
                error!(%device_type, slot, ?error, "Failed to enable continuous mode",);
                self.stop_then_teardown_device_runtime(device_type, slot)
                    .await?;
                let device = self.get_mut_device(device_type, slot)?;
                device.mark_error(error.to_string());
                device.schedule_recover_backoff();
                return Err(error);
            }
        }

        match self.get_device(device_type, slot) {
            Ok(device) => Ok(device.info()),
            Err(error) => {
                error!(%device_type, slot, ?error, "Failed to get device info");
                Err(error)
            }
        }
    }

    pub async fn list(&self) -> Result<Answer, ManagerError> {
        if self.device.is_empty() {
            trace!("No devices available for list generation request");
            return Ok(Answer::DeviceInfo(Vec::new()));
        };
        let mut list = Vec::new();
        for device in self.device.values() {
            list.push(device.info())
        }
        Ok(Answer::DeviceInfo(list))
    }

    pub async fn sources(&self) -> Result<Vec<SourceSelection>, ManagerError> {
        if self.device.is_empty() {
            trace!("No devices available for list generation request");
            return Ok(Vec::new());
        };
        let mut list = Vec::new();
        for source in self.device.keys() {
            list.push(source.clone())
        }
        Ok(list)
    }

    pub async fn info(
        &self,
        device_type: DeviceSelection,
        slot: u8,
    ) -> Result<Answer, ManagerError> {
        Ok(Answer::DeviceInfo(vec![self
            .get_device(device_type, slot)?
            .info()]))
    }

    async fn alloc_slot(&mut self, identity: &DeviceIdentity) -> u8 {
        if let Some(slot) = self.slot_registry.assigned(identity) {
            let errored = self
                .get_device(identity.device_type, slot)
                .is_ok_and(|device| matches!(device.status, DeviceStatus::Error(_)));
            if errored {
                info!(
                    device_type = %identity.device_type,
                    slot,
                    "Removing errored device to reuse its slot"
                );
                if let Err(error) = self.delete(identity.device_type, slot).await {
                    warn!(
                        device_type = %identity.device_type,
                        slot,
                        ?error,
                        "Failed to remove errored device before reusing its slot"
                    );
                }
            }
        }

        self.slot_registry.resolve(identity)
    }

    pub async fn register_device(
        &mut self,
        device_info: DeviceInfo,
    ) -> Result<Answer, ManagerError> {
        if let Ok(device) = self.get_device(device_info.device_type, device_info.slot) {
            error!(
                device_type = %device_info.device_type,
                slot = device_info.slot,
                "Device register: Error, device already exists"
            );
            return Err(ManagerError::DeviceAlreadyExist(
                device.device_type,
                device.slot,
            ));
        }

        let device = Device {
            slot: device_info.slot,
            source: device_info.source.clone(),
            handler: None,
            actor: None,
            status: DeviceStatus::Available,
            broadcast: None,
            device_type: device_info.device_type,
            properties: device_info.properties,
            recover_attempts: 0,
            next_recover_at: None,
        };

        let info = device.info();

        self.device.insert(device_info.source, device);

        if let Ok(sources) = self.sources().await {
            self.discovery_service.broadcast_known_sources(sources);
        }

        Ok(Answer::DeviceInfo(vec![info]))
    }

    pub async fn turnoff_device_on_continuous_mode(
        &mut self,
        device_type: DeviceSelection,
        slot: u8,
    ) -> Result<Answer, ManagerError> {
        let device = self.get_device(device_type, slot)?;

        self.check_device_status(device, &[DeviceStatus::ContinuousMode])?;

        let source = device.source.clone();
        turnoff_device_continuous_mode(&source).await?;

        sleep(Duration::from_millis(500)).await;

        let device_info = self.list().await?;
        info!(?device_info, "Device successfully conclude turnoff process",);
        Ok(device_info)
    }

    pub fn teardown_device_runtime(
        &mut self,
        device_type: DeviceSelection,
        slot: u8,
    ) -> Result<(), ManagerError> {
        let device = self.get_mut_device(device_type, slot)?;
        if let Some(handle) = device.actor.take() {
            handle.abort();
        }
        if let Some(broadcast) = device.broadcast.take() {
            broadcast.abort();
        }
        device.handler = None;
        device.status = DeviceStatus::Available;
        Ok(())
    }

    pub async fn stop_then_teardown_device_runtime(
        &mut self,
        device_type: DeviceSelection,
        slot: u8,
    ) -> Result<(), ManagerError> {
        let (handler, device_type, source) = {
            let device = self.get_device(device_type, slot)?;
            (
                device.handler.clone(),
                device.device_type,
                device.source.clone(),
            )
        };

        if let Some(handler) = handler {
            match device_type {
                DeviceSelection::Ping1D => {
                    let id = <bluerobotics_ping::ping1d::ProfileStruct as bluerobotics_ping::message::MessageInfo>::id();
                    if let Err(error) = handler
                        .send(super::devices::PingRequest::Ping1D(
                            super::devices::Ping1DRequest::ContinuousStop(
                                bluerobotics_ping::ping1d::ContinuousStopStruct { id },
                            ),
                        ))
                        .await
                    {
                        warn!(
                            %device_type,
                            slot,
                            ?error,
                            "stop_then_teardown: ContinuousStop failed"
                        );
                    }
                }
                DeviceSelection::Ping360 => {
                    if matches!(source, SourceSelection::SerialStream(_)) {
                        if let Err(error) = handler
                            .send(super::devices::PingRequest::Ping360(
                                super::devices::Ping360Request::MotorOff,
                            ))
                            .await
                        {
                            warn!(%device_type, slot, ?error, "stop_then_teardown: MotorOff failed");
                        }
                    }
                }
                DeviceSelection::Common | DeviceSelection::Auto => {}
            }
        }

        if let Err(error) = turnoff_device_continuous_mode(&source).await {
            warn!(%device_type, slot, ?error, "stop_then_teardown: turnoff failed");
        }
        sleep(Duration::from_millis(500)).await;

        self.teardown_device_runtime(device_type, slot)
    }

    pub async fn recover_device(
        &mut self,
        device_type: DeviceSelection,
        slot: u8,
    ) -> Result<Answer, ManagerError> {
        self.stop_then_teardown_device_runtime(device_type, slot)
            .await?;
        match self.auto_create_device(device_type, slot).await {
            Ok(answer) => {
                self.get_mut_device(device_type, slot)?
                    .reset_recover_backoff();
                Ok(answer)
            }
            Err(error) => {
                error!(%device_type, slot, ?error, "Failed to recover device");
                let device = self.get_mut_device(device_type, slot)?;
                device.mark_error(error.to_string());
                device.schedule_recover_backoff();
                Err(error)
            }
        }
    }

    pub async fn delete(
        &mut self,
        device_type: DeviceSelection,
        slot: u8,
    ) -> Result<Answer, ManagerError> {
        let device = self.remove_device(device_type, slot)?;
        self.slot_registry.release(device_type, slot);
        let device_info = device.info();

        if let Ok(sources) = self.sources().await {
            self.discovery_service.broadcast_known_sources(sources);
        }

        Ok(Answer::DeviceInfo(vec![device_info]))
    }

    pub async fn continuous_mode(
        &mut self,
        device_type: DeviceSelection,
        slot: u8,
    ) -> Result<Answer, ManagerError> {
        match self.get_device(device_type, slot)?.status {
            DeviceStatus::Available => {
                self.auto_create_device(device_type, slot).await?;
                trace!(%device_type, slot, "Successfully created device in continuous_mode");
            }
            DeviceStatus::Error(_) => {
                return self.recover_device(device_type, slot).await;
            }
            DeviceStatus::ContinuousMode | DeviceStatus::Running => {}
        }

        match self.get_device(device_type, slot)?.status.clone() {
            DeviceStatus::ContinuousMode => {
                let updated_device_info = self.get_device(device_type, slot)?.info();
                Ok(Answer::DeviceInfo(vec![updated_device_info]))
            }
            DeviceStatus::Running => {
                // Ensure device properties are initialized before starting continuous mode
                self.update_device_properties(device_type, slot).await?;

                // Get an inner subscriber for device's stream
                let subscriber = self.get_subscriber(device_type, slot).await?;

                let broadcast_handle = self
                    .continuous_mode_start(subscriber, device_type, slot)
                    .await;
                if let Some(handle) = &broadcast_handle {
                    if !handle.is_finished() {
                        trace!(%device_type, slot, "Success start_continuous_mode");
                    } else {
                        return Err(ManagerError::Other(
                            "Error while start_continuous_mode".to_string(),
                        ));
                    }
                } else {
                    return Err(ManagerError::Other(
                        "Error while start_continuous_mode".to_string(),
                    ));
                };

                self.continuous_mode_startup_routine(device_type, slot)
                    .await?;

                let device = self.get_mut_device(device_type, slot)?;
                device.broadcast = broadcast_handle;
                device.status = DeviceStatus::ContinuousMode;

                let updated_device_info = self.get_device(device_type, slot)?.info();

                Ok(Answer::DeviceInfo(vec![updated_device_info]))
            }
            status => Err(ManagerError::DeviceStatus(status, device_type, slot)),
        }
    }

    pub async fn continuous_mode_off(
        &mut self,
        device_type: DeviceSelection,
        slot: u8,
    ) -> Result<Answer, ManagerError> {
        let device = self.get_device(device_type, slot)?;
        self.check_device_status(device, &[DeviceStatus::ContinuousMode])?;

        self.continuous_mode_shutdown_routine(device_type, slot)
            .await?;

        let device = self.get_mut_device(device_type, slot)?;
        if let Some(broadcast) = device.broadcast.take() {
            broadcast.abort_handle().abort();
        }

        device.status = DeviceStatus::Running;

        let updated_device_info = device.info();

        Ok(Answer::DeviceInfo(vec![updated_device_info]))
    }

    pub async fn update_device_properties(
        &mut self,
        device_type: DeviceSelection,
        slot: u8,
    ) -> Result<(), ManagerError> {
        let device = self.get_device(device_type, slot)?;
        self.check_device_status(
            device,
            &[
                DeviceStatus::Running,
                DeviceStatus::ContinuousMode,
                DeviceStatus::Available,
            ],
        )?;

        let handler = self.extract_handler(self.get_device_handler(device_type, slot).await?)?;

        let device = self.get_mut_device(device_type, slot)?;

        let device_information = handler
            .send(super::devices::PingRequest::Common(
                super::devices::PingCommonRequest::DeviceInformation,
            ))
            .await
            .map_err(|error| {
                error!(?error, "Something went wrong while executing properties");
                ManagerError::DeviceError(error)
            })?;
        let protocol_version = handler
            .send(super::devices::PingRequest::Common(
                super::devices::PingCommonRequest::ProtocolVersion,
            ))
            .await
            .map_err(|error| {
                error!(?error, "Something went wrong while executing properties");
                ManagerError::DeviceError(error)
            })?;
        let device_information = match device_information {
            PingAnswer::PingMessage(bluerobotics_ping::Messages::Common(
                bluerobotics_ping::common::Messages::DeviceInformation(msg),
            )) => msg,
            unexpected => {
                return Err(ManagerError::Other(format!(
                    "Unexpected response while getting device information: {unexpected:?}, device type: {device_type}, slot: {slot}"
                )))
            }
        };

        let protocol_version = match protocol_version {
            PingAnswer::PingMessage(bluerobotics_ping::Messages::Common(
                bluerobotics_ping::common::Messages::ProtocolVersion(msg),
            )) => msg,
            unexpected => {
                return Err(ManagerError::Other(format!(
                    "Unexpected response while getting protocol version: {unexpected:?}, device type: {device_type}, slot: {slot}"
                )))
            }
        };

        let common_properties = CommonProperties {
            device_information,
            protocol_version,
        };

        match &device.device_type {
            DeviceSelection::Common => {
                device.properties = Some(DeviceProperties::Common(common_properties))
            }
            DeviceSelection::Ping1D => {
                let ping_1d_properties = Ping1DProperties {
                    common: common_properties,
                };

                device.properties = Some(DeviceProperties::Ping1D(ping_1d_properties))
            }
            DeviceSelection::Ping360 => {
                let device_data = handler
                    .send(super::devices::PingRequest::Ping360(
                        super::devices::Ping360Request::DeviceData,
                    ))
                    .await
                    .map_err(|error| {
                        trace!(?error, "Something went wrong while executing properties");
                        ManagerError::DeviceError(error)
                    })?;

                let device_data = match device_data {
                    PingAnswer::PingMessage(bluerobotics_ping::Messages::Ping360(
                        bluerobotics_ping::ping360::Messages::DeviceData(msg),
                    )) => msg,
                    error => return Err(ManagerError::Other(format!(
                        "properties : Unexpected answer from Ping360 device, slot: {slot}, details: {error:?}"
                    )))
                };

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

                let ping_360_properties = Ping360Properties {
                    common: common_properties,
                    continuous_mode_settings: Arc::new(RwLock::new(auto_transmit)),
                };

                device.properties = Some(DeviceProperties::Ping360(ping_360_properties))
            }
            DeviceSelection::Auto => device.properties = None,
        };

        Ok(())
    }

    async fn get_device_properties(
        &self,
        device_type: DeviceSelection,
        slot: u8,
    ) -> Result<Option<DeviceProperties>, ManagerError> {
        let device = self.get_device(device_type, slot)?;

        if device.properties.is_none() {
            warn!(%device_type, slot, "No properties found for device");
        }

        Ok(device.properties.clone())
    }

    pub async fn update_ping360_config(
        &self,
        device_type: DeviceSelection,
        slot: u8,
        new_config: Ping360Config,
    ) -> Result<(), ManagerError> {
        let device = self.get_device(device_type, slot)?;
        if let Some(DeviceProperties::Ping360(properties)) = &device.properties {
            let mut config = properties
                .continuous_mode_settings
                .write()
                .map_err(|err| ManagerError::Other(err.to_string()))?;
            *config = new_config;
            return Ok(());
        }
        Err(ManagerError::DeviceSourceError(
            "set_ping360_config: Can't set Ping360Config".to_string(),
        ))
    }

    pub async fn get_ping360_config(
        &self,
        device_type: DeviceSelection,
        slot: u8,
    ) -> Result<Answer, ManagerError> {
        let device = self.get_device(device_type, slot)?;
        if let Some(DeviceProperties::Ping360(properties)) = &device.properties {
            return Ok(Answer::DeviceConfig(ModifyDeviceResult::Ping360Config(
                *properties.continuous_mode_settings.read().map_err(|err| {
                    ManagerError::Other(format!(
                        "get_ping360_config: {err}, device type: {device_type}, slot: {slot}"
                    ))
                })?,
            )));
        }
        Err(ManagerError::DeviceSourceError(
            "get_ping360_config: Can't return Ping360Config".to_string(),
        ))
    }

    pub async fn modify_device(&mut self, request: ModifyDevice) -> Result<Answer, ManagerError> {
        match request.modify {
            ModifyDeviceCommand::SetIp(ip) => {
                let device_info = self.info(request.device_type, request.slot).await?;
                let Answer::DeviceInfo(data) = device_info else {
                    return Err(ManagerError::NoDevices);
                };

                let Some(info) = data.first() else {
                    return Err(ManagerError::NoDevices);
                };

                let SourceSelection::UdpStream(inner) = &info.source else {
                    return Err(ManagerError::Other(format!(
                        "modify_device : invalid request for device : {request:?}"
                    )));
                };

                self.modify_device_ip(ip, inner.ip).await?;
                self.delete(request.device_type, request.slot).await?;
                Ok(Answer::DeviceConfig(ModifyDeviceResult::ConfigAcknowledge(
                    request,
                )))
            }
            ModifyDeviceCommand::SetPing360Config(config) => {
                self.update_ping360_config(request.device_type, request.slot, config)
                    .await?;
                Ok(Answer::DeviceConfig(ModifyDeviceResult::ConfigAcknowledge(
                    request,
                )))
            }
            ModifyDeviceCommand::GetPing360Config => {
                self.get_ping360_config(request.device_type, request.slot)
                    .await
            }
        }
    }

    pub async fn modify_device_ip(
        &mut self,
        ip: Ipv4Addr,
        destination: Ipv4Addr,
    ) -> Result<(), ManagerError> {
        let socket =
            UdpSocket::bind("0.0.0.0:0").map_err(|err| ManagerError::Other(err.to_string()))?; // Bind to any available port
        socket
            .set_broadcast(true)
            .map_err(|err| ManagerError::Other(err.to_string()))?;

        let command = format!("SetSS1IP {}", ip);

        socket
            .send_to(command.as_bytes(), format!("{destination}:30303"))
            .map_err(|err| ManagerError::Other(err.to_string()))?;
        Ok(())
    }

    async fn shutdown(&mut self) {
        let device_ids: Vec<_> = self
            .device
            .values()
            .map(|device| (device.device_type, device.slot))
            .collect();
        for (device_type, slot) in device_ids {
            if let Err(error) = self
                .stop_then_teardown_device_runtime(device_type, slot)
                .await
            {
                warn!("DeviceManager shutdown failed: {error}");
            }
        }
    }
}

impl ManagerActorHandler {
    pub async fn send(&self, request: Request) -> Result<Answer, ManagerError> {
        let (result_sender, result_receiver) = oneshot::channel();

        match &request {
            // Devices requests are forwarded directly to device and let manager handle other incoming request.
            Request::Ping(request) => {
                trace!(
                    ?request,
                    "Handling Ping request: Forwarding request to device handler"
                );
                let handler_request =
                    Request::GetDeviceHandler(crate::device::manager::DeviceSlotWrapper {
                        device_type: request.device_type,
                        slot: request.slot,
                    });
                let manager_request = ManagerActorRequest {
                    request: handler_request,
                    respond_to: result_sender,
                };
                self.sender
                    .send(manager_request)
                    .await
                    .map_err(|err| ManagerError::TokioMpsc(err.to_string()))?;
                let result = match result_receiver
                    .await
                    .map_err(|err| ManagerError::TokioMpsc(err.to_string()))
                {
                    Ok(answer) => answer,
                    Err(error) => {
                        error!(
                            ?error,
                            "DeviceManagerHandler: Failed to receive handler from Manager"
                        );
                        return Err(error);
                    }
                };

                match result? {
                    Answer::InnerDeviceHandler(handler) => {
                        trace!(
                            ?request,
                            "Handling Ping request: Successfully received the handler"
                        );
                        let result = handler.send(request.device_request.clone()).await;
                        match result {
                            Ok(result) => {
                                info!(?request, "Handling Ping request: Success");
                                Ok(Answer::DeviceMessage(DeviceAnswer {
                                    answer: result,
                                    device_type: request.device_type,
                                    slot: request.slot,
                                }))
                            }
                            Err(error) => {
                                error!(
                                    ?request,
                                    ?error,
                                    "Handling Ping request: Error occurred on device"
                                );
                                Err(ManagerError::DeviceError(error))
                            }
                        }
                    }
                    answer => Ok(answer), //should be unreachable
                }
            }
            _ => {
                trace!(
                    ?request,
                    "Handling DeviceManager request: Forwarding request."
                );
                let device_request = ManagerActorRequest {
                    request: request.clone(),
                    respond_to: result_sender,
                };

                self.sender
                    .send(device_request)
                    .await
                    .map_err(|err| ManagerError::TokioMpsc(err.to_string()))?;

                match result_receiver
                    .await
                    .map_err(|err| ManagerError::TokioMpsc(err.to_string()))?
                {
                    Ok(answer) => {
                        trace!(?request, "Handling DeviceManager request: Success");
                        Ok(answer)
                    }
                    Err(error) => {
                        error!(
                            ?request,
                            ?error,
                            "Handling DeviceManager request: Error ocurred on manager",
                        );
                        Err(error)
                    }
                }
            }
        }
    }
}

pub async fn turnoff_device_continuous_mode(source: &SourceSelection) -> Result<(), ManagerError> {
    match source {
        SourceSelection::SerialStream(serial_config) => {
            debug!(
                ?serial_config,
                "Sending break line to serial device for 1 second",
            );
            let serial_stream = tokio_serial::new(&serial_config.path, serial_config.baudrate)
                .open_native_async()
                .map_err(|err| ManagerError::DeviceSourceError(err.to_string()))?;
            serial_stream.set_break().map_err(|err| {
                ManagerError::DeviceSourceError(format!("Failed to send break signal: {}", err))
            })?;
            sleep(Duration::from_secs(1)).await;
            serial_stream.clear_break().map_err(|err| {
                ManagerError::DeviceSourceError(format!("Failed to clear break signal: {}", err))
            })?;
            drop(serial_stream);
        }
        SourceSelection::UdpStream(udp_config) => {
            debug!(?udp_config, "Sending empty datagram to UDP device");
            let socket = UdpSocket::bind("0.0.0.0:0").map_err(|err| {
                ManagerError::DeviceSourceError(format!("Failed to bind UDP socket: {}", err))
            })?;
            let empty_datagram: [u8; 0] = [];
            socket
                .send_to(
                    &empty_datagram,
                    format!("{}:{}", udp_config.ip, udp_config.port),
                )
                .map_err(|err| {
                    ManagerError::DeviceSourceError(format!(
                        "Failed to send empty datagram: {}",
                        err
                    ))
                })?;
        }
        SourceSelection::FakeStream(_) => (),
    }

    Ok(())
}
