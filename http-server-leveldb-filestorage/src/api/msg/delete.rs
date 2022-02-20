use actix_web::web::{Data, Query};
use actix_web::HttpResponse;
use crate::AppData;
use log::{error, info};
use msg_store_server_api::msg::rm::handle;
use msg_store_uuid::Uuid;
use serde::{Deserialize, Serialize};
use std::process::exit;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Info {
    uuid: String,
}

const ROUTE: &'static str = "DEL /api/msg";
pub fn http_handle(data: Data<AppData>, info: Query<Info>) -> HttpResponse {
    info!("{}", ROUTE);
    let uuid = match Uuid::from_string(&info.uuid) {
        Ok(uuid) => uuid,
        Err(_error) => {
            info!("{} 400 {}", ROUTE, "InvalidUUID");
            return HttpResponse::BadRequest().body("InvalidUUID")
        }
    };
    match handle(&data.store,&data.db,&data.file_storage, &data.stats, uuid) {
        Ok(_) => {
            info!("{} 200", ROUTE);
            return HttpResponse::Ok().finish()
        },
        Err(err) => {
            error!("{} {}", ROUTE, err);
            exit(1);
        }
    }
}

// pub fn ws_handle(ctx: &mut ws::WebsocketContext<Websocket>, data: Data<AppData>, info: Value) {
//     http_route_hit_log(MSG_DELETE, Some(info.clone()));
//     let mut reply = ws_reply_with(ctx, MSG_DELETE);
//     let uuid = match get_required_uuid(&info) {
//         Ok(uuid) => uuid,
//         Err(message) => {
//             return reply(Reply::BadRequest(format!("/data/{}", message)));
//         }
//     };
//     reply(handle(data, uuid));
// }
