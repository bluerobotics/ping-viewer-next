use tracing::info;

#[macro_use]
extern crate lazy_static;

mod cli;
mod device;
mod logger;
mod server;

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
        handler
            .send(crate::device::manager::Request::List {})
            .await
            .unwrap()
    );

    server::manager::run(&cli::manager::server_address())
        .await
        .unwrap();
}
