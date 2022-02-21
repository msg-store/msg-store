use actix_web::HttpResponse;
use actix_web::web::{Data, Json};
use crate::AppData;
use msg_store_server_api::store::set::handle;
use log::{error, info};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fmt::Display;
use std::process::exit;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Info {
    max_byte_size: Option<u64>,
}

impl Display for Info {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", json!(self))
    }
}

const ROUTE: &'static str = "PUT /api/store";
pub async fn http_handle(data: Data<AppData>, info: Json<Info>) -> HttpResponse {
    info!("{} {}", ROUTE, info);
    let result = handle(
        &data.store, 
        &data.db,
        &data.file_storage, 
        &data.stats, 
        &data.configuration, 
        &data.configuration_path, 
        info.max_byte_size).await;
    if let Err(err) = result {
        error!("{} {}", ROUTE, err);
        exit(1);
    }
    HttpResponse::Ok().finish()
}
