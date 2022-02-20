use actix_web::{web::Data, HttpResponse};
use crate::AppData;
use msg_store_server_api::store::get::handle;
use log::{error, info};
use std::process::exit;

const ROUTE: &'static str = "GET /api/store";
pub async fn http_handle(data: Data<AppData>) -> HttpResponse {
    info!("{}", ROUTE);
    match handle(&data.store).await {
        Ok(store_data) => {
            info!("{} 200", ROUTE);
            HttpResponse::Ok().json(store_data)
        },
        Err(err) => {
            error!("{} {}", ROUTE, err);
            exit(1);
        }
    }
}
