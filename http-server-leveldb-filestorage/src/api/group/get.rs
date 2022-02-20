use crate::AppData;
use actix_web::{
    web::{Data, Query},
    HttpResponse,
};
use log::{error, info};
use msg_store_server_api::group::get;
use serde::{Deserialize, Serialize};
use std::process::exit;

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Info {
    priority: Option<u32>,
    include_msg_data: Option<bool>,
}

const ROUTE: &'static str = "GET /api/group";
pub fn http_handle(data: Data<AppData>, info: Query<Info>) -> HttpResponse {
    let include_msg_data = match info.include_msg_data {
        Some(include_msg_data) => include_msg_data,
        None => false
    };
    {
        let priority_string = match info.priority {
            Some(priority) => priority.to_string(),
            None => "N/A".to_string()
        };
        info!("{} priority: {}, includeMsgData: {}", ROUTE, priority_string, include_msg_data);
    }
    let result = get::handle(&data.store, info.priority, include_msg_data);
    match result {
        Ok(groups) => {
            info!("{} 200", ROUTE);
            HttpResponse::Ok().json(groups)
        },
        Err(err) => {
            error!("{} {}", ROUTE, err);
            exit(1);
        }
    }
}

// pub fn ws_handle(ctx: &mut WebsocketContext<Websocket>, data: Data<AppData>, info: Value) {
//     http_route_hit_log(GROUP_GET, Some(info.clone()));
//     let mut reply = ws_reply_with(ctx, GROUP_GET);
//     let info = match validate_params(info) {
//         Ok(info) => info,
//         Err(message) => {
//             reply(Reply::BadRequest(message));
//             return;
//         }
//     };
//     reply(handle(data, info));
// }
