use actix::{Actor, Addr, AsyncContext, Handler, Message, StreamHandler};
use actix_web::HttpRequest;
use actix_web_actors::ws;
use lazy_static::lazy_static;
use paperclip::actix::{
    api_v2_operation, get,
    web::{self, HttpResponse},
    Apiv2Schema,
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tracing::{debug, info};
use uuid::Uuid;

pub struct StringMessage(String);

impl Message for StringMessage {
    type Result = ();
}

#[derive(Serialize, Debug)]
pub struct WebsocketError {
    pub error: String,
}

#[derive(Debug)]
pub struct WebsocketActorContent {
    pub actor: Addr<WebsocketActor>,
    pub re: Option<Regex>,
    pub device_number: Option<Uuid>,
}

#[derive(Debug, Default)]
pub struct WebsocketManager {
    pub clients: Vec<WebsocketActorContent>,
}

impl WebsocketManager {
    pub fn send(&self, value: &serde_json::Value, name: &str, device_number: &Option<Uuid>) {
        if self.clients.is_empty() {
            return;
        }

        let string = serde_json::to_string_pretty(value).unwrap();
        for client in &self.clients {
            // check client list was subscribed or subscribed to all
            if client.device_number.is_none()
                || client.device_number.unwrap() == device_number.unwrap()
            {
                let is_match = client.re.as_ref().map_or(false, |regx| regx.is_match(name));
                if is_match {
                    client.actor.do_send(StringMessage(string.clone()));
                }
            }
        }
    }
    pub fn get_client_count(&self) -> usize {
        self.clients.len()
    }
}

lazy_static! {
    pub static ref MANAGER: Arc<Mutex<WebsocketManager>> =
        Arc::new(Mutex::new(WebsocketManager::default()));
}

pub fn send_to_websockets(message: Value, device: Option<Uuid>) {
    MANAGER
        .lock()
        .unwrap()
        .send(&message, &message.to_string(), &device);
}

#[derive(Debug)]
pub struct WebsocketActor {
    server: Arc<Mutex<WebsocketManager>>,
    pub filter: String,
    pub device_number: Option<Uuid>,
}

impl WebsocketActor {
    pub fn new(message_filter: String, device_number: Option<Uuid>) -> Self {
        Self {
            server: MANAGER.clone(),
            filter: message_filter,
            device_number,
        }
    }
}

impl Handler<StringMessage> for WebsocketActor {
    type Result = ();

    fn handle(&mut self, message: StringMessage, context: &mut Self::Context) {
        context.text(message.0);
    }
}

impl Actor for WebsocketActor {
    type Context = ws::WebsocketContext<Self>;
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WebsocketActor {
    fn started(&mut self, ctx: &mut Self::Context) {
        info!("Starting websocket, add itself in manager.");
        self.server
            .lock()
            .unwrap()
            .clients
            .push(WebsocketActorContent {
                actor: ctx.address(),
                re: Regex::new(&self.filter).ok(),
                device_number: (self.device_number),
            });
    }

    fn finished(&mut self, ctx: &mut Self::Context) {
        info!("Finishing websocket, remove itself from manager.");
        self.server
            .lock()
            .unwrap()
            .clients
            .retain(|x| x.actor != ctx.address());
    }

    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Text(text)) => {
                let v: Vec<&str> = text.split("&&").collect();

                for request in v {
                    let request = request.trim();
                    if request.starts_with('/') {
                        ctx.text(request_endpoint(request));
                    } else {
                        let error_msg = format!(
                            "{} {}, missing / ?",
                            json!("Error: Invalid command:"),
                            request
                        );
                        ctx.text(error_msg)
                    }
                }
            }
            Ok(ws::Message::Close(msg)) => ctx.close(msg),
            _ => (),
        }
    }
}

#[api_v2_operation(skip)]
#[get("/ws")]
pub async fn websocket(
    req: HttpRequest,
    query: web::Query<WebsocketQuery>,
    stream: web::Payload,
) -> Result<HttpResponse, actix_web::Error> {
    let filter = match query.clone().into_inner().filter {
        Some(filter) => filter.clone(),
        _ => ".*".to_owned(),
    };

    let device_number = query.into_inner().device_number;

    debug!("New websocket with filter {:#?}", &filter);
    debug!("Device selected {:#?}", &device_number);

    ws::start(WebsocketActor::new(filter, device_number), &req, stream)
}

#[derive(Deserialize, Apiv2Schema, Clone)]
pub struct WebsocketQuery {
    /// Regex filter to select the desired incoming messages
    filter: Option<String>,
    device_number: Option<Uuid>,
}

fn request_endpoint(request: &str) -> String {
    let v: Vec<&str> = request.trim_start_matches('/').splitn(5, '/').collect();
    match v[0] {
        "create" => {
            // let _package = packages::reading(packages::Sensors::from_str(v[1]).unwrap(), false);
            json!("Ok: Command received").to_string()
        }
        _ => format!("{} {}", json!("Error: Invalid command:"), request),
    }
}
