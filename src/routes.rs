use crate::{
    state::{CONFIG, DBEntry, DataBase},
    template::{HtmlTemplate, IndexTemplate},
    utils::{BumAhhError, clean_filename, make_url_list, random},
};
use axum::{
    extract::{Multipart, Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse},
};
use serde::Deserialize;
use std::path;
use tokio::{fs, io::AsyncWriteExt};
use tower::ServiceExt;
use tracing::{error, warn};

pub async fn root() -> impl IntoResponse {
    HtmlTemplate(IndexTemplate {
        domain: format!("{}://{}", CONFIG.external_protocol, CONFIG.external_host),
    })
}

pub async fn upload_file(
    State(db): State<DataBase>,
    headers: HeaderMap,
    mut form: Multipart,
) -> Result<impl IntoResponse, (StatusCode, BumAhhError)> {
    let mut entries: Vec<DBEntry> = vec![];
    let mut total_entries_size: u64 = 0;
    while let Some(mut field) = form.next_field().await.map_err(|err| {
        (
            StatusCode::BAD_REQUEST,
            BumAhhError::InvalidRequest(err.to_string()),
        )
    })? && let Some("file") = field.name()
    {
        // only certain amount of files per upload
        if entries.len() >= CONFIG.max_file_count {
            warn!(
                "tried to upload more than {} files in a single request",
                CONFIG.max_file_count
            );
            for entry in entries {
                _ = fs::remove_file(CONFIG.root_dir.join(entry.key)).await;
            }
            return Err((
                StatusCode::TOO_MANY_REQUESTS,
                BumAhhError::TooManyFiles(CONFIG.max_file_count),
            ));
        }

        // check total storage
        if db.size().await + total_entries_size >= CONFIG.max_on_disk_storage {
            error!("storage bucket limit reached");
            for entry in entries {
                _ = fs::remove_file(CONFIG.root_dir.join(entry.key)).await;
            }
            return Err((StatusCode::TOO_MANY_REQUESTS, BumAhhError::OutOfStorage));
        }

        // clean filename
        let mut filename = field
            .file_name()
            .map_or(random(5).collect::<String>(), |x| {
                format!("{}-{}", random(5).collect::<String>(), clean_filename(x))
            });

        // limited file name length
        if filename.len() > CONFIG.max_filename_length {
            filename = filename[0..CONFIG.max_filename_length].to_string();
        }

        // created file
        let filepath = path::Path::new(&CONFIG.root_dir).join(&filename);
        let mut file = fs::File::create(&filepath)
            .await
            .inspect_err(|e| error!("failed to create file: {e}"))
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, BumAhhError::IO(e)))?;

        // stream in chunks
        let mut file_size: usize = 0;
        while let Some(chunk) = field
            .chunk()
            .await
            .inspect_err(|e| error!("failed to get next chunk: {e}"))
            .map_err(|err| {
                (
                    StatusCode::BAD_REQUEST,
                    BumAhhError::InvalidRequest(err.to_string()),
                )
            })?
        {
            file_size = file_size.checked_add(chunk.len()).ok_or_else(|| {
                (
                    StatusCode::BAD_REQUEST,
                    BumAhhError::FileTooBig(CONFIG.max_file_size),
                )
            })?;

            file.write_all(chunk.as_ref())
                .await
                .inspect_err(|e| error!("failed to write to file : {e}"))
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, BumAhhError::IO(e)))?;

            if file_size >= CONFIG.max_file_size {
                warn!(
                    "tried uploading a rather huge file >{}KB",
                    CONFIG.max_file_size / 1024
                );
                for entry in entries {
                    _ = fs::remove_file(CONFIG.root_dir.join(entry.key)).await;
                }
                _ = fs::remove_file(filepath).await;
                return Err((
                    StatusCode::BAD_REQUEST,
                    BumAhhError::FileTooBig(CONFIG.max_file_size),
                ));
            }
        }

        // nothing burger for a file?
        if 0 == file_size {
            _ = fs::remove_file(filepath).await;
            continue;
        }

        // push to entries
        entries.push(DBEntry::new(
            filename,
            file_size as u64,
            random(5).collect::<String>(),
        ));
        total_entries_size += file_size as u64;
    }

    // in which form the client wants response
    let accepts_html = headers
        .get("accept")
        .and_then(|s| s.to_str().ok())
        .is_some_and(|f| f.contains("html"));

    // add to db, return.
    let response = Html(make_url_list(&entries, accepts_html));
    db.insert_mul(entries.into_iter()).await;
    Ok(response)
}

pub async fn serve_file(
    State(db): State<DataBase>,
    Path(filename): Path<String>,
    request: axum::extract::Request,
) -> Result<impl IntoResponse, (StatusCode, BumAhhError)> {
    match db.get_key(&filename).await {
        Some(entry) => {
            let path = CONFIG.root_dir.join(&entry.key);
            let service = tower_http::services::ServeFile::new(path);
            let response = service
                .oneshot(request)
                .await
                .inspect_err(|e| error!("failed to serve file: {e}"))
                .map_err(|_| (StatusCode::NOT_FOUND, BumAhhError::FileNotFound))?;
            Ok(response)
        }
        None => {
            warn!("file={} does not exist", filename);
            Err((StatusCode::NOT_FOUND, BumAhhError::FileNotFound))
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct DeleteKey {
    del_key: String,
}

pub async fn delete_file(
    State(db): State<DataBase>,
    Path(filename): Path<String>,
    Query(query): Query<DeleteKey>,
) -> Result<impl IntoResponse, (StatusCode, BumAhhError)> {
    let file_entry = db
        .get_key(filename)
        .await
        .ok_or((StatusCode::NOT_FOUND, BumAhhError::FileNotFound))?;
    if file_entry.delete_key == query.del_key {
        db.delete_key(&file_entry.key).await;
        return Ok(Html("ok\n"));
    }

    Err((StatusCode::NOT_FOUND, BumAhhError::FileNotFound))
}
