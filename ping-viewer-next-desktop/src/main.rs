// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::net::{SocketAddr, SocketAddrV4};

use ping_viewer_next::{cli, device, logger, server};
#[cfg(target_os = "linux")]
use tao::platform::unix::WindowExtUnix;
use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Icon, WindowBuilder},
};
use wry::WebViewBuilder;
#[cfg(target_os = "linux")]
use wry::WebViewBuilderExtUnix;

#[tokio::main]
async fn main() {
    let app_dir = app_dirs2::app_root(
        app_dirs2::AppDataType::UserData,
        &app_dirs2::AppInfo {
            name: "PingViewerNext",
            author: "BlueRobotics",
        },
    )
    .expect("should create app directory");
    cli::manager::set_base_dir(app_dir);

    cli::manager::init();

    logger::manager::init();

    let (manager, handler) = device::manager::DeviceManager::new(10);

    let (recordings_manager, recordings_manager_handler) = device::recording::RecordingManager::new(
        10,
        cli::manager::recordings_path(),
        handler.clone(),
    );
    tokio::spawn(async move { recordings_manager.run().await });

    tokio::spawn(async move { manager.run().await });

    let addr = SocketAddr::V4(SocketAddrV4::new(
        std::net::Ipv4Addr::LOCALHOST,
        get_free_port().expect("should find free TCP port"),
    ));

    std::thread::spawn(move || {
        run_server(addr, handler, recordings_manager_handler).expect("should start server");
    });

    wait_for_server(addr);

    let url = format!("http://{}", addr);
    run_window(url);
}

fn run_window(url: String) {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Ping Viewer")
        .with_window_icon(window_icon())
        .build(&event_loop)
        .expect("should create window");

    let builder = WebViewBuilder::new()
        .with_url(url)
        .with_devtools(cfg!(debug_assertions));

    #[cfg(not(target_os = "linux"))]
    let webview = builder.build(&window).expect("should create webview");

    #[cfg(target_os = "linux")]
    let webview = {
        let vbox = window
            .default_vbox()
            .expect("should get vertical box for window");
        builder
            .build_gtk(vbox)
            .expect("should create webview with gtk")
    };

    #[cfg(debug_assertions)]
    webview.open_devtools();
    #[cfg(not(debug_assertions))]
    let _ = webview;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        if let Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } = event
        {
            *control_flow = ControlFlow::Exit;
        }
    });
}

fn window_icon() -> Option<Icon> {
    let image = image::load_from_memory_with_format(
        include_bytes!("../icons/128x128.png"),
        image::ImageFormat::Png,
    )
    .ok()?
    .into_rgba8();
    let (width, height) = image.dimensions();
    Icon::from_rgba(image.into_raw(), width, height).ok()
}

fn get_free_port() -> std::io::Result<u16> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    Ok(listener.local_addr()?.port())
}

fn wait_for_server(addr: SocketAddr) {
    for _ in 0..100 {
        if std::net::TcpStream::connect(addr).is_ok() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    eprintln!("server at {addr} never came up");
}

#[actix_web::main]
async fn run_server(
    server_address: SocketAddr,
    handler: device::manager::ManagerActorHandler,
    recordings_handler: device::recording::RecordingsManagerHandler,
) -> std::io::Result<()> {
    server::manager::run(server_address, handler, recordings_handler).await
}
