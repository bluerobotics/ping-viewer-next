/// Specially for DeviceManager to retrieve checks and structures from Devices stored in it's hashmap collection
pub mod continuous_mode;
/// Specially for auto creation methods, from UDP or serial port
pub mod device_discovery;
/// Specially for continuous_mode methods, startup, shutdown, handle and errors routines for each device type
pub mod device_handle;

use paperclip::actix::Apiv2Schema;
use serde::{Deserialize, Serialize};
use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
    net::{Ipv4Addr, SocketAddrV4, UdpSocket},
    ops::Deref,
    time::Duration,
};
use tokio::{
    sync::{mpsc, oneshot},
    time::sleep,
};

use tokio_serial::{SerialPort, SerialPortBuilderExt, SerialStream};
use tracing::{error, info, trace, warn};
use udp_stream::UdpStream;
use uuid::Uuid;

use super::devices::{DeviceActor, DeviceActorHandler};
use bluerobotics_ping::device::{Ping1D, Ping360};

pub struct Device {
    pub id: Uuid,
    pub source: SourceSelection,
    pub handler: super::devices::DeviceActorHandler,
    pub actor: tokio::task::JoinHandle<DeviceActor>,
    pub broadcast: Option<tokio::task::JoinHandle<()>>,
    pub status: DeviceStatus,
    pub device_type: DeviceSelection,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeviceInfo {
    pub id: Uuid,
    pub source: SourceSelection,
    pub status: DeviceStatus,
    pub device_type: DeviceSelection,
}
impl Device {
    pub fn info(&self) -> DeviceInfo {
        DeviceInfo {
            id: self.id,
            source: self.source.clone(),
            status: self.status.clone(),
            device_type: self.device_type.clone(),
        }
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        trace!(
            "Removing Device from DeviceManager, details: {:?}",
            self.info()
        );
        self.actor.abort();
        if let Some(broadcast_handle) = &self.broadcast {
            trace!("Device broadcast handle closed for: {:?}", self.info().id);
            broadcast_handle.abort();
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Apiv2Schema)]
pub enum DeviceSelection {
    Common,
    Ping1D,
    Ping360,
    Auto,
}

#[derive(Debug, Clone, Deserialize, Serialize, Hash, Apiv2Schema)]
pub enum SourceSelection {
    UdpStream(SourceUdpStruct),
    SerialStream(SourceSerialStruct),
}

enum SourceType {
    Udp(UdpStream),
    Serial(SerialStream),
}

#[derive(Clone, Debug, Deserialize, Serialize, Hash, Apiv2Schema)]
pub struct SourceUdpStruct {
    pub ip: Ipv4Addr,
    pub port: u16,
}

#[derive(Clone, Debug, Deserialize, Serialize, Hash, Apiv2Schema)]
pub struct SourceSerialStruct {
    pub path: String,
    pub baudrate: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeviceStatus {
    Running,
    Stopped,
    ContinuousMode,
}

pub struct DeviceManager {
    receiver: mpsc::Receiver<ManagerActorRequest>,
    pub device: HashMap<Uuid, Device>,
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
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ManagerError {
    DeviceNotExist(Uuid),
    DeviceAlreadyExist(Uuid),
    DeviceStatus(DeviceStatus, Uuid),
    DeviceError(super::devices::DeviceError),
    DeviceSourceError(String),
    NoDevices,
    TokioMpsc(String),
    NotImplemented(Request),
    Other(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeviceAnswer {
    #[serde(flatten)]
    pub answer: crate::device::devices::PingAnswer,
    pub device_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, Apiv2Schema)]
#[serde(tag = "command", content = "payload")]
pub enum Request {
    AutoCreate,
    Create(CreateStruct),
    Delete(UuidWrapper),
    List,
    Info(UuidWrapper),
    Search,
    Ping(DeviceRequestStruct),
    GetDeviceHandler(UuidWrapper),
    ModifyDevice(ModifyDevice),
    EnableContinuousMode(UuidWrapper),
    DisableContinuousMode(UuidWrapper),
}

#[derive(Debug, Clone, Serialize, Deserialize, Apiv2Schema)]
pub enum ModifyDeviceCommand {
    Ip(Ipv4Addr),
}

#[derive(Debug, Clone, Serialize, Deserialize, Apiv2Schema)]
pub struct ModifyDevice {
    pub uuid: Uuid,
    pub modify: ModifyDeviceCommand,
}

#[derive(Debug, Clone, Serialize, Deserialize, Apiv2Schema)]
pub struct UuidWrapper {
    pub uuid: Uuid,
}

impl Deref for UuidWrapper {
    type Target = Uuid;

    fn deref(&self) -> &Self::Target {
        &self.uuid
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, Apiv2Schema)]
pub struct CreateStruct {
    pub source: SourceSelection,
    pub device_selection: DeviceSelection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceRequestStruct {
    pub uuid: Uuid,
    pub device_request: crate::device::devices::PingRequest,
}

impl DeviceManager {
    async fn handle_message(&mut self, actor_request: ManagerActorRequest) {
        trace!("DeviceManager: Received a request, details: {actor_request:?}");
        match actor_request.request {
            Request::AutoCreate => {
                let result = self.auto_create().await;
                if let Err(e) = actor_request.respond_to.send(result) {
                    error!("DeviceManager: Failed to return AutoCreate response: {e:?}");
                }
            }
            Request::Create(request) => {
                let result = self.create(request.source, request.device_selection).await;
                if let Err(e) = actor_request.respond_to.send(result) {
                    error!("DeviceManager: Failed to return Create response: {e:?}");
                }
            }
            Request::Delete(uuid) => {
                let result = self.delete(*uuid).await;
                if let Err(e) = actor_request.respond_to.send(result) {
                    error!("DeviceManager: Failed to return Delete response: {e:?}");
                }
            }
            Request::List => {
                let result = self.list().await;
                if let Err(e) = actor_request.respond_to.send(result) {
                    error!("DeviceManager: Failed to return List response: {e:?}");
                }
            }
            Request::Info(device_id) => {
                let result = self.info(*device_id).await;
                if let Err(e) = actor_request.respond_to.send(result) {
                    error!("DeviceManager: Failed to return Info response: {:?}", e);
                }
            }
            Request::EnableContinuousMode(uuid) => {
                let result = self.continuous_mode(*uuid).await;
                if let Err(e) = actor_request.respond_to.send(result) {
                    error!("DeviceManager: Failed to return EnableContinuousMode response: {e:?}");
                }
            }
            Request::DisableContinuousMode(uuid) => {
                let result = self.continuous_mode_off(*uuid).await;
                if let Err(e) = actor_request.respond_to.send(result) {
                    error!("DeviceManager: Failed to return DisableContinuousMode response: {e:?}");
                }
            }
            Request::GetDeviceHandler(id) => {
                let answer = self.get_device_handler(*id).await;
                if let Err(e) = actor_request.respond_to.send(answer) {
                    error!("DeviceManager: Failed to return GetDeviceHandler response: {e:?}");
                }
            }
            Request::ModifyDevice(request) => {
                let answer = self.modify_device(request).await;
                if let Err(err) = actor_request.respond_to.send(answer) {
                    error!("DeviceManager: Failed to return ModifyDevice response: {err:?}");
                }
            }
            _ => {
                if let Err(e) = actor_request
                    .respond_to
                    .send(Err(ManagerError::NotImplemented(actor_request.request)))
                {
                    warn!("DeviceManager: Failed to return response: {e:?}");
                }
            }
        }
    }

    pub fn new(size: usize) -> (Self, ManagerActorHandler) {
        let (sender, receiver) = mpsc::channel(size);
        let actor = DeviceManager {
            receiver,
            device: HashMap::new(),
        };
        let actor_handler = ManagerActorHandler { sender };

        trace!("DeviceManager and handler successfully created: Success");
        (actor, actor_handler)
    }

    pub async fn run(mut self) {
        info!("DeviceManager is running");
        while let Some(msg) = self.receiver.recv().await {
            self.update_devices_status().await; // Todo: move to an outer process
            self.handle_message(msg).await;
        }
        error!("DeviceManager has stopped please check your application");
    }

    pub async fn update_devices_status(&mut self) {
        if let Ok(Answer::DeviceInfo(answer)) = self.list().await {
            for device in answer {
                if let Some(device_entry) = self.device.get_mut(&device.id) {
                    if device_entry.status == DeviceStatus::Stopped {
                        break;
                    }
                    if device_entry.actor.is_finished() {
                        info!("Device stopped, device id: {device:?}");
                        device_entry.status = DeviceStatus::Stopped;
                    }
                }
            }
        }
    }

    pub async fn create(
        &mut self,
        source: SourceSelection,
        mut device_selection: DeviceSelection,
    ) -> Result<Answer, ManagerError> {
        let mut hasher = DefaultHasher::new();
        source.hash(&mut hasher);
        let hash = Uuid::from_u128(hasher.finish().into());

        if self.device.contains_key(&hash) {
            trace!("Device creation error: Device already exist for provided SourceSelection, details: {source:?}");
            return Err(ManagerError::DeviceAlreadyExist(hash));
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

                SourceType::Serial(serial_stream)
            }
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

                        sleep(retry_delay).await;
                        continue;
                    }
                    e => warn!("Device creation error: Abnormal answer: {e:?}."),
                }
            }
        }

