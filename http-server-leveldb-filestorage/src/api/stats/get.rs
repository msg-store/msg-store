use actix_web::{web::Data, HttpResponse};
use crate::AppData;
use server_api::stats::get::handle;
use log::{error, info};
use std::process::exit;

const ROUTE: &'static str = "GET /api/stats";
pub fn http_handle(data: Data<AppData>) -> HttpResponse {
    info!("{}", ROUTE);
    match handle(&data.stats) {
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
