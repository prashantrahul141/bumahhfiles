mod gc;
mod routes;
mod state;
mod template;
mod utils;

use axum::{Router, extract::DefaultBodyLimit, http::Request, response::Response, routing::get};
use state::{CONFIG, DataBase};
use std::{net::SocketAddr, time::Duration};
use tower_http::{classify::ServerErrorsFailureClass, trace::TraceLayer};
use tracing::{Span, debug, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use routes::{delete_file, root, serve_file, stat, upload_file};
use std::fs;

use crate::state::DBEntry;

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

fn setup_files_dir() -> Vec<DBEntry> {
    if std::path::Path::exists(&CONFIG.root_dir) {
        info!(
            "files directory exists path={:?}, querying it.",
            CONFIG.root_dir
        );
        let dir = fs::read_dir(&CONFIG.root_dir).unwrap();
        let mut entries = vec![];
        for file in dir {
            if let Ok(file) = file
                && let Ok(metadata) = file.metadata()
                && let Ok(name) = file.file_name().into_string()
            {
                entries.push(DBEntry::new(name, metadata.len()));
            }
        }
        return entries;
    }
    info!(
        "files directory doesnt exist, creating new at path={:?}",
        CONFIG.root_dir
    );
    _ = fs::create_dir(&CONFIG.root_dir);
    vec![]
}

#[tokio::main]
async fn main() {
    setup_env();
    setup_tracing();
    info!("config = {CONFIG:?}");

    let existing_files = setup_files_dir();
    debug!("found {:?} existing files", existing_files.len());

    // app state
    let db = DataBase::from(existing_files.into_iter()).await;

    // axum app
    let app = Router::new()
        .route("/", get(root).post(upload_file))
        .route("/stat", get(stat))
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
