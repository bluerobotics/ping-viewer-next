use crate::device::manager::ManagerActorHandler;

use super::protocols;
use actix_cors::Cors;
use actix_web::{middleware, web::Data, App, HttpServer};
use tracing::info;

use paperclip::actix::{
    web::{self, Scope},
    OpenApiExt,
};

fn add_v1_paths(scope: Scope) -> Scope {
    scope.configure(protocols::v1::rest::register_services)
}

pub async fn run(server_address: &str, handler: ManagerActorHandler) -> std::io::Result<()> {
    let server_address = server_address.to_string();
    info!("ServerManager: Service starting");

    let server = HttpServer::new(move || {
        let cors = Cors::permissive();

        let v1 = add_v1_paths(web::scope("/v1"));
        let default = add_v1_paths(web::scope(""));

        App::new()
            .app_data(Data::new(handler.clone()))
            .wrap(cors)
            .wrap(middleware::Logger::default())
            .wrap_api()
            .with_json_spec_at("/api/spec")
            .with_swagger_ui_at("/docs")
            .service(v1)
            .service(protocols::v1::rest::server_metadata)
            .service(protocols::v1::websocket::websocket)
            .service(default)
            .build()
    });

    info!("ServerManager: HTTP server running at http://{server_address}");
    server.bind(server_address)?.run().await
}
