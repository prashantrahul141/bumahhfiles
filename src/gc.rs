use std::time::{Duration, SystemTime};

use crate::{
    state::{CONFIG, DBEntry, DataBase},
    utils::retention_time,
};
use tokio::time::sleep;
use tracing::debug;

pub fn start_gc(db: DataBase) {
    debug!("starting gc");
    tokio::spawn(async move {
        loop {
            sleep(CONFIG.gc_run_internal).await;
            gc(db.clone()).await;
        }
    });
}

async fn gc(db: DataBase) {
    debug!("running gc");
    let ids = db
        .entries()
        .await
        .iter()
        .filter_map(|(key, file)| if file_expired(file) { Some(*key) } else { None })
        .collect::<Vec<_>>();
    db.delete_mul(ids.into_iter()).await;
}

fn file_expired(file: &DBEntry) -> bool {
    let start_time = file.created_at;
    let retention_time = retention_time(file.size) as u64;
    match start_time.checked_add(Duration::from_hours(retention_time)) {
        Some(expiration) => SystemTime::now() >= expiration,
        None => true,
    }
}
