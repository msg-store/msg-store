use actix_web::HttpResponse;
use actix_web::web::{Data, Query};
use crate::AppData;
use msg_store_server_api::group_defaults::rm::handle;
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::process::exit;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Info {
    priority: u16,
}

const ROUTE: &'static str = "DEL /api/group-defaults";
pub async fn http_handle(data: Data<AppData>, info: Query<Info>) -> HttpResponse {
    info!("{} priority: {}", ROUTE, info.priority);
    let result = handle(
        &data.store, 
        &data.configuration, 
        &data.configuration_path, 
        info.priority).await;
    if let Err(err) = result {
        error!("{} {}", ROUTE, err);
        exit(1);
    }
    info!("{} 200", ROUTE);
    HttpResponse::Ok().finish()
}
