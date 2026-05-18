mod routes;
mod state;
mod template;
mod utils;

use axum::{Router, routing::get};
use state::{CONFIG, DataBase};

use routes::{root, serve_file, upload};
use std::fs;

#[tokio::main]
async fn main() {
    dbg!(&CONFIG);
    if !std::path::Path::exists(&CONFIG.root_dir) {
        fs::create_dir(&CONFIG.root_dir).unwrap();
    }
    let db = DataBase::default();
    let app = Router::new()
        .route("/", get(root).post(upload))
        .route("/{filename}", get(serve_file))
        .with_state(db.clone());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
