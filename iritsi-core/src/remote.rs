use std::io::Read;

use crate::err::CoreErr;

pub const FEED_URL: &str = "https://cms.metrobilbao.eus/es/get/open_data/horarios/es";

#[cfg(feature = "bundled-feed")]
pub const BUNDLED_FEED: &[u8] = include_bytes!("../../google_transit.zip");

const MAX_FEED_SIZE: u64 = 64 * 1024 * 1024;

const PROGRESS_STEP: usize = 64 * 1024;

pub fn fetch(url: &str, mut progress: impl FnMut(u64, Option<u64>)) -> Result<Vec<u8>, CoreErr> {
    let mut res = ureq::get(url).call()?;
    let total = res
        .headers()
        .get("content-length")
        .and_then(|len| len.to_str().ok()?.parse().ok());
    let mut reader = res.body_mut().as_reader().take(MAX_FEED_SIZE);
    let mut feed = Vec::with_capacity(total.unwrap_or(2 * 1024 * 1024).min(MAX_FEED_SIZE) as usize);
    let mut chunk = vec![0; PROGRESS_STEP];
    progress(0, total);
    loop {
        let read = reader.read(&mut chunk)?;
        if read == 0 {
            break;
        }
        feed.extend_from_slice(&chunk[..read]);
        progress(feed.len() as u64, total);
    }
    Ok(feed)
}

pub fn hash(feed: &[u8]) -> u64 {
    feed.iter().fold(0xcbf29ce484222325, |hash, &b| {
        (hash ^ b as u64).wrapping_mul(0x100000001b3)
    })
}
