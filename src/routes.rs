use crate::{
    state::{CONFIG, DBEntry, DataBase},
    template::{HtmlTemplate, IndexTemplate},
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

pub async fn root() -> impl IntoResponse {
    HtmlTemplate(IndexTemplate {
        domain: format!("{}://{}", CONFIG.protocol, CONFIG.host),
    })
}

pub async fn upload_file(
    State(db): State<DataBase>,
    headers: HeaderMap,
    mut form: Multipart,
) -> Result<impl IntoResponse, BumAhhError> {
    let mut entries: Vec<DBEntry> = vec![];
    let mut total_entries_size: u64 = 0;
    while let Some(mut field) = form
        .next_field()
        .await
        .map_err(|err| BumAhhError::InvalidRequest(err.to_string()))?
        && let Some("file") = field.name()
    {
        // only certain amount of files per upload
        if entries.len() >= CONFIG.max_file_count {
            for entry in entries {
                _ = fs::remove_file(CONFIG.root_dir.join(entry.key)).await;
            }
            return Err(BumAhhError::TooManyFiles(CONFIG.max_file_count));
        }

        // check total storage
        if db.size().await + total_entries_size >= CONFIG.max_on_disk_storage {
            for entry in entries {
                _ = fs::remove_file(CONFIG.root_dir.join(entry.key)).await;
            }
            return Err(BumAhhError::OutOfStorage);
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
        let mut file = fs::File::create(&filepath).await?;

        // stream in chunks
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

            if file_size >= CONFIG.max_file_size {
                for entry in entries {
                    _ = fs::remove_file(CONFIG.root_dir.join(entry.key)).await;
                }
                _ = fs::remove_file(filepath).await;
                return Err(BumAhhError::FileTooBig(CONFIG.max_file_size));
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
