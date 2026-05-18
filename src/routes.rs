use crate::{
    state::{CONFIG, DBEntry, DataBase},
    template::IndexTemplate,
    utils::{BumAhhError, clean_filename, make_url_list, random},
};
use axum::{
    extract::{Multipart, Path, State},
    http::HeaderMap,
    response::{Html, IntoResponse},
};
use std::path;
use tokio::{fs, io::AsyncWriteExt};
use tower::ServiceExt;

pub async fn root() -> IndexTemplate {
    IndexTemplate {
        domain: format!("{}://{}", CONFIG.protocol, CONFIG.host),
    }
}

pub async fn upload_file(
    State(mut db): State<DataBase>,
    headers: HeaderMap,
    mut form: Multipart,
) -> Result<impl IntoResponse, BumAhhError> {
    let mut entries = vec![];
    while let Some(mut field) = form
        .next_field()
        .await
        .map_err(|err| BumAhhError::InvalidRequest(err.to_string()))?
        && let Some("file") = field.name()
    {
        let filename = field
            .file_name()
            .map_or(random(5).collect::<String>(), |x| {
                format!("{}-{}", random(5).collect::<String>(), clean_filename(x))
            });

        if filename.len() > CONFIG.max_filename_length {
            return Err(BumAhhError::InvalidRequest("Filename too long".into()));
        }

        let filepath = path::Path::new(&CONFIG.root_dir).join(&filename);
        let mut file = fs::File::create(&filepath).await?;

        let mut file_size: usize = 0;
        while let Some(chunk) = field
            .chunk()
            .await
            .map_err(|err| BumAhhError::InvalidRequest(err.to_string()))?
        {
            file_size = file_size
                .checked_add(chunk.len())
                .ok_or_else(|| BumAhhError::FileTooBig(CONFIG.max_file_size))?;

            file.write_all(chunk.as_ref())
                .await
                .map_err(BumAhhError::IO)?;

            if file_size > CONFIG.max_file_size {
                _ = fs::remove_file(filepath).await;
                return Err(BumAhhError::FileTooBig(CONFIG.max_file_size));
            }
        }

        if 0 == file_size {
            _ = fs::remove_file(filepath).await;
            continue;
        }

        entries.push(DBEntry::new(
            filename,
            file_size,
            random(5).collect::<String>(),
        ));
    }

    let accepts_html = headers
        .get("accept")
        .and_then(|s| s.to_str().ok())
        .is_some_and(|f| f.contains("html"));

    let response = Html(make_url_list(&entries, accepts_html));
    db.insert_mul(entries.into_iter()).await;
    Ok(response)
}

pub async fn serve_file(
    State(db): State<DataBase>,
    Path(filename): Path<String>,
    request: axum::extract::Request,
) -> Result<impl IntoResponse, BumAhhError> {
    match db.get_key(&filename).await {
        Some(entry) => {
            let path = CONFIG.root_dir.join(&entry.key);
            let service = tower_http::services::ServeFile::new(path);
            let response = service
                .oneshot(request)
                .await
                .map_err(|_| BumAhhError::FileNotFound)?;
            Ok(response)
        }
        None => Err(BumAhhError::FileNotFound),
    }
}
