mod gc;
mod routes;
mod state;
mod template;
mod utils;

use axum::{Router, http::Request, routing::get};
use state::{CONFIG, DataBase};
use std::time::Duration;
use tower_http::{classify::ServerErrorsFailureClass, trace::TraceLayer};
use tracing::{Span, info_span};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use routes::{delete_file, root, serve_file, upload_file};
use std::fs;

#[tokio::main]
async fn main() {
    // setup logging
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

    // setup files directory
    if std::path::Path::exists(&CONFIG.root_dir) {
        fs::remove_dir_all(&CONFIG.root_dir).unwrap();
    }
    _ = fs::create_dir(&CONFIG.root_dir);

    // app state
    let db = DataBase::default();

    // axum app
    let app = Router::new()
        .route("/", get(root).post(upload_file))
        .route("/{filename}", get(serve_file).delete(delete_file))
        .route("/d/{filename}", get(delete_file))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &Request<_>| {
                    info_span!(
                        "REQ",
                        "{} | {}",
                        request.method(),
                        request.uri().to_string()
                    )
                })
                .on_failure(
                    |_error: ServerErrorsFailureClass, _latency: Duration, _span: &Span| {
                        tracing::error!("something went wrong")
                    },
                ),
        )
        .with_state(db.clone());

    // setup gc
    gc::start_gc(db);

    // serve
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
