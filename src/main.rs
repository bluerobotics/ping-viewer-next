use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use ping_viewer_next::{cli, device, logger, server, vehicle::zenoh_client_bridge};

#[tokio::main]
async fn main() {
    // CLI should be started before logger to allow control over verbosity
    cli::manager::init();
    // Logger should start before everything else to register any log information
    logger::manager::init();

    let vehicle_data = Arc::new(RwLock::new(None));

    // Start the Zenoh-client with shared data
    tokio::spawn(zenoh_client_bridge(vehicle_data.clone()));

    let (mut manager, handler) = device::manager::DeviceManager::new(10);

    //Todo: Load previous devices
    if cli::manager::is_enable_auto_create() {
        match manager.auto_create().await {
            Ok(answer) => info!("DeviceManager initialized with following devices: {answer:?}"),
            Err(err) => info!("DeviceManager unable to initialize with devices, details {err:?}"),
        }
    }

    let (recordings_manager, recordings_manager_handler) =
        device::recording::RecordingManager::new(10, "recordings", handler.clone());
    tokio::spawn(async move { recordings_manager.run().await });

    tokio::spawn(async move { manager.run().await });

    server::manager::run(
        &cli::manager::server_address(),
        handler,
        recordings_manager_handler,
    )
    .await
    .unwrap();
}
