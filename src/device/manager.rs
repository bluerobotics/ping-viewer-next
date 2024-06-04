// use std::sync::Arc;

// use tokio::sync::Mutex;
use std::{
    collections::HashMap,
    net::{Ipv4Addr, SocketAddrV4},
    sync::Arc,
};

use bluerobotics_ping::{
    device::{Common, Ping1D, Ping360, PingDevice},
    message::{MessageInfo, PingMessage},
    ping1d::{self, ProfileStruct},
    Messages,
};

use paperclip::actix::Apiv2Schema;
use serde::{Deserialize, Serialize};

use serde_json::json;
use tokio::sync::RwLock;
use tokio_serial::{SerialPort, SerialPortBuilderExt, SerialStream};
use tracing::{error, info};
use udp_stream::UdpStream;

use crate::server::protocols::v1::websocket;

struct Device {
    source: SourceSelection,
    device: DeviceType,
    status: DeviceStatus,
}

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use uuid::Uuid;

// impl Device {
//     pub fn generate_uuid(&self) -> Uuid {
//         let mut hasher = DefaultHasher::new();
//         self.source.hash(&mut hasher);
//         // self.device.hash(&mut hasher);
//         // self.status.hash(&mut hasher);

//         let hash = hasher.finish();
//         Uuid::from_u128(hash.into())
//     }
// }
pub enum DeviceType {
    Common(bluerobotics_ping::device::Common),
    Ping1D(bluerobotics_ping::device::Ping1D),
    Ping360(bluerobotics_ping::device::Ping360),
}

pub enum DeviceSelection {
    Common,
    Ping1D,
    Ping360,
}

#[derive(Apiv2Schema, Debug, Deserialize, Serialize, Hash)]
pub enum SourceSelection {
    UdpStream(SourceUdpStruct),
    SerialStream(SourceSerialStruct),
}

enum SourceType {
    Udp(UdpStream),
    Serial(SerialStream),
}

#[derive(Apiv2Schema, Debug, Deserialize, Serialize)]
pub struct DeviceNumber {
    pub number: Option<usize>,
}

#[derive(Apiv2Schema, Debug, Deserialize, Serialize, Hash)]
pub struct SourceUdpStruct {
    pub ip: Ipv4Addr,
    pub port: u16,
}

#[derive(Apiv2Schema, Debug, Deserialize, Serialize, Hash)]
pub struct SourceSerialStruct {
    pub path: String,
    pub baudrate: u32,
}
#[derive(Debug, Hash)]
enum DeviceStatus {
    Running,
    Stopped,
    Resetting,
    Error,
}

#[derive(Default)]
pub struct DeviceManager {
    ping: HashMap<Uuid, Device>,
    monitor: HashMap<Uuid, tokio::task::JoinHandle<()>>,
}

lazy_static! {
    static ref MANAGER: Arc<RwLock<DeviceManager>> = Default::default();
}

