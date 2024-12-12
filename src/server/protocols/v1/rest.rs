use crate::device::manager::{ManagerActorHandler, Request, UuidWrapper};
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
use uuid::Uuid;

#[cfg(not(feature = "embed-frontend"))]
#[derive(rust_embed::RustEmbed)]
#[folder = "src/server/protocols/v1/frontend"]
struct Asset;

#[cfg(feature = "embed-frontend")]
#[derive(rust_embed::RustEmbed)]
#[folder = "ping-viewer-next-frontend/dist"]
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

#[api_v2_operation(skip)]
#[get("/addons/{tail:.*}")]
async fn addons_handler() -> impl Responder {
    // Vue router handle /addons routes
    handle_embedded_file("index.html")
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
        .service(device_manager_get)
        .service(device_manager_post)
        .service(post_create)
        .service(device_manager_device_get)
        .service(device_manager_device_ping1d_get)
        .service(device_manager_device_ping360_get)
        .service(device_manager_device_common_get)
        .service(addons_handler)
        .service(cockpit_extras)
        .service(index_files);
}

async fn send_request_and_broadcast(
    manager_handler: &web::Data<ManagerActorHandler>,
    request: Request,
) -> Result<Json<crate::device::manager::Answer>, Error> {
    let request_has_id = match &request {
        Request::ModifyDevice(modify) => Some(modify.uuid),
        Request::Ping(device_request) => Some(device_request.uuid),
        Request::Delete(uuid_wrapper) => Some(uuid_wrapper.uuid),
        Request::Info(uuid_wrapper) => Some(uuid_wrapper.uuid),
        Request::EnableContinuousMode(uuid_wrapper) => Some(uuid_wrapper.uuid),
        Request::DisableContinuousMode(uuid_wrapper) => Some(uuid_wrapper.uuid),
        _ => None,
    };

    let answer = manager_handler.send(request).await?;
    crate::server::protocols::v1::websocket::send_to_websockets(json!(answer), request_has_id);
    Ok(Json(answer))
}

#[api_v2_operation(tags("Device Manager"))]
#[post("device_manager/request")]
async fn post_request(
    manager_handler: web::Data<ManagerActorHandler>,
    json: web::Json<crate::device::manager::Request>,
) -> Result<Json<crate::device::manager::Answer>, Error> {
    let request = json.into_inner();

    send_request_and_broadcast(&manager_handler, request).await
}

#[derive(Debug, Clone, Serialize, Deserialize, Apiv2Schema)]
pub enum DeviceManagerGetOptionsV1 {
    AutoCreate,
    List,
    Search,
}

#[derive(Debug, Clone, Serialize, Deserialize, Apiv2Schema)]
pub enum DeviceManagerPostOptionsV1 {
    Delete,
    Info,
    EnableContinuousMode,
    DisableContinuousMode,
}

#[api_v2_operation(tags("Device Manager"))]
#[get("device_manager/{selection}")]
async fn device_manager_get(
    manager_handler: web::Data<ManagerActorHandler>,
    selection: web::Path<DeviceManagerGetOptionsV1>,
) -> Result<Json<crate::device::manager::Answer>, Error> {
    let request = match selection.into_inner() {
        DeviceManagerGetOptionsV1::AutoCreate => crate::device::manager::Request::AutoCreate,
        DeviceManagerGetOptionsV1::List => crate::device::manager::Request::List,
        DeviceManagerGetOptionsV1::Search => crate::device::manager::Request::Search,
    };

    send_request_and_broadcast(&manager_handler, request).await
}

#[api_v2_operation(tags("Device Manager"))]
#[post("device_manager/create")]
async fn post_create(
    manager_handler: web::Data<ManagerActorHandler>,
    info: web::Json<crate::device::manager::CreateStruct>,
) -> Result<Json<crate::device::manager::Answer>, Error> {
    let create_struct = info.into_inner();

    let request = crate::device::manager::Request::Create(create_struct);

    send_request_and_broadcast(&manager_handler, request).await
}

#[api_v2_operation(tags("Device Manager : Device"))]
#[post("device_manager/{device}/{selection}")]
async fn device_manager_post(
    manager_handler: web::Data<ManagerActorHandler>,
    info: web::Path<(Uuid, DeviceManagerPostOptionsV1)>,
) -> Result<Json<crate::device::manager::Answer>, Error> {
    let info = info.into_inner();
    let uuid = info.0;
    let request = info.1;

    let request = match request {
        DeviceManagerPostOptionsV1::Delete => {
            crate::device::manager::Request::Delete(UuidWrapper { uuid })
        }
        DeviceManagerPostOptionsV1::Info => {
            crate::device::manager::Request::Info(UuidWrapper { uuid })
        }
        DeviceManagerPostOptionsV1::EnableContinuousMode => {
            crate::device::manager::Request::EnableContinuousMode(UuidWrapper { uuid })
        }
        DeviceManagerPostOptionsV1::DisableContinuousMode => {
            crate::device::manager::Request::DisableContinuousMode(UuidWrapper { uuid })
        }
    };

    send_request_and_broadcast(&manager_handler, request).await
}

