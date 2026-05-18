use crate::{state::CONFIG, template::IndexTemplate};
use axum::{
    extract::{Multipart, State},
    http::HeaderMap,
    response::{Html, IntoResponse},
};

pub async fn root() -> IndexTemplate {
    IndexTemplate {
        domain: format!("{}://{}", CONFIG.protocol, CONFIG.host),
    }
}
