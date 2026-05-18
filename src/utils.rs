use std::{
    hash::{DefaultHasher, Hash, Hasher},
    io,
};

use axum::response::IntoResponse;
use lazy_static::lazy_static;
use rand::seq::IndexedRandom;
use thiserror::Error;

#[derive(Error, Debug)]
#[allow(unused)]
pub enum BumAhhError {
    #[error("I/O Error")]
    IO(#[from] io::Error),
    #[error("Internal Error")]
    Internal(String),
    #[error("Invalid Request: {0}")]
    InvalidRequest(String),
    #[error("File too big, max file size in bytes: {0}")]
    FileTooBig(usize),
    #[error("File not found")]
    FileNotFound,
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

pub fn make_url_list(urls: &[String], html: bool) -> String {
    if !html {
        urls.join("\n")
    } else {
        urls.iter()
            .map(|x| format!("<a href={x}>{x}</a>"))
            .collect::<Vec<_>>()
            .join("<br>")
    }
}

pub fn hash_one<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}