#[api_v2_operation(tags("Device Manager : Device"))]
#[get("device_manager/{device}/{request}")]
async fn device_manager_device_get(
    manager_handler: web::Data<ManagerActorHandler>,
    info: web::Path<(Uuid, crate::device::devices::PingRequest)>,
) -> Result<Json<crate::device::manager::Answer>, Error> {
    let info = info.into_inner();
    let uuid = info.0;
    let request = info.1;

    let request =
        crate::device::manager::Request::Ping(crate::device::manager::DeviceRequestStruct {
            uuid,
            device_request: request,
        });

    send_request_and_broadcast(&manager_handler, request).await
}

#[api_v2_operation(tags("Device Manager : Device"))]
#[get("device_manager/{device}/ping1d/{request}")]
async fn device_manager_device_ping1d_get(
    manager_handler: web::Data<ManagerActorHandler>,
    info: web::Path<(Uuid, crate::device::devices::Ping1DRequest)>,
) -> Result<Json<crate::device::manager::Answer>, Error> {
    let info = info.into_inner();
    let uuid = info.0;
    let request = info.1;

    let request = crate::device::devices::PingRequest::Ping1D(request);

    let request =
        crate::device::manager::Request::Ping(crate::device::manager::DeviceRequestStruct {
            uuid,
            device_request: request,
        });

    send_request_and_broadcast(&manager_handler, request).await
}

#[api_v2_operation(tags("Device Manager : Device"))]
#[get("device_manager/{device}/ping360/{request}")]
async fn device_manager_device_ping360_get(
    manager_handler: web::Data<ManagerActorHandler>,
    info: web::Path<(Uuid, crate::device::devices::Ping360Request)>,
) -> Result<Json<crate::device::manager::Answer>, Error> {
    let info = info.into_inner();
    let uuid = info.0;
    let request = info.1;

    let request = crate::device::devices::PingRequest::Ping360(request);

    let request =
        crate::device::manager::Request::Ping(crate::device::manager::DeviceRequestStruct {
            uuid,
            device_request: request,
        });

    send_request_and_broadcast(&manager_handler, request).await
}

#[api_v2_operation(tags("Device Manager : Device"))]
#[get("device_manager/{device}/common/{request}")]
async fn device_manager_device_common_get(
    manager_handler: web::Data<ManagerActorHandler>,
    info: web::Path<(Uuid, crate::device::devices::PingCommonRequest)>,
) -> Result<Json<crate::device::manager::Answer>, Error> {
    let info = info.into_inner();
    let uuid = info.0;
    let request = info.1;

    let request = crate::device::devices::PingRequest::Common(request);

    let request =
        crate::device::manager::Request::Ping(crate::device::manager::DeviceRequestStruct {
            uuid,
            device_request: request,
        });

    send_request_and_broadcast(&manager_handler, request).await
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
    pub extras: Extras,
}

#[derive(Debug, Serialize, Deserialize, Apiv2Schema)]
pub struct Extras {
    pub cockpit: &'static str,
}

#[derive(Debug, Serialize, Deserialize, Apiv2Schema)]
pub struct CockpitExtras {
    pub target_system: String,
    pub target_cockpit_api_version: String,
    pub widgets: Vec<CockpitWidget>,
}

#[derive(Debug, Serialize, Deserialize, Apiv2Schema)]
pub struct CockpitWidget {
    name: String,
    config_iframe_url: Option<String>,
    iframe_url: String,
    iframe_icon: String,
    version: String,
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
            extras: Extras {
                cockpit: "/cockpit_extras.json",
            },
        }
    }
}

#[api_v2_operation]
#[get("/cockpit_extras.json")]
async fn cockpit_extras(
    manager_handler: web::Data<ManagerActorHandler>,
) -> Result<Json<CockpitExtras>, Error> {
    let devices = match manager_handler.send(Request::List).await {
        Ok(crate::device::manager::Answer::DeviceInfo(devices)) => devices,
        Ok(unexpected) => {
            return Err(Error::Internal(format!(
                "Unexpected response from device manager: {:?}",
                unexpected
            )))
        }
        Err(err) => {
            return Err(Error::Internal(format!(
                "Unexpected error from device manager: {:?}",
                err
            )))
        }
    };

    let widgets = devices
        .into_iter()
        .filter_map(|device| {
            let name = match device.device_type {
                crate::device::manager::DeviceSelection::Ping1D => Some("ping1d"),
                crate::device::manager::DeviceSelection::Ping360 => Some("ping360"),
                _ => None,
            }?;

            Some(CockpitWidget {
                name: name.to_string(),
                config_iframe_url: None,
                iframe_url: format!("/addons/widget/{}/?uuid={}", name, device.id),
                iframe_icon: format!("/images/{}.png", name),
                version: "1.0.0".to_string(),
            })
        })
        .collect();

    Ok(Json(CockpitExtras {
        target_system: "Cockpit".to_string(),
        target_cockpit_api_version: "1.0.0".to_string(),
        widgets,
    }))
}
