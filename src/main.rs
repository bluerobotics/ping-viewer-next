

use device::manager::{CreateStruct, DeviceRequestStruct, UuidWrapper};
use serde::{Deserialize, Serialize};
use tracing::info;
use ts_rs::TS;

#[macro_use]
extern crate lazy_static;

mod cli;
/// The Device module consists of two main modules: devices and manager.
mod device;
mod logger;
mod server;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "RequestModels.ts")]
pub struct Command {
    pub command: CommandType,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "module")]
pub enum CommandType {
    DeviceManager(device::manager::Request),
}

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub enum Command {
//     DeviceManager(device::manager::Request),
// }

#[tokio::main]
async fn main() {
    // CLI should be started before logger to allow control over verbosity
    cli::manager::init();
    // Logger should start before everything else to register any log information
    logger::manager::init();

    let (manager, handler) = device::manager::DeviceManager::new(10);
    tokio::spawn(async move { manager.run().await });

    //Todo: Load previous devices
    info!(
        "DeviceManager initialized with following devices: {:?}",
        handler.send(crate::device::manager::Request::List).await
    );
    use serde_json::json;
    use uuid::Uuid;

    // Define your requests
    let requests = vec![
        crate::device::manager::Request::Ping(DeviceRequestStruct {
            uuid: Uuid::parse_str("00000000-0000-0000-001e-10da679f8cee").unwrap(),
            device_request: crate::device::devices::PingRequest::Ping360(
                crate::device::devices::Ping360Request::Transducer(
                    bluerobotics_ping::ping360::TransducerStruct {
                        mode: 1,
                        gain_setting: 2,
                        angle: 0,
                        transmit_duration: 500,
                        sample_period: 80,
                        transmit_frequency: 700,
                        number_of_samples: 1200,
                        transmit: 1,
                        reserved: 1,
                    },
                ),
            ),
        }),
        crate::device::manager::Request::EnableContinuousMode(UuidWrapper {
            uuid: Uuid::parse_str("00000000-0000-0000-001e-10da679f8cee").unwrap(),
        }),
        crate::device::manager::Request::List,
        crate::device::manager::Request::Create(CreateStruct {
            source: device::manager::SourceSelection::SerialStream(
                device::manager::SourceSerialStruct {
                    path: "/dev/ttyUSB0".to_string(),
                    baudrate: 115200,
                },
            ),
            device_selection: device::manager::DeviceSelection::Auto,
        }),
        crate::device::manager::Request::Create(CreateStruct {
            source: device::manager::SourceSelection::UdpStream(device::manager::SourceUdpStruct {
                ip: "192.168.0.1".parse().unwrap(),
                port: 9092,
            }),
            device_selection: device::manager::DeviceSelection::Auto,
        }),
    ];

    // Print each request as JSON
    for request in requests {
        println!(
            "{}",
            json!(Command {
                command: CommandType::DeviceManager(request)
            })
        );
    }

    server::manager::run(&cli::manager::server_address(), handler)
        .await
        .unwrap();
}
