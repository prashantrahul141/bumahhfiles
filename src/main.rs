mod routes;
mod state;
mod template;
mod utils;

use axum::{Router, routing::get};
use state::CONFIG;

use routes::root;
use std::fs;

use routes::upload;
use state::DataBase;

#[tokio::main]
async fn main() {
    dbg!(&CONFIG);
    if !std::path::Path::exists(&CONFIG.root_dir) {
        fs::create_dir(&CONFIG.root_dir).unwrap();
    }
    let app = Router::new().route("/", get(root));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
