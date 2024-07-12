use serde::{Deserialize, Serialize};
use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
    net::{Ipv4Addr, SocketAddrV4},
};
use tokio::sync::{mpsc, oneshot};
use tokio_serial::{SerialPort, SerialPortBuilderExt, SerialStream};
// use tracing::{error, info};
use udp_stream::UdpStream;
use uuid::Uuid;

use super::devices::{DeviceActor, DeviceActorHandler};
use bluerobotics_ping::device::{Ping1D, Ping360};

struct Device {
    id: Uuid,
    source: SourceSelection,
    handler: super::devices::DeviceActorHandler,
    actor: tokio::task::JoinHandle<DeviceActor>,
    status: DeviceStatus,
    device_type: DeviceSelection,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeviceSelection {
    Common,
    Ping1D,
    Ping360,
    Auto,
}

#[derive(Debug, Clone, Deserialize, Serialize, Hash)]
pub enum SourceSelection {
    UdpStream(SourceUdpStruct),
    SerialStream(SourceSerialStruct),
}

enum SourceType {
    Udp(UdpStream),
    Serial(SerialStream),
}

#[derive(Clone, Debug, Deserialize, Serialize, Hash)]
pub struct SourceUdpStruct {
    pub ip: Ipv4Addr,
    pub port: u16,
}

#[derive(Clone, Debug, Deserialize, Serialize, Hash)]
pub struct SourceSerialStruct {
    pub path: String,
    pub baudrate: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeviceStatus {
    Running,
    Stopped,
}

pub struct DeviceManager {
    receiver: mpsc::Receiver<ManagerActorRequest>,
    device: HashMap<Uuid, Device>,
}

#[derive(Debug)]
pub struct ManagerActorRequest {
    pub request: Request,
    pub respond_to: oneshot::Sender<Answer>,
}
#[derive(Clone)]
pub struct ManagerActorHandler {
    pub sender: mpsc::Sender<ManagerActorRequest>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Answer {
    Ping(PingAnswer),
    #[serde(skip)]
    InnerDeviceHandler(DeviceActorHandler),
    DeviceInfo(Vec<DeviceInfo>),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ManagerError {
    DeviceNotExist(Uuid),
    DeviceAlreadyExist(Uuid),
    DeviceIsStopped(Uuid),
    Other,
}

#[derive(Debug, Serialize, Deserialize, Clone)]

pub struct DeviceActorAnswer {
    pub answer: crate::device::devices::DeviceActorAnswer,
    pub device_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Request {
    Create(CreateStruct),
    Delete(Uuid),
    List,
    Status,
    Search,
    Ping(DeviceRequestStruct),
    GetDeviceHandler(Uuid),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateStruct {
    pub source: SourceSelection,
    pub device_selection: DeviceSelection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceRequestStruct {
    pub target: Uuid,
    pub request: crate::device::devices::PingRequest,
}

impl DeviceManager {
    async fn handle_message(&mut self, actor_request: ManagerActorRequest) {
        trace!(
            "DeviceManager: Received a request, details: {:?}",
            actor_request
        );
        match actor_request.request {
            Request::Create(request) => {
                let result = self.create(request.source, request.device_selection).await;
                if let Err(e) = actor_request.respond_to.send(result) {
                    error!("DeviceManager: Failed to return Create response: {:?}", e);
                }
            }
            Request::Delete(uuid) => {
                let result = self.delete(uuid).await;
                if let Err(e) = actor_request.respond_to.send(result) {
                    error!("DeviceManager: Failed to return Delete response: {:?}", e);
                }
            }
            Request::List => {
                let result = self.list().await;
                if let Err(e) = actor_request.respond_to.send(result) {
                    error!("DeviceManager: Failed to return List response: {:?}", e);
                }
            }
            Request::GetDeviceHandler(id) => {
                let answer = self.get_device_handler(id).await;
                if let Err(e) = actor_request.respond_to.send(answer) {
                    error!(
                        "DeviceManager: Failed to return GetDeviceHandler response: {:?}",
                        e
                    );
                }
            }
            _ => todo!(), // Unreachable, DeviceManagerHandler uses GetDeviceHandler and forwards the requests.
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
        while let Some(msg) = self.receiver.recv().await {
            self.update_devices_status().await; // Todo: move to an outer process
            self.handle_message(msg).await;
        }
        error!("DeviceManager has stopped please check your application");
    }

    pub async fn update_devices_status(&mut self) {
        if let Answer::DeviceInfo(answer) = self.list().await {
            for device in answer {
                if let Some(device_entry) = self.device.get_mut(&device.id) {
                    if device_entry.status == DeviceStatus::Stopped {
                        break;
                    }
                    if device_entry.actor.is_finished() {
                        info!("Device stopped, device id: {:?}", device);
                        device_entry.status = DeviceStatus::Stopped;
                    }
                }
                if self.device.get(&device.id).unwrap().actor.is_finished() {
                    self.device.get_mut(&device.id).unwrap().status = DeviceStatus::Stopped;
                };
            }
        }
    }

    pub async fn create(
        &mut self,
        source: SourceSelection,
        mut device_selection: DeviceSelection,
    ) -> Answer {
        let mut hasher = DefaultHasher::new();
        source.hash(&mut hasher);
        let hash = Uuid::from_u128(hasher.finish().into());

        if self.device.contains_key(&hash) {
            return Answer::Error(ManagerError::DeviceAlreadyExist(hash));
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
                let serial_stream = tokio_serial::new(
                    source_serial_struct.path.clone(),
                    source_serial_struct.baudrate,
                )
                .open_native_async()
                .map_err(|err| ManagerError::DeviceSourceError(err.to_string()))?;

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
            if let super::devices::PingAnswer::UpgradeResult(result) = device.try_upgrade().await {
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
                },
                Err(err) => {
                    error!(
                        "Device creation error: Can't auto upgrade the DeviceType, details: {:?}",
                        err
                    );
                    return Err(ManagerError::DeviceError(err));
                }
                _ => todo!(),
            }
        }

        let actor = tokio::spawn(async move { device.run().await });

        let device = Device {
            id: hash,
            source,
            handler,
            actor,
            status: DeviceStatus::Running,
            device_type: device_selection,
        };

        let device_info = device.info();

        self.device.insert(hash, device);

        Answer::DeviceInfo(vec![device_info])
    }

    pub async fn list(&self) -> Answer {
        let mut list = Vec::new();
        for device in self.device.values() {
            list.push(device.info())
        }
        Answer::DeviceInfo(list)
    }

    pub async fn delete(&mut self, device_id: Uuid) -> Answer {
        match self.device.get(&device_id) {
            Some(device) => Answer::DeviceInfo(vec![device.info()]),
            None => Answer::Error(ManagerError::DeviceNotExist(device_id)),
        }
    }

    pub async fn get_device_handler(&self, target: Uuid) -> Answer {
        if self.device.contains_key(&target) {
            let handler: DeviceActorHandler = self.device.get(&target).unwrap().handler.clone();
            return Answer::InnerDeviceHandler(handler);
        }
        Answer::Error(ManagerError::DeviceNotExist(target))
    }
}

impl ManagerActorHandler {
    pub async fn send(
        &self,
        request: Request,
    ) -> Result<Answer, tokio::sync::mpsc::error::SendError<ManagerActorRequest>> {
        let (result_sender, result_receiver) = oneshot::channel();

        let result = match &request {
            Request::Ping(request) => {
                let get_handler_target = request.target;
                let handler_request = Request::GetDeviceHandler(get_handler_target);
                let manager_request = ManagerActorRequest {
                    request: handler_request,
                    respond_to: result_sender,
                };
                self.sender.send(manager_request).await?;

                let result = result_receiver.await.unwrap_or(Answer::Error(
                    ManagerError::ManagerUnreachable(request.clone()),
                ));

                match result {
                    Answer::InnerDeviceHandler(handler) => {
                        let result = handler.send(request.request.clone()).await;
                        match result {
                            Ok(result) => Answer::Ping(DeviceActorAnswer {
                                answer: result,
                                device_id: request.target,
                            }),
                            Err(_) => {
                                Answer::Error(ManagerError::DeviceUnreachable(request.target))
                            }
                        }
                    }
                    Answer::Error(e) => Answer::Error(e),
                    _ => Answer::Error(ManagerError::Other), //should be unreachable
                }
            }
            _ => {
                let device_request = ManagerActorRequest {
                    request: request.clone(),
                    respond_to: result_sender,
                };

                self.sender.send(device_request).await?;

                result_receiver
                    .await
                    .unwrap_or(Answer::Error(ManagerError::Other))
            }
        };

        Ok(result)
    }
}
