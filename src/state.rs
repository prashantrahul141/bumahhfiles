use std::hash::Hash;
use std::sync::Arc;
use std::time::Duration;
use std::{collections::HashMap, time::SystemTime};
use std::{fmt::Debug, path::PathBuf};
use tokio::{fs, sync::RwLock};

use lazy_static::lazy_static;
use tracing::debug;

use crate::utils::hash_one;

#[derive(Debug)]
pub struct Config {
    pub root_dir: PathBuf,
    pub host: String,
    pub protocol: String,
    pub max_file_size: usize,
    pub gc_run_internal: Duration,
    pub max_file_count: usize,
    pub max_filename_length: usize,
    pub _max_on_disk_storage: usize,
    pub _max_retention_days: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            root_dir: std::path::PathBuf::from("files"),
            max_file_size: 250 * 1000 * 1000,
            max_filename_length: 240,
            max_file_count: 5,
            gc_run_internal: Duration::from_secs(30),
            _max_on_disk_storage: Default::default(),
            _max_retention_days: Default::default(),
            host: "0.0.0.0:3000".into(),
            protocol: "http".into(),
        }
    }
}

lazy_static! {
    pub static ref CONFIG: Config = Config::default();
}

impl std::fmt::Debug for CONFIG {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        (**self).fmt(f)
    }
}

#[derive(Debug)]
pub struct DBEntry {
    pub key: String,
    pub size: usize,
    pub created_at: SystemTime,
    pub delete_key: String,
}

impl DBEntry {
    pub fn new(key: String, size: usize, delete_key: String) -> Self {
        Self {
            key,
            size,
            created_at: SystemTime::now(),
            delete_key,
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct DataBase {
    inner: Arc<RwLock<HashMap<u64, Arc<DBEntry>>>>,
}

#[allow(unused)]
impl DataBase {
    pub async fn get(&self, hash: u64) -> Option<Arc<DBEntry>> {
        debug!("Get entry for hash={hash}");
        let r = self.inner.read().await;
        r.get(&hash).cloned()
    }

    pub async fn get_key<S: AsRef<str> + Hash + Debug>(&self, key: S) -> Option<Arc<DBEntry>> {
        debug!("Get entry for key={key:?}");
        let hash = hash_one(&key);
        self.get(hash).await
    }

    pub async fn insert(&self, entry: DBEntry) {
        debug!("Inserting entry={entry:?}");
        let mut w = self.inner.write().await;
        w.insert(hash_one(&entry.key), Arc::new(entry));
    }

    pub async fn insert_mul<E: Iterator<Item = DBEntry> + Debug>(&self, entries: E) {
        let mut w = self.inner.write().await;
        debug!("Inserting entries={entries:?}");
        for entry in entries {
            w.insert(hash_one(&entry.key), Arc::new(entry));
        }
    }

    pub async fn entries(&self) -> Vec<(u64, Arc<DBEntry>)> {
        self.inner
            .read()
            .await
            .iter()
            .map(|(k, v)| (*k, Arc::clone(v)))
            .collect::<Vec<_>>()
    }

    pub async fn delete(&self, key: u64) -> Option<Arc<DBEntry>> {
        debug!("deleting entry with key={key}");
        let mut w = self.inner.write().await;
        let entry = w.remove(&key);
        if let Some(e) = &entry {
            fs::remove_file(CONFIG.root_dir.join(&e.key)).await;
        }
        entry
    }

    pub async fn delete_mul<E: Iterator<Item = u64> + Debug>(
        &self,
        keys: E,
    ) -> Vec<Option<Arc<DBEntry>>> {
        debug!("deleting entries with key={keys:?}");
        let mut w = self.inner.write().await;
        let mut deleted = vec![];
        for key in keys {
            let entry = w.remove(&key);
            if let Some(e) = &entry {
                fs::remove_file(CONFIG.root_dir.join(&e.key)).await;
            }
            deleted.push(entry)
        }
        deleted
    }
}
