use actix::{
    dev::ContextFutureSpawner, fut, Actor, ActorFutureExt, Addr, AsyncContext, Handler, Message,
    StreamHandler, WrapFuture,
};
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
use tracing::info;
use uuid::Uuid;

use crate::device::manager::{ManagerActorHandler, Request};

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
    pub fn send(&self, value: &serde_json::Value, name: &str, device_number: Option<Uuid>) {
        if self.clients.is_empty() {
            return;
        }

        let string = serde_json::to_string(value).unwrap();
        for client in &self.clients {
            // check client list was subscribed or subscribed to all
            if client.device_number.is_none() || client.device_number == device_number {
                let is_match = client.re.as_ref().map_or(false, |regx| regx.is_match(name));
                if is_match {
                    client.actor.do_send(StringMessage(string.clone()));
                }
            }
        }
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
        .send(&message, &message.to_string(), device);
}

pub struct WebsocketActor {
    server: Arc<Mutex<WebsocketManager>>,
    pub filter: String,
    pub device_number: Option<Uuid>,
    pub manager_handler: web::Data<ManagerActorHandler>,
}

impl WebsocketActor {
    pub fn new(
        message_filter: String,
        device_number: Option<Uuid>,
        manager_handler: web::Data<ManagerActorHandler>,
    ) -> Self {
        Self {
            server: MANAGER.clone(),
            filter: message_filter,
            device_number,
            manager_handler,
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
        info!("ServerManager: Starting websocket client, add itself in manager.");
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
        info!("ServerManager: Finishing websocket, remove itself from manager.");
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
                let manager_requests: Vec<Request> = match serde_json::from_str(&text) {
                    Ok(requests) => requests,
                    Err(err) => match serde_json::from_str(&text) {
                        Ok(request) => vec![request],
                        Err(_) => {
                            let error_msg = format!("Error: {}", err);
                            ctx.text(error_msg);
                            return;
                        }
                    },
                };

                for request in manager_requests {
                    let manager_handler = self.manager_handler.clone();

                    let future =
                        async move { manager_handler.send(request).await }.into_actor(self);

                    future
                        .then(|res, _, ctx| {
                            match &res {
                                Ok(result) => {
                                    crate::server::protocols::v1::websocket::send_to_websockets(
                                        json!(result),
                                        None,
                                    );
                                }
                                Err(err) => {
                                    ctx.text(serde_json::to_string_pretty(err).unwrap());
                                }
                            }
                            fut::ready(())
                        })
                        .wait(ctx);
                }
            }
            Ok(ws::Message::Close(msg)) => ctx.close(msg),
            _ => (),
        }
    }
}

#[api_v2_operation(skip)]
#[get("ws")]
pub async fn websocket(
    req: HttpRequest,
    query: web::Query<WebsocketQuery>,
    stream: web::Payload,
    manager_handler: web::Data<ManagerActorHandler>,
) -> Result<HttpResponse, actix_web::Error> {
    let filter = match query.clone().into_inner().filter {
        Some(filter) => filter.clone(),
        _ => ".*".to_owned(),
    };
    let device_number = query.into_inner().device_number;

    if let Some(device_number) = device_number {
        let request = crate::device::manager::Request::Info(device_number);
        match manager_handler.send(request).await {
            Ok(response) => {
                info!(
                    "ServerManager: Received websocket request connection for device: {response:?}"
                );
            }
            Err(err) => {
                return Ok(HttpResponse::InternalServerError().json(json!(err)));
            }
        }
    }

    ws::start(
        WebsocketActor::new(filter, device_number, manager_handler.clone()),
        &req,
        stream,
    )
}

#[derive(Deserialize, Apiv2Schema, Clone)]
pub struct WebsocketQuery {
    /// Regex filter to select the desired incoming messages
    filter: Option<String>,
    device_number: Option<Uuid>,
}
