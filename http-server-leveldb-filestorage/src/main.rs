use actix_web::{
    middleware,
    web::{self, Data},
    App, HttpServer,
};
use log::error;
use server_api::config::StoreConfig;
use server_api::file_storage::FileStorage;
use server_api::stats::Stats;
use msg_store::Store;
use msg_store_database_plugin::Db;
use env_logger::{Builder, Target};
use std::{
    path::PathBuf, sync::Mutex
};
use std::process::exit;

mod api;
mod init;

use init::init;


pub struct AppData {
    pub store: Mutex<Store>,
    pub configuration: Mutex<StoreConfig>,
    pub configuration_path: Option<PathBuf>,
    pub db: Mutex<Box<dyn Db>>,
    pub file_storage: Option<Mutex<FileStorage>>,
    pub stats: Mutex<Stats>
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "debug,actix_server=info,actix_web=error");
    // std::env::set_var("RUST_LOG", "debug,actix_server=info");
    let mut builder = Builder::from_default_env();
    builder.target(Target::Stdout).init();
    // env_logger::init();

    let init_result = match init() {
        Ok(init_result) => init_result,
        Err(error) => {
            error!("{}", error);
            exit(1)
        }
    };


    let app_data = Data::new(AppData {
        store: init_result.store,
        db: init_result.db,
        file_storage: init_result.file_storage, // TODO: Fix
        configuration_path: init_result.configuration_path,
        configuration: init_result.configuration,
        stats: init_result.stats
    });

    HttpServer::new(move || {
        App::new()
            // enable logger
            .wrap(middleware::Logger::default())
            .app_data(app_data.clone())
            .route("/api/export", web::get().to(api::export::http_handle))
            .route(
                "/api/group",
                web::delete().to(api::group::delete::handle_http),
            )
            .route("/api/group", web::get().to(api::group::get::http_handle))
            .route(
                "/api/group-defaults",
                web::delete().to(api::group_defaults::delete::http_handle),
            )
            .route(
                "/api/group-defaults",
                web::get().to(api::group_defaults::get::http_handle),
            )
            .route(
                "/api/group-defaults",
                web::post().to(api::group_defaults::post::http_handle),
            )
            .route("/api/msg", web::get().to(api::msg::get::http_handle))
            .route("/api/msg", web::delete().to(api::msg::delete::http_handle))
            .route("/api/msg", web::post().to(api::msg::post::http_handle))
            .route(
                "/api/stats",
                web::delete().to(api::stats::delete::http_handle),
            )
            .route("/api/stats", web::get().to(api::stats::get::http_handle))
            .route("/api/stats", web::put().to(api::stats::put::http_handle))
            .route("/api/store", web::get().to(api::store::get::http_handle))
            .route("/api/store", web::put().to(api::store::put::http_handle))
            // .service(web::resource("/ws").route(web::get().to(api::ws::ws_index)))
    })
    // start http server on 127.0.0.1:8080
    .bind(init_result.host)?
    .run()
    .await
}
