use std::{env, fs::write, net::SocketAddr, path::PathBuf, sync::Arc};

use axum::{
    Extension, Router,
    middleware::from_extractor,
    routing::{get, get_service, patch, post},
};
use lazy_static::lazy_static;
use sqlx::{SqlitePool, migrate};
use tokio::signal;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    services::ServeDir,
    trace::TraceLayer,
};
use tracing_subscriber::EnvFilter;

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
use user::{Claim, authorize, register, reset_password};

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

#[tokio::main]
async fn main() {
    migrate(&CONFIG.database_path).await;
    let pool = SqlitePool::connect(&format!("sqlite://{}", CONFIG.database_path))
        .await
        .unwrap();
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("file-station=debug,tower_http=debug"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
    let app = Router::new()
        .nest(
            "/api/v1",
            Router::new()
                .route("/auth", post(authorize))
                .route("/users", post(register))
                .route("/user", patch(reset_password))
                .nest_service(
                    "/file/",
                    get_service(ServeDir::new(CONFIG.folder_path.clone()))
                        .layer(from_extractor::<Claim>())
                        .delete(delete_file)
                        .patch(rename_file)
                        .post(upload_file),
                )
                .route("/files/{*path}", get(get_folder).post(create_folder))
                .route("/files/", get(get_folder).post(create_folder))
                .route("/search", get(search_file))
                .route(
                    "/share",
                    get(get_share_file)
                        .post(add_share_file)
                        .delete(delete_share),
                )
                .route("/shares", get(get_share_index)),
        )
        .route("/assets/", get(static_handler))
        .fallback(static_handler)
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
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}
