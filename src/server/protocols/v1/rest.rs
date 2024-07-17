use crate::device::manager::ManagerActorHandler;
use crate::server::protocols::v1::errors::Error;
use actix_web::Responder;
use mime_guess::from_path;
use paperclip::actix::{
    api_v2_operation, get, post,
    web::{self, HttpResponse, Json},
    Apiv2Schema,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

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

#[api_v2_operation(skip)]
#[get("/{file_path:.*}")]
async fn index_files(file_path: web::Path<String>) -> impl Responder {
    handle_embedded_file(&file_path)
}

/// The "register_service" route is used by BlueOS extensions manager
#[api_v2_operation]
#[get("register_service")]
async fn server_metadata() -> Result<Json<ServerMetadata>, Error> {
    let package = ServerMetadata::default();
    Ok(Json(package))
}

pub fn register_services(cfg: &mut web::ServiceConfig) {
    cfg.service(index)
        .service(post_request)
        .service(index_files);
}

#[api_v2_operation]
#[post("device/request")]
async fn post_request(
    manager_handler: web::Data<ManagerActorHandler>,
    json: web::Json<crate::device::manager::Request>,
) -> Result<Json<crate::device::manager::Answer>, Error> {
    let request = json.into_inner();

    let answer = manager_handler.send(request).await?;

    // Broadcast the results to webscoket clients.
    crate::server::protocols::v1::websocket::send_to_websockets(json!(answer), None);

    Ok(Json(answer))
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
            name: "Ping Viewer Next",
            description: "A ping protocol extension for expose devices to web.",
            icon: "mdi-compass-outline",
            company: "BlueRobotics",
            version: "0.0.0",
            new_page: false,
            webpage: "https://github.com/RaulTrombin/navigator-assistant",
            api: "/docs",
        }
    }
}
