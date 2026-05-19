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
    #[error("I/O Error")]
    IO(#[from] io::Error),
    #[error("Internal Error")]
    Internal(String),
    #[error("Invalid Request: {0}")]
    InvalidRequest(String),
    #[error("File too big, max file size in bytes: {0} bytes")]
    FileTooBig(usize),
    #[error("Too many files, allowed a maximum of: {0}")]
    TooManyFiles(usize),
    #[error("File not found")]
    FileNotFound,
    #[error("Storage bucket has reached its limit")]
    OutOfStorage,
    #[error("unknown data store error")]
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

pub fn random(n: usize) -> rand::seq::IndexedSamples<'static, [char], char> {
    safe_chars.sample(&mut rand::rng(), n)
}

fn make_url_from_key<K: AsRef<str> + Display>(key: K) -> String {
    format!(
        "{}://{}/{}",
        CONFIG.external_protocol, CONFIG.external_host, key
    )
}

fn make_del_url<K: AsRef<str> + Display>(key: K, del_id: K) -> String {
    format!(
        "{}://{}/d/{}?del_key={}",
        CONFIG.external_protocol, CONFIG.external_host, key, del_id
    )
}

pub fn make_url_list(urls: &[DBEntry], html: bool) -> String {
    if !html {
        format!(
            "url | size | del_key\n{}\n",
            urls.iter()
                .map(|x| {
                    format!(
                        "{url} (~ {size:.2}KB) ({del_key})",
                        url = make_url_from_key(&x.key),
                        size = x.size / 1024,
                        del_key = x.delete_key
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        )
    } else {
        urls.iter()
            .map(|x| {
                format!(
                    "<a target='_blank' href={url}>{url}</a> (~ {size:.2}KB) <a target='_blank' href={del_url}>delete</a>",
                    url = make_url_from_key(&x.key),
                    size = x.size / 1024,
                    del_url = make_del_url(&x.key, &x.delete_key)
                )
            })
            .collect::<Vec<_>>()
            .join("<br>")
    }
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
