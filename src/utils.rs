use std::{
    env,
    ffi::OsStr,
    fmt::{Debug, Display},
    hash::{DefaultHasher, Hash, Hasher},
    io,
    str::FromStr,
    time::Duration,
};

use axum::response::IntoResponse;
use lazy_static::lazy_static;
use rand::seq::IndexedRandom;
use thiserror::Error;

use crate::state::{CONFIG, DBEntry};

#[derive(Error, Debug)]
#[allow(unused)]
pub enum BumAhhError {
    #[error("I/O Error\n")]
    IO(#[from] io::Error),
    #[error("Internal Error\n")]
    Internal(String),
    #[error("Invalid Request: {0}\n")]
    InvalidRequest(String),
    #[error("File too big, max file size in bytes: {0} bytes\n")]
    FileTooBig(usize),
    #[error("Too many files, allowed a maximum of: {0}\n")]
    TooManyFiles(usize),
    #[error("File not found\n")]
    FileNotFound,
    #[error("Storage bucket has reached its limit\n")]
    OutOfStorage,
    #[error("unknown data store error\n")]
    Unknown,
}

impl IntoResponse for BumAhhError {
    fn into_response(self) -> axum::response::Response {
        self.to_string().into_response()
    }
}

lazy_static! {
    // i really dont want random() to pick '.' as the first character
    static ref safe_chars: Vec<char> = ('a'..='z')
        .chain('A'..='Z')
        .chain('0'..='9')
        .chain('_'..='_')
        .collect::<Vec<char>>();
    static ref allowed_chars: Vec<char> = safe_chars
        .iter()
        .cloned()
        .chain(std::iter::once('.'))
        .collect();
}

pub fn clean_filename<S: AsRef<str>>(filename: S) -> String {
    filename
        .as_ref()
        .chars()
        .map(|x| if allowed_chars.contains(&x) { x } else { '_' })
        .collect::<String>()
}

fn truncate<S: AsRef<str>>(s: S, n: usize) -> String {
    s.as_ref().chars().take(n).collect()
}

pub fn limit_filename_len<S: AsRef<str>>(filename: S, max_len: usize) -> String {
    let filename = filename.as_ref();

    if filename.chars().count() <= max_len {
        return filename.to_string();
    }

    let (stem, ext) = match filename.rsplit_once('.') {
        Some((stem, ext)) => (stem, Some(ext)),
        None => (filename, None),
    };

    match ext {
        Some(ext) => {
            let ext_len = ext.chars().count() + 1;

            if ext_len >= max_len {
                return format!(".{}", truncate(ext, max_len.saturating_sub(1)));
            }

            let allowed_stem_len = max_len - ext_len;

            format!("{}.{}", truncate(stem, allowed_stem_len), ext)
        }

        None => truncate(filename, max_len),
    }
}

pub fn random(n: usize) -> rand::seq::IndexedSamples<'static, [char], char> {
    safe_chars.sample(&mut rand::rng(), n)
}

pub fn make_url_from_key<K: AsRef<str> + Display>(key: K) -> String {
    format!(
        "{}://{}/{}",
        CONFIG.external_protocol, CONFIG.external_host, key
    )
}

pub fn make_del_url(entry: &DBEntry) -> String {
    format!(
        "{}://{}/d/{}?del_key={}",
        CONFIG.external_protocol,
        CONFIG.external_host,
        entry.key,
        entry.delete_key()
    )
}

pub fn hash_one<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

pub fn retention_time(file_size: u64) -> Duration {
    // the equation blows if you provide file size bigger than max file size.
    if file_size > CONFIG.max_file_size as u64 {
        return Duration::from_mins(0);
    }

    let hrs = CONFIG.min_retention_hrs
        + (CONFIG.max_retention_hrs
            * (1_f32 - (file_size as f32 / (CONFIG.max_file_size) as f32))
                .powf(std::f32::consts::E));
    Duration::from_mins((hrs * 60.0) as u64)
}

pub fn env_or<E, T>(key: E, default: T) -> T
where
    E: AsRef<OsStr>,
    T: FromStr,
    <T as FromStr>::Err: Debug,
{
    match env::var(key) {
        Ok(v) => v.parse::<T>().unwrap(),
        Err(_) => default,
    }
}

pub fn clean_file(file: &DBEntry) {
    _ = std::fs::remove_file(CONFIG.root_dir.join(&file.key));
}

pub fn clean_files(files: &[DBEntry]) {
    for file in files {
        clean_file(file);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_limit_filename_len() {
        let name_expected = vec![
            (".dotfile", ".dotfile"),
            (".dotfileswithareallylongname", ".dotfilesw"),
            ("nodots", "nodots"),
            ("nodotsbutareallylongname", "nodotsbuta"),
            ("normal.mp4", "normal.mp4"),
            ("nor.norm.mp4", "nor.no.mp4"),
            ("...............", ".........."),
            ("n.n.n.n.n.n.n.n.n.n.n.n.n", "n.n.n.n..n"),
            ("n.n.n.n.n.n.n.n.n.n.n.n.", "n.n.n.n.n."),
        ];
        for (name, expected) in name_expected {
            let was = limit_filename_len(name, 10);
            assert_eq!(expected, was);
        }
    }
}
