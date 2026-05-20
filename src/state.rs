use std::hash::Hash;
use std::sync::Arc;
use std::time::Duration;
use std::{collections::HashMap, time::SystemTime};
use std::{fmt::Debug, path::PathBuf};
use tokio::{fs, sync::RwLock};

use lazy_static::lazy_static;
use tracing::{debug, info};

use crate::utils::{env_or, hash_one};

#[derive(Debug)]
pub struct Config {
    pub root_dir: PathBuf,
    pub internal_host: String,
    pub internal_port: u16,
    pub external_protocol: String,
    pub external_host: String,
    pub gc_run_interval: Duration,
    pub max_file_count: usize,
    pub max_filename_length: usize,
    pub max_on_disk_storage: u64,
    pub max_file_size: usize,
    pub max_retention_hrs: f32,
    pub min_retention_hrs: f32,
    pub version: &'static str,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            root_dir: std::path::PathBuf::from(env_or("BUMAHH_ROOT_DIR", "files".to_string())),
            internal_host: env_or("BUMAHH_INTERNAL_HOST", "0.0.0.0".to_string()),
            internal_port: env_or("BUMAHH_INTERNAL_PORT", 3000),
            external_protocol: env_or("BUMAHH_EXTERNAL_PROTOCOL", "http".to_string()),
            external_host: env_or("BUMAHH_EXTERNAL_HOST", "0.0.0.0:3000".to_string()),
            gc_run_interval: Duration::from_mins(env_or("BUMAHH_GC_INTERVAL_MIN", 30)),
            max_file_count: env_or("BUMAHH_MAX_FILE_COUNT", 5),
            max_filename_length: env_or("BUMAHH_MAX_FILENAME_LENGTH", 240),
            max_on_disk_storage: env_or("BUMAHH_MAX_ON_DISK_STORAGE", 15 * 1024 * 1024 * 1024),
            max_file_size: env_or("BUMAHH_MAX_FILE_SIZE", 200 * 1024 * 1024),
            max_retention_hrs: env_or("BUMAHH_MAX_RETENTION_HRS", 7.0 * 24.0),
            min_retention_hrs: env_or("BUMAHH_MIN_RETENTION_HRS", 1.0),
            version: env!("GIT_HASH"),
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
    pub size: u64,
    pub created_at: SystemTime,
    pub delete_key: String,
}

impl DBEntry {
    pub fn new(key: String, size: u64, delete_key: String) -> Self {
        Self {
            key,
            size,
            created_at: SystemTime::now(),
            delete_key,
        }
    }
}

#[derive(Default, Debug, Clone)]
struct DataBaseInner {
    entries: HashMap<u64, Arc<DBEntry>>,
    size: u64,
}

#[derive(Default, Debug, Clone)]
pub struct DataBase {
    inner: Arc<RwLock<DataBaseInner>>,
}

#[allow(unused)]
impl DataBase {
    pub async fn len(&self) -> usize {
        self.inner.read().await.entries.len()
    }

    pub async fn size(&self) -> u64 {
        self.inner.read().await.size
    }

    pub async fn get(&self, hash: u64) -> Option<Arc<DBEntry>> {
        let r = self.inner.read().await;
        r.entries.get(&hash).cloned()
    }

    pub async fn get_key<S: AsRef<str> + Hash + Debug>(&self, key: S) -> Option<Arc<DBEntry>> {
        debug!("Get entry for key={key:?}");
        let hash = hash_one(&key);
        self.get(hash).await
    }

    pub async fn insert(&self, entry: DBEntry) {
        info!("inserting entry={entry:?}");
        let mut w = self.inner.write().await;
        w.size += entry.size;
        w.entries.insert(hash_one(&entry.key), Arc::new(entry));
    }

    pub async fn insert_mul<E: Iterator<Item = DBEntry> + Debug>(&self, entries: E) {
        let mut w = self.inner.write().await;
        let mut count = 0;
        for entry in entries {
            count += 1;
            w.size += entry.size;
            w.entries.insert(hash_one(&entry.key), Arc::new(entry));
        }
        debug!("inserted {count} entries");
    }

    pub async fn entries(&self) -> Vec<(u64, Arc<DBEntry>)> {
        self.inner
            .read()
            .await
            .entries
            .iter()
            .map(|(k, v)| (*k, Arc::clone(v)))
            .collect::<Vec<_>>()
    }

    pub async fn delete_key<S: AsRef<str> + Hash + Debug>(&self, key: &S) -> Option<Arc<DBEntry>> {
        self.delete(hash_one(key)).await
    }

    pub async fn delete(&self, key: u64) -> Option<Arc<DBEntry>> {
        info!("deleting entry with key={key}");
        let mut w = self.inner.write().await;
        let entry = w.entries.remove(&key);
        if let Some(e) = &entry {
            w.size -= e.size;
            fs::remove_file(CONFIG.root_dir.join(&e.key)).await;
        }
        entry
    }

    pub async fn delete_mul<E: Iterator<Item = u64> + Debug>(
        &self,
        keys: E,
    ) -> Vec<Option<Arc<DBEntry>>> {
        info!("deleting entries with key={keys:?}");
        let mut w = self.inner.write().await;
        let mut deleted = vec![];
        for key in keys {
            let entry = w.entries.remove(&key);
            if let Some(e) = &entry {
                w.size -= e.size;
                fs::remove_file(CONFIG.root_dir.join(&e.key)).await;
            }
            deleted.push(entry)
        }
        deleted
    }
}
