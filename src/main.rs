mod gc;
mod routes;
mod state;
mod template;
mod utils;

use axum::{Router, extract::DefaultBodyLimit, http::Request, response::Response, routing::get};
use state::{CONFIG, DataBase};
use std::{net::SocketAddr, time::Duration};
use tower_http::{classify::ServerErrorsFailureClass, trace::TraceLayer};
use tracing::{Span, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use routes::{delete_file, root, serve_file, upload_file};
use std::fs;

fn setup_env() {
    if dotenvy::dotenv().is_ok() {
        println!("loaded env");
    } else {
        eprintln!("failed to load env");
    }
}

fn setup_tracing() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!(
                    "{}=debug,tower_http=debug,axum::rejection=trace",
                    env!("CARGO_CRATE_NAME")
                )
                .into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    info!("logging setup done")
}

fn setup_files_dir() {
    if std::path::Path::exists(&CONFIG.root_dir) {
        info!("files directory exists, deleting it.");
        fs::remove_dir_all(&CONFIG.root_dir).unwrap();
    }
    info!("creating new files directory at path={:?}", CONFIG.root_dir);
    _ = fs::create_dir(&CONFIG.root_dir);
}

#[tokio::main]
async fn main() {
    setup_env();
    setup_tracing();
    info!("CONFIG = {CONFIG:?}");
    setup_files_dir();

    // app state
    let db = DataBase::default();

    // axum app
    let app = Router::new()
        .route("/", get(root).post(upload_file))
        .route("/{filename}", get(serve_file).delete(delete_file))
        .route("/d/{filename}", get(delete_file))
        .layer(DefaultBodyLimit::max(
            CONFIG.max_file_size * CONFIG.max_file_count * 2,
        ))
        .layer(
            TraceLayer::new_for_http()
                .on_request(|_request: &Request<_>, _span: &Span| {
                    tracing::debug!("new request -------------------")
                })
                .on_response(|_response: &Response, _latency: Duration, _span: &Span| {
                    tracing::debug!("response done -----------------")
                })
                .on_failure(
                    |_error: ServerErrorsFailureClass, _latency: Duration, _span: &Span| {
                        tracing::error!("request failed")
                    },
                ),
        )
        .with_state(db.clone());

    // setup gc
    gc::start_gc(db);

    // serve
    let addr = SocketAddr::new(CONFIG.internal_host.parse().unwrap(), CONFIG.internal_port);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    info!("listening at addr={addr}");
    axum::serve(listener, app).await.unwrap();
}
