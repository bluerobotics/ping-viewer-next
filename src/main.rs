use tracing::info;

use ping_viewer_next::{cli, device, logger, server};

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