        let actor = tokio::spawn(async move { device.run().await });

        let device = Device {
            id: hash,
            source,
            handler,
            actor,
            status: DeviceStatus::Running,
            broadcast: None,
            device_type: device_selection,
        };

        self.device.insert(hash, device);

        trace!("Device broadcast enable by default for: {hash:?}");
        let device_info = self.continuous_mode(hash).await?;

        info!("New device created and available, details: {device_info:?}");
        Ok(device_info)
    }

    pub async fn auto_create(&mut self) -> Result<Answer, ManagerError> {
        let mut results = Vec::new();

        let mut available_source = Vec::new();

        match device_discovery::serial_discovery().await {
            Some(result) => available_source.extend(result),
            None => warn!("Auto create: Unable to find available devices on serial ports"),
        }

        match device_discovery::network_discovery() {
            Some(result) => available_source.extend(result),
            None => warn!("Auto create: Unable to find available devices on network"),
        }

        for source in available_source {
            match self.create(source.clone(), DeviceSelection::Auto).await {
                Ok(answer) => match answer {
                    Answer::DeviceInfo(device_info) => {
                        results.extend(device_info);
                    }
                    msg => {
                        warn!("Some unexpected message during auto_create, details: {msg:?}")
                    }
                },
                Err(err) => {
                    error!("Failed to create device for source {source:?}: {err:?}");
                }
            }
        }

        Ok(Answer::DeviceInfo(results))
    }

