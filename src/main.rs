use std::{env, fs::write, net::SocketAddr, path::PathBuf, sync::Arc};

use axum::{
    extract::extractor_middleware,
    handler::Handler,
    routing::{get, get_service, patch, post},
    Extension, Router,
};
use lazy_static::lazy_static;
use sqlx::{migrate, SqlitePool};
use tokio::signal;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    services::ServeDir,
    trace::TraceLayer,
};

mod config;
mod dist;
mod file;
mod user;

use config::Config;
use dist::static_handler;
use file::{
    file::{delete_file, rename_file, search_file, upload_file},
    folder::{create_folder, get_folder},
    share::{add_share_file, delete_share, get_share_file, get_share_index},
};
use user::{authorize, register, reset_password, Claim};

lazy_static! {
    pub static ref CONFIG: Arc<Config> = Arc::new(Config::from_env());
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

/// Shutdown signal handler, stop the loop
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    println!("signal received, starting graceful shutdown");
}

async fn handle_file_error(_: std::io::Error) {
    ()
}

#[tokio::main]
async fn main() {
    migrate(&CONFIG.database_path).await;
    let pool = SqlitePool::connect(&format!("sqlite://{}", CONFIG.database_path))
        .await
        .unwrap();
    // Set the RUST_LOG, if it hasn't been explicitly defined
    if env::var_os("RUST_LOG").is_none() {
        env::set_var("RUST_LOG", "file-station=debug,tower_http=debug")
    }
    tracing_subscriber::fmt::init();
    let app = Router::new()
        .nest(
            "/api/v1",
            Router::new()
                .route("/auth", post(authorize))
                .route("/users", post(register))
                .route("/user", patch(reset_password))
                .nest(
                    "/file/",
                    get_service(ServeDir::new(CONFIG.folder_path.clone()))
                        .handle_error(handle_file_error)
                        .layer(extractor_middleware::<Claim>())
                        .delete(delete_file)
                        .patch(rename_file)
                        .post(upload_file),
                )
                .route("/files/*path", get(get_folder).post(create_folder))
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
                .allow_methods(Any)
                .allow_headers(Any)
                .allow_origin(Any),
        )
        .layer(CompressionLayer::new().gzip(true).deflate(true).br(true))
        .layer(Extension(pool))
        .layer(TraceLayer::new_for_http().on_request(()));
    let addr: SocketAddr = env::var("FS_LISTEN")
        .unwrap_or("127.0.0.1:5000".to_string())
        .parse()
        .unwrap();
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}
