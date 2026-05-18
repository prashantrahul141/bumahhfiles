use crate::state::{CONFIG, DBEntry, DataBase};
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

fn file_expired(_file: &DBEntry) -> bool {
    true
}
