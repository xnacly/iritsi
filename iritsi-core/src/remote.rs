use std::io::Read;

use crate::err::CoreErr;

pub const FEED_URL: &str = "https://cms.metrobilbao.eus/es/get/open_data/horarios/es";

const MAX_FEED_SIZE: u64 = 64 * 1024 * 1024;

pub fn fetch(url: &str) -> Result<Vec<u8>, CoreErr> {
    let mut res = ureq::get(url).call()?;
    let mut feed = Vec::with_capacity(2 * 1024 * 1024);
    res.body_mut()
        .as_reader()
        .take(MAX_FEED_SIZE)
        .read_to_end(&mut feed)?;
    Ok(feed)
}

pub fn hash(feed: &[u8]) -> u64 {
    feed.iter().fold(0xcbf29ce484222325, |hash, &b| {
        (hash ^ b as u64).wrapping_mul(0x100000001b3)
    })
}
