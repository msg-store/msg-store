use actix::prelude::*;
// use actix_files as fs;
use crate::{api, AppData};
use actix_web::{
    // middleware,
    web,
    // HttpServer,
    web::Data,
    // App,
    Error,
    HttpRequest,
    HttpResponse,
};
use actix_web_actors::ws;
use log::error;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::process::exit;

use super::{from_value_prop_required, get_required_string, ws_bad_request, http_route_hit_log, ws_not_found};

/// do websocket handshake and start `MyWebSocket` actor
pub async fn ws_index(
    r: HttpRequest,
    stream: web::Payload,
    data: Data<AppData>,
) -> Result<HttpResponse, Error> {
    // println!("{:?}", r);
    let res = ws::start(Websocket::new(data), &r, stream);
    // println!("{:?}", res);
    res
}

/// websocket connection is long running connection, it easier
/// to handle with an actor
pub struct Websocket {
    /// Client must send ping at least once per 10 seconds (CLIENT_TIMEOUT),
    /// otherwise we drop connection.
    // hb: Instant,
    app_data: Data<AppData>,
}

impl Actor for Websocket {
    type Context = ws::WebsocketContext<Self>;

    /// Method is called on actor start. We start the heartbeat process here.
    fn started(&mut self, _ctx: &mut Self::Context) {
        // self.hb(ctx);
    }
}

pub mod command {

    pub const MSG_GET: &'static str = "msg/get";
    pub const MSG_POST: &'static str = "msg/post";
    pub const MSG_DELETE: &'static str = "msg/delete";

    pub const GROUP_GET: &'static str = "group/get";
    pub const GROUP_DELETE: &'static str = "group/delete";

    pub const GROUP_DEFAULTS_GET: &'static str = "group-defaults/get";
    pub const GROUP_DEFAULTS_POST: &'static str = "group-defaults/post";
    pub const GROUP_DEFAULTS_DELETE: &'static str = "group-defaults/delete";

    pub const STORE_GET: &'static str = "store/get";
    pub const STORE_PUT: &'static str = "store/put";

    pub const STATS_GET: &'static str = "stats/get";
    pub const STATS_PUT: &'static str = "stats/put";
    pub const STATS_DELETE: &'static str = "stats/delete";

    pub const EXPORT: &'static str = "export";
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum Response {
    BadRequest { code: u16, message: &'static str },
    NotFound { code: u16, message: &'static str },
}

impl From<Response> for String {
    fn from(response: Response) -> Self {
        match serde_json::to_string(&response) {
            Ok(text) => text,
            Err(error) => {
                error!("ERROR_CODE: 4806076e-4ce9-45a2-b753-c423046d8263. Could not convert response into string: {}", error.to_string());
                exit(1);
            }
        }
    }
}

/// Handler for `ws::Message`
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for Websocket {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {}
            Ok(ws::Message::Text(text)) => {

                http_route_hit_log("/api/ws", Some(text.clone()));

                let obj = match serde_json::from_str::<Value>(&text) {
                    Ok(obj) => obj,
                    Err(_error) => {
                        ctx.text(Response::BadRequest {
                            code: 400,
                            message: "body must be a json object",
                        });
                        return;
                    }
                };
                let route = match get_required_string(&obj, "cmd") {
                    Ok(route) => route,
                    Err(message) => {
                        return ctx.text(ws_bad_request("unknown", format!("/{}", message)))
                    }
                };
                let data = match from_value_prop_required::<Value>(&obj, "data", "object") {
                    Ok(data) => data,
                    Err(message) => {
                        return ctx.text(ws_bad_request("unknown", format!("/{}", message)));
                    }
                };
                let app_data = self.app_data.clone();
                if route == command::MSG_GET {
                    // api::msg::get::ws_handle(ctx, app_data, data);
                } else if route == command::MSG_POST {
                    // api::msg::post::ws_handle(ctx, app_data, data);
                } else if route == command::MSG_DELETE {
                    api::msg::delete::ws_handle(ctx, app_data, data);
                } else if route == command::GROUP_GET {
                    api::group::get::ws_handle(ctx, app_data, data);
                } else if route == command::GROUP_DELETE {
                    api::group::delete::handle_ws(ctx, app_data, data);
                } else if route == command::GROUP_DEFAULTS_GET {
                    api::group_defaults::get::ws_handle(ctx, app_data, data);
                } else if route == command::GROUP_DEFAULTS_POST {
                    api::group_defaults::post::ws_handle(ctx, app_data, data);
                } else if route == command::GROUP_DEFAULTS_DELETE {
                    api::group_defaults::delete::ws_handle(ctx, app_data, data);
                } else if route == command::STORE_GET {
                    api::store::get::ws_handle(ctx, app_data);
                } else if route == command::STORE_PUT {
                    api::store::put::ws_handle(ctx, app_data, data);
                } else if route == command::STATS_GET {
                    api::stats::get::ws_handle(ctx, app_data);
                } else if route == command::STATS_PUT {
                    api::stats::put::ws_handle(ctx, app_data, data);
                } else if route == command::STATS_DELETE {
                    api::stats::delete::ws_handle(ctx, app_data);
                } else if route == command::EXPORT {
                    api::export::ws_handle(ctx, app_data, data);
                } else {
                    ctx.text(ws_not_found(&route, "/cmd is unknown".to_string()));
                }
            }
            Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            _ => ctx.stop(),
        }
    }
}

impl Websocket {
    fn new(app_data: Data<AppData>) -> Self {
        Self { app_data }
    }
}
