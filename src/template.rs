use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};

use crate::state::DBEntry;

pub struct Stat {
    pub files_serving_count: usize,
    pub storage_used_percent: u64,
    pub version: &'static str,
}

#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate {
    pub domain: String,
    pub stat: Option<Stat>,
}

#[derive(Template)]
#[template(path = "url_list.html")]
pub struct UrlListTemplate<'a> {
    pub entries: &'a [DBEntry],
}

#[derive(Template)]
#[template(path = "url_list_raw.html")]
pub struct UrlListRawTemplate<'a> {
    pub entries: &'a [DBEntry],
}

pub struct HtmlTemplate<T>(pub T);

impl<T> IntoResponse for HtmlTemplate<T>
where
    T: Template,
{
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(html) => Html(html).into_response(),
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to render template. Error: {err}"),
            )
                .into_response(),
        }
    }
}