impl DeviceManager {
    pub fn get_instance() -> &'static RwLock<Self> {
        &MANAGER
    }

    pub async fn create(source: SourceSelection, device_selection: DeviceSelection) {
        let _device: DeviceType;

        let mut hasher = DefaultHasher::new();
        source.hash(&mut hasher);
        let hash = Uuid::from_u128(hasher.finish().into());

        if Self::get_instance().read().await.ping.contains_key(&hash) {
            error!("Source already in use");
            return;
        }

        let port = match &source {
            SourceSelection::UdpStream(source_udp_struct) => {
                let socket_addr = SocketAddrV4::new(source_udp_struct.ip, source_udp_struct.port);

                let udp_stream = UdpStream::connect(socket_addr.into())
                    .await
                    .map_err(|e| {
                        error!("Error connecting to UDP socket: {}", e);
                        e
                    })
                    .unwrap();
                SourceType::Udp(udp_stream)
            }
            SourceSelection::SerialStream(source_serial_struct) => {
                let serial_stream = tokio_serial::new(
                    source_serial_struct.path.clone(),
                    source_serial_struct.baudrate,
                )
                .open_native_async()
                .map_err(|e| {
                    error!("Error opening serial port: {}", e);
                    e
                })
                .unwrap();

                info!("Cleaning serial buffer");
                serial_stream.clear(tokio_serial::ClearBuffer::All).unwrap();

                SourceType::Serial(serial_stream)
            }
        };

        let device = match device_selection {
            DeviceSelection::Common => match port {
                SourceType::Udp(udp_port) => DeviceType::Common(Common::new(udp_port)),
                SourceType::Serial(serial_port) => DeviceType::Common(Common::new(serial_port)),
            },
            DeviceSelection::Ping1D => match port {
                SourceType::Udp(udp_port) => DeviceType::Ping1D(Ping1D::new(udp_port)),
                SourceType::Serial(serial_port) => DeviceType::Ping1D(Ping1D::new(serial_port)),
            },
            DeviceSelection::Ping360 => match port {
                SourceType::Udp(udp_port) => DeviceType::Ping360(Ping360::new(udp_port)),
                SourceType::Serial(serial_port) => DeviceType::Ping360(Ping360::new(serial_port)),
            },
        };

        let status = DeviceStatus::Running;

        let lock = Self::get_instance().write();

        lock.await.ping.insert(
            hash,
            Device {
                source,
                device,
                status,
            },
        );

        Self::subscribe(Some(hash)).await
    }

    pub async fn list() -> Vec<String> {
        let lock = Self::get_instance().read().await;
        let mut list = Vec::new();
        for device in lock.ping.iter() {
            list.push(format!("{:?}", device.0));
        }
        list
    }

    pub async fn init(device_number: Uuid) {
        let lock = Self::get_instance().read().await;
        if !lock.ping.is_empty() {
            if let Some(device) = lock.ping.get(&device_number) {
                match &device.device {
                    DeviceType::Ping1D(inner) => {
                        inner
                            .continuous_start(bluerobotics_ping::ping1d::ProfileStruct::id())
                            .await
                            .unwrap();
                    }
                    DeviceType::Ping360(_) => todo!(),
                    DeviceType::Common(_) => todo!(),
                }
            }
        }
    }

    // pub async fn request(request : ping1d::Messages) {
    //     // get inner struct
    //     request.inner();

    //     let lock = Self::get_instance().read().await;
    //     if !lock.ping.is_empty() {
    //         match &lock.ping[0].device {
    //             DeviceType::Ping1D(inner) => {
    //                 let result: Result<request., PingError> = inner.request().await;
    //                 match result {
    //                     Ok(mode_auto_struct) => {
    //                         println!("{mode_auto_struct:?}");// Handle successful result
    //                     }
    //                     Err(e) => {
    //                         println!("{e:?}");
    //                     }

    //             }
    //         }
    //             DeviceType::Common(_) => todo!(),
    //             DeviceType::Ping360(_) => todo!(),
    //     }
    // }
    // }

    pub async fn stop(device_number: Uuid) {
        let lock = Self::get_instance().read().await;
        if !lock.ping.is_empty() {
            if let Some(device) = lock.ping.get(&device_number) {
                match &device.device {
                    DeviceType::Ping1D(inner) => {
                        inner
                            .continuous_stop(bluerobotics_ping::ping1d::ProfileStruct::id())
                            .await
                            .unwrap();
                    }
                    DeviceType::Ping360(_) => todo!(),
                    DeviceType::Common(_) => todo!(),
                }
            }
        }
    }

    pub async fn subscribe(device_number: Option<Uuid>) {
        let mut lock = Self::get_instance().write().await;
        let hash = device_number.unwrap();

        if !lock.ping.is_empty() {
            if let Some(device) = lock.ping.get(&device_number.unwrap()) {
                match &device.device {
                    DeviceType::Ping1D(inner) => {
                        let mut subscribed = inner.subscribe();

                        let (_tx, _rx) = tokio::sync::oneshot::channel::<Vec<ProfileStruct>>();
                        inner
                            .continuous_start(bluerobotics_ping::ping1d::ProfileStruct::id())
                            .await
                            .unwrap();

                        let handle = tokio::spawn(async move {
                            let mut profile_struct_vector: Vec<ProfileStruct> = Vec::new();
                            loop {
                                let received = subscribed.recv().await;
                                match received {
                                    Ok(msg) => {
                                        if msg.message_id
                                            == bluerobotics_ping::ping1d::ProfileStruct::id()
                                        {
                                            match Messages::try_from(&msg) {
                                                Ok(Messages::Ping1D(
                                                    ping1d::Messages::Profile(answer),
                                                )) => {
                                                    profile_struct_vector.push(answer.clone());
                                                    websocket::send_to_websockets(json!(format!(
                                                    "Device Number {device_number:?} : Message : {:?}",
                                                    answer
                                                )), device_number);
                                                }
                                                _ => continue,
                                            }
                                        }
                                    }
                                    Err(_e) => break,
                                }
                            }
                        });
                        lock.monitor.insert(hash, handle);
                    }
                    DeviceType::Ping360(_) => todo!(),
                    DeviceType::Common(_) => todo!(),
                }
            }
        }
    }
}
