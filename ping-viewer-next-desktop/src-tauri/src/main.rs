// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use ping_viewer_next::{cli, device, logger, server};
use tauri::Manager;

#[tokio::main]
async fn main() {
    cli::manager::init();

    logger::manager::init();

    let (manager, handler) = device::manager::DeviceManager::new(10);

    tokio::spawn(async move { manager.run().await });

    run_tauri_app(handler).await;
}

async fn run_tauri_app(handler: device::manager::ManagerActorHandler) {
    tauri::Builder::default()
        .setup(|app: &mut tauri::App| {
            let window = app.get_webview_window("main").unwrap();

            std::thread::spawn(move || {
                run_from_tauri(&cli::manager::server_address(), handler).unwrap();
            });

            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(6));
                window.eval("window.location.replace('http://127.0.0.1:8080')").unwrap();
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[actix_web::main]
pub async fn run_from_tauri(
    server_address: &str,
    handler: device::manager::ManagerActorHandler,
) -> std::io::Result<()> {
    server::manager::run(server_address, handler).await
}
