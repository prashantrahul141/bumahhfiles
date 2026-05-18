use lazy_static::lazy_static;

#[derive(Debug)]
pub struct Config {
    pub root_dir: PathBuf,
    pub host: String,
    pub protocol: String,
    pub max_file_size: usize,
    pub max_filename_length: usize,
    pub _max_on_disk_storage: usize,
    pub _max_retention_days: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            root_dir: std::path::PathBuf::new().join("files"),
            max_file_size: 10000,
            max_filename_length: 240,
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
