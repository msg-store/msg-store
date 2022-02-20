use crate::AppData;
use actix_web::HttpResponse;
use actix_web::web::{Data, Query};
use log::{error, info};
use server_api::export::handle;
// use msg_store::api::error_codes::{self, log_err};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fmt::Display;
use std::path::PathBuf;
use std::process::exit;
use std::str::FromStr;

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Info {
    output_directory: String
}

impl Display for Info {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", json!(self))
    }
}

const ROUTE: &'static str = "GET /api/export";

pub async fn http_handle(data: Data<AppData>, info: Query<Info>) -> HttpResponse {
    info!("{} {}", ROUTE, info);
    let output_path = match PathBuf::from_str(&info.output_directory) {
        Ok(output_path) => output_path,
        Err(error) => {
            info!("{} 400 {}: {}", ROUTE, "Invalid Path", error.to_string());
            return HttpResponse::BadRequest().body("Invalid Path")
        }
    };
    let result = handle(
        &data.store, 
        &data.db, 
        &data.file_storage, 
        &data.stats, 
        // &data.configuration, 
        &output_path).await;
    if let Err(error) = result {
        // TODO: FIX program closes when the database dir is not found
        error!("{} {}", ROUTE, error);
        exit(1);
    }
    return HttpResponse::Ok().finish()
}
