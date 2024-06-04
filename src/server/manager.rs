use super::protocols;
use actix_web::{middleware, App, HttpServer};
use paperclip::actix::OpenApiExt;
use tracing::info;

pub async fn run(server_address: &str) -> std::io::Result<()> {
    let server_address = server_address.to_string();
    info!("starting HTTP server at http://{server_address}");

    let server = HttpServer::new(|| {
        App::new()
            .wrap(middleware::Logger::default())
            .wrap_api()
            .with_json_spec_at("/api/spec")
            .with_swagger_ui_at("/docs")
            .configure(protocols::v1::rest::register_services)
            .service(protocols::v1::websocket::websocket)
            .build()
    });

    server.bind(server_address)?.run().await
}
