use std::time::SystemTime;

use crate::{
    state::{CONFIG, DBEntry, DataBase},
    utils::retention_time,
};
use tokio::time::sleep;
use tracing::info;

pub fn start_gc(db: DataBase) {
    info!("starting gc");
    tokio::spawn(async move {
        loop {
            sleep(CONFIG.gc_run_interval).await;
            trigger_gc(db.clone()).await;
        }
    });
}

async fn trigger_gc(db: DataBase) {
    info!("running gc");
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
    match start_time.checked_add(retention_time(file.size)) {
        Some(expiration) => SystemTime::now() >= expiration,
        None => true,
    }
}
