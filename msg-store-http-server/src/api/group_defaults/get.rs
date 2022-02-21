use actix_web::HttpResponse;
use actix_web::web::{Data, Query};
use crate::AppData;
use msg_store_server_api::group_defaults::get::handle;
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::process::exit;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Info {
    priority: Option<u16>,
}

const ROUTE: &'static str = "GET /api/group-defaults";
pub async fn http_handle(data: Data<AppData>, info: Query<Info>) -> HttpResponse {
    let result = handle(&data.store, info.priority).await;
    match result {
        Ok(groups) => {
            info!("{} 200", ROUTE);
            HttpResponse::Ok().json(groups)
        },
        Err(err) => {
            error!("{} {}", ROUTE, err);
            exit(1)
        }
    }
}
