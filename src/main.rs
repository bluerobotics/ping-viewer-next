use serde::{Deserialize, Serialize};
use tracing::info;

#[macro_use]
extern crate lazy_static;

mod cli;
/// The Device module consists of two main modules: devices and manager.
mod device;
mod logger;
mod server;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "module")]
pub enum ModuleType {
    DeviceManager(device::manager::Request),
}

#[tokio::main]
async fn main() {
    // CLI should be started before logger to allow control over verbosity
    cli::manager::init();
    // Logger should start before everything else to register any log information
    logger::manager::init();

    let (mut manager, handler) = device::manager::DeviceManager::new(10);

    //Todo: Load previous devices
    if cli::manager::is_enable_auto_create() {
        match manager.auto_create().await {
            Ok(answer) => info!("DeviceManager initialized with following devices: {answer:?}"),
            Err(err) => info!("DeviceManager unable to initialize with devices, details {err:?}"),
        }
    }

    tokio::spawn(async move { manager.run().await });

    server::manager::run(&cli::manager::server_address(), handler)
        .await
        .unwrap();
}
