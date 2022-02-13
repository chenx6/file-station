use std::net::SocketAddr;

use axum::{
    handler::Handler,
    routing::{get, patch, post},
    AddExtensionLayer, Router,
};
use sqlx::SqlitePool;
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

#[tokio::main]
async fn main() {
    let pool = SqlitePool::connect("sqlite://database.db").await.unwrap();
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
