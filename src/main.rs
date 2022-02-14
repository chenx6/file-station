use std::{
    env,
    fs::{canonicalize, create_dir, write},
    net::SocketAddr,
    path::PathBuf,
};

use axum::{
    handler::Handler,
    routing::{get, patch, post},
    AddExtensionLayer, Router,
};
use lazy_static::lazy_static;
use sqlx::{migrate, SqlitePool};
use tower_http::{
    compression::CompressionLayer,
    cors::{any, CorsLayer},
};

mod dist;
mod file;
mod user;

use dist::static_handler;
use file::{
    file::{delete_file, get_file, rename_file, search_file, upload_file},
    folder::{create_folder, get_folder},
    share::{add_share_file, delete_share, get_share_file, get_share_index},
};
use user::{authorize, register, reset_password};

lazy_static! {
    pub static ref FOLDER: PathBuf = {
        let path = env::var("FS_FOLDER").unwrap_or(String::from("./files/"));
        let path = PathBuf::from(path);
        if !path.exists() {
            create_dir(&path).unwrap();
        }
        canonicalize(path).unwrap()
    };
}

/// Perform migration if database is not exist
pub async fn migrate(db_url: &str) {
    if !PathBuf::from(db_url).exists() {
        write(db_url, "").unwrap();
        let pool = SqlitePool::connect(&format!("sqlite://{}", db_url))
            .await
            .unwrap();
        migrate!().run(&pool).await.unwrap();
    }
}

#[tokio::main]
async fn main() {
    let db_url = env::var("FS_DATABASE").unwrap_or("database.db".to_string());
    migrate(&db_url).await;
    let pool = SqlitePool::connect(&format!("sqlite://{}", db_url))
        .await
        .unwrap();
    let app = Router::new()
        .nest(
            "/api/v1",
            Router::new()
                .route("/auth", post(authorize))
                .route("/users", post(register))
                .route("/user", patch(reset_password))
                .route(
                    "/file",
                    get(get_file)
                        .delete(delete_file)
                        .patch(rename_file)
                        .post(upload_file),
                )
                .route("/files", get(get_folder).post(create_folder))
                .route("/search", get(search_file))
                .route(
                    "/share",
                    get(get_share_file)
                        .post(add_share_file)
                        .delete(delete_share),
                )
                .route("/shares", get(get_share_index)),
        )
        .route("/assets/", static_handler.into_service())
        .fallback(static_handler.into_service())
        .layer(
            CorsLayer::new()
                .allow_methods(any())
                .allow_headers(any())
                .allow_origin(any()),
        )
        .layer(CompressionLayer::new().br(true))
        .layer(AddExtensionLayer::new(pool));
    let addr = SocketAddr::from(([127, 0, 0, 1], 5000)); // TODO Accept user input address
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