    pub async fn list(&self) -> Result<Answer, ManagerError> {
        if self.device.is_empty() {
            trace!("No devices available for list generation request");
            return Err(ManagerError::NoDevices);
        };
        let mut list = Vec::new();
        for device in self.device.values() {
            list.push(device.info())
        }
        Ok(Answer::DeviceInfo(list))
    }

    pub async fn info(&self, device_id: Uuid) -> Result<Answer, ManagerError> {
        self.check_device_uuid(device_id)?;
        Ok(Answer::DeviceInfo(vec![self.get_device(device_id)?.info()]))
    }

    pub async fn delete(&mut self, device_id: Uuid) -> Result<Answer, ManagerError> {
        match self.device.remove(&device_id) {
            Some(device) => {
                let device_info = device.info();
                drop(device);
                trace!("Device delete id {device_id:?}: Success",);
                Ok(Answer::DeviceInfo(vec![device_info]))
            }
            None => {
                error!("Device delete id {device_id:?} : Error, device doesn't exist");
                Err(ManagerError::DeviceNotExist(device_id))
            }
        }
    }

    pub async fn continuous_mode(&mut self, device_id: Uuid) -> Result<Answer, ManagerError> {
        self.check_device_status(device_id, &[DeviceStatus::Running])?;
        let device_type = self.get_device_type(device_id)?;

        // Get an inner subscriber for device's stream
        let subscriber = self.get_subscriber(device_id).await?;

        let broadcast_handle = self
            .continuous_mode_start(subscriber, device_id, device_type.clone())
            .await;
        if let Some(handle) = &broadcast_handle {
            if !handle.is_finished() {
                trace!("Success start_continuous_mode for {device_id:?}");
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

        self.continuous_mode_startup_routine(device_id, device_type)
            .await?;

        let device = self.get_mut_device(device_id)?;
        device.broadcast = broadcast_handle;
        device.status = DeviceStatus::ContinuousMode;

        let updated_device_info = self.get_device(device_id)?.info();

        Ok(Answer::DeviceInfo(vec![updated_device_info]))
    }

    pub async fn continuous_mode_off(&mut self, device_id: Uuid) -> Result<Answer, ManagerError> {
        self.check_device_status(device_id, &[DeviceStatus::ContinuousMode])?;
        let device_type = self.get_device_type(device_id)?;

        let device = self.get_mut_device(device_id)?;
        if let Some(broadcast) = device.broadcast.take() {
            broadcast.abort_handle().abort();
        }

        device.status = DeviceStatus::Running;

        let updated_device_info = device.info();

        self.continuous_mode_shutdown_routine(device_id, device_type)
            .await?;

        Ok(Answer::DeviceInfo(vec![updated_device_info]))
    }

    pub async fn modify_device(&mut self, request: ModifyDevice) -> Result<Answer, ManagerError> {
        match request.modify {
            ModifyDeviceCommand::Ip(ip) => {
                let device_info = self.info(request.uuid).await?;
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
                self.delete(request.uuid).await
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
}

impl ManagerActorHandler {
    pub async fn send(&self, request: Request) -> Result<Answer, ManagerError> {
        let (result_sender, result_receiver) = oneshot::channel();

        match &request {
            // Devices requests are forwarded directly to device and let manager handle other incoming request.
            Request::Ping(request) => {
                trace!("Handling Ping request: {request:?}: Forwarding request to device handler");
                let get_handler_target = request.uuid;
                let handler_request =
                    Request::GetDeviceHandler(crate::device::manager::UuidWrapper {
                        uuid: get_handler_target,
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
                    Ok(ans) => ans,
                    Err(err) => {
                        error!("DeviceManagerHandler: Failed to receive handler from Manager, details: {err:?}");
                        return Err(err);
                    }
                };

                match result? {
                    Answer::InnerDeviceHandler(handler) => {
                        trace!(
                            "Handling Ping request: {request:?}: Successfully received the handler"
                        );
                        let result = handler.send(request.device_request.clone()).await;
                        match result {
                            Ok(result) => {
                                info!("Handling Ping request: {request:?}: Success");
                                Ok(Answer::DeviceMessage(DeviceAnswer {
                                    answer: result,
                                    device_id: request.uuid,
                                }))
                            }
                            Err(err) => {
                                error!(
                                    "Handling Ping request: {request:?}: Error ocurred on device: {err:?}"                                );
                                Err(ManagerError::DeviceError(err))
                            }
                        }
                    }
                    answer => Ok(answer), //should be unreachable
                }
            }
            _ => {
                trace!("Handling DeviceManager request: {request:?}: Forwarding request.");
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
                    Ok(ans) => {
                        trace!("Handling DeviceManager request: {request:?}: Success");
                        Ok(ans)
                    }
                    Err(err) => {
                        error!(
                            "Handling DeviceManager request: {request:?}: Error ocurred on manager: {err:?}",
                        );
                        Err(err)
                    }
                }
            }
        }
    }
}
