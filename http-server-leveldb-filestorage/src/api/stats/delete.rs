use actix_web::{web::Data, HttpResponse};
use crate::AppData;
use msg_store_server_api::stats::rm::handle;
use log::{error, info};
use std::process::exit;

const ROUTE: &'static str = "DEL /api/stats";
pub async fn http_handle(data: Data<AppData>) -> HttpResponse {
    info!("{}", ROUTE);
    match handle(&data.stats).await {
        Ok(stats) => {
            info!("{} 200 {}", ROUTE, stats);
            HttpResponse::Ok().json(stats)
        },
        Err(err) => {
            error!("{} {}", ROUTE, err);
            exit(1);
        }
    }
}
