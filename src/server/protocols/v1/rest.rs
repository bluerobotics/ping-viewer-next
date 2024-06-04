use crate::device::manager::{DeviceManager, DeviceSelection, SourceSelection};
use crate::server::protocols::v1::errors::Error;
use actix_web::Responder;
use mime_guess::from_path;
use paperclip::actix::{
    api_v2_operation, get, post,
    web::{self, HttpResponse, Json},
    Apiv2Schema,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// use std::vec;
// use validator::Validate;

#[derive(rust_embed::RustEmbed)]
#[folder = "src/server/protocols/v1/frontend"]
struct Asset;

fn handle_embedded_file(path: &str) -> HttpResponse {
    match Asset::get(path) {
        Some(content) => HttpResponse::Ok()
            .content_type(from_path(path).first_or_octet_stream().as_ref())
            .body(content.data.into_owned()),
        None => HttpResponse::NotFound().body("404 Not Found"),
    }
}

#[api_v2_operation(skip)]
#[get("/")]
async fn index() -> impl Responder {
    handle_embedded_file("index.html")
}

/// The "register_service" route is used by BlueOS extensions manager
// #[api_v2_operation]
// #[get("register_service")]
// async fn get_server_metadata() -> Result<Json<ServerMetadata>, Error> {
//     let package = ServerMetadata::default();
//     Ok(Json(package))
// }

pub fn register_services(cfg: &mut web::ServiceConfig) {
    cfg.service(index)
        .service(init)
        .service(stop)
        .service(post_create_udp)
        .service(post_create_serial)
        .service(list)
        .service(ws_start);
    // .service(get_server_metadata);
}

#[api_v2_operation]
#[get("v1/settings/{device_number}/init")]
async fn init(
    device_number: web::Path<Uuid>,
) -> Result<Json<crate::server::protocols::v1::structure::AnsPackage>, Error> {
    // let package = packages::init();
    DeviceManager::init(device_number.into_inner()).await;
    let package = crate::server::protocols::v1::structure::AnsPackage::new(
        crate::server::protocols::v1::structure::Operation::Settings,
    );
    Ok(Json(package))
}

#[api_v2_operation]
#[get("v1/settings/{device_number}/stop")]
async fn stop(
    device_number: web::Path<Uuid>,
) -> Result<Json<crate::server::protocols::v1::structure::AnsPackage>, Error> {
    DeviceManager::stop(device_number.into_inner()).await;
    let package = crate::server::protocols::v1::structure::AnsPackage::new(
        crate::server::protocols::v1::structure::Operation::Settings,
    );
    Ok(Json(package))
}

#[api_v2_operation]
#[post("v1/settings/create/udp")]
async fn post_create_udp(
    json: web::Json<crate::device::manager::SourceUdpStruct>,
) -> Result<Json<crate::server::protocols::v1::structure::AnsPackage>, Error> {
    let source_type = json.into_inner();
    let source_type = SourceSelection::UdpStream(source_type);
    let device_selector = DeviceSelection::Ping1D;
    DeviceManager::create(source_type, device_selector).await;
    let package = crate::server::protocols::v1::structure::AnsPackage::new(
        crate::server::protocols::v1::structure::Operation::Settings,
    );
    Ok(Json(package))
}

#[api_v2_operation]
#[post("v1/settings/create/serial")]
async fn post_create_serial(
    json: web::Json<crate::device::manager::SourceSerialStruct>,
) -> Result<Json<crate::server::protocols::v1::structure::AnsPackage>, Error> {
    let source_type = json.into_inner();
    let source_type = SourceSelection::SerialStream(source_type);
    let device_selector = DeviceSelection::Ping1D;
    DeviceManager::create(source_type, device_selector).await;
    let package = crate::server::protocols::v1::structure::AnsPackage::new(
        crate::server::protocols::v1::structure::Operation::Settings,
    );
    Ok(Json(package))
}

#[api_v2_operation]
#[get("v1/settings/list")]
async fn list() -> Result<Json<crate::server::protocols::v1::structure::DevicesList>, Error> {
    let list = DeviceManager::list().await;
    let package = crate::server::protocols::v1::structure::DevicesList { list };
    Ok(Json(package))
}

#[api_v2_operation]
#[get("v1/settings/ws_clients")]
async fn ws_clients() -> Result<Json<crate::server::protocols::v1::structure::AnsPackage>, Error> {
    crate::server::protocols::v1::websocket::MANAGER
        .lock()
        .unwrap()
        .get_client_count();
    let package = crate::server::protocols::v1::structure::AnsPackage::new(
        crate::server::protocols::v1::structure::Operation::Settings,
    );
    Ok(Json(package))
}

#[api_v2_operation]
#[get("v1/settings/ws_start/{device_number}")]
async fn ws_start(
    device_number: web::Path<Uuid>,
) -> Result<Json<crate::server::protocols::v1::structure::AnsPackage>, Error> {
    DeviceManager::subscribe(Some(device_number.into_inner())).await;
    let package = crate::server::protocols::v1::structure::AnsPackage::new(
        crate::server::protocols::v1::structure::Operation::Settings,
    );
    Ok(Json(package))
}

#[derive(Debug, Serialize, Deserialize, Apiv2Schema)]
pub struct ServerMetadata {
    pub name: &'static str,
    pub description: &'static str,
    pub icon: &'static str,
    pub company: &'static str,
    pub version: &'static str,
    pub new_page: bool,
    pub webpage: &'static str,
    pub api: &'static str,
}

impl Default for ServerMetadata {
    fn default() -> Self {
        Self {
            name: "Navigator Assistant",
            description: "A navigator extension to expose navigator to web.",
            icon: "mdi-compass-outline",
            company: "BlueRobotics",
            version: "0.0.1",
            new_page: false,
            webpage: "https://github.com/RaulTrombin/navigator-assistant",
            api: "/docs",
        }
    }
}
