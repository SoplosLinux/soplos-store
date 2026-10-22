// Small HTTP layer over ureq, with a disk cache that falls back to the
// last saved copy when the network request fails.

use std::sync::OnceLock;
use std::time::Duration;

use crate::cache;

const MAX_BODY: u64 = 64 * 1024 * 1024;

fn agent() -> &'static ureq::Agent {
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(15)))
            .build()
            .into()
    })
}

pub fn get(url: &str, headers: &[(&str, &str)]) -> Option<Vec<u8>> {
    let mut request = agent().get(url);
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let mut response = request.call().ok()?;
    response.body_mut().with_config().limit(MAX_BODY).read_to_vec().ok()
}

/// Answer to a HEAD request: the Last-Modified date, when the server gives
/// one.
pub struct Head {
    pub last_modified: Option<String>,
}

pub fn head(url: &str, headers: &[(&str, &str)]) -> Option<Head> {
    let mut request = agent().head(url);
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let response = request.call().ok()?;
    let last_modified = response.headers().get("Last-Modified").and_then(|v| v.to_str().ok()).map(str::to_string);
    Some(Head { last_modified })
}

/// Downloads a file to `dest`, a piece at a time: an AppImage can be
/// hundreds of megabytes, so it is neither held in memory nor cut by a
/// size limit. Only connecting and waiting for the answer have a time
/// limit, so a server that does not answer cannot hold the job for good.
/// `progress` is called with the bytes written so far and the total size
/// when the server tells it.
pub fn download(url: &str, dest: &std::path::Path, headers: &[(&str, &str)], mut progress: impl FnMut(u64, Option<u64>)) -> Result<(), String> {
    use std::io::{Read, Write};

    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    let agent = AGENT.get_or_init(|| {
        ureq::Agent::config_builder()
            .timeout_connect(Some(Duration::from_secs(15)))
            .timeout_recv_response(Some(Duration::from_secs(30)))
            .build()
            .into()
    });
    let mut request = agent.get(url);
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let mut response = request.call().map_err(|e| e.to_string())?;
    let total = response.headers().get("Content-Length").and_then(|v| v.to_str().ok()).and_then(|v| v.parse().ok());
    let mut reader = response.body_mut().as_reader();
    let mut file = std::fs::File::create(dest).map_err(|e| e.to_string())?;
    let mut buffer = vec![0u8; 64 * 1024];
    let mut done = 0u64;
    loop {
        let read = reader.read(&mut buffer).map_err(|e| e.to_string())?;
        if read == 0 {
            break;
        }
        file.write_all(&buffer[..read]).map_err(|e| e.to_string())?;
        done += read as u64;
        progress(done, total);
    }
    file.flush().map_err(|e| e.to_string())
}

pub fn post_json(url: &str, body: &serde_json::Value) -> Option<serde_json::Value> {
    let bytes = serde_json::to_vec(body).ok()?;
    let mut response = agent()
        .post(url)
        .header("Content-Type", "application/json")
        .send(&bytes[..])
        .ok()?;
    let data = response.body_mut().with_config().limit(MAX_BODY).read_to_vec().ok()?;
    serde_json::from_slice(&data).ok()
}

/// GET through the disk cache in `~/.cache/soplos-store/<sub>`.
pub fn cached(url: &str, sub: &str, ttl: Duration, headers: &[(&str, &str)]) -> Option<Vec<u8>> {
    let path = cache::dir(sub).join(cache::key(url));
    if let Some(data) = cache::read_fresh(&path, ttl) {
        return Some(data);
    }
    match get(url, headers) {
        Some(data) => {
            let _ = std::fs::write(&path, &data);
            Some(data)
        }
        // Offline: an expired copy is better than nothing.
        None => std::fs::read(&path).ok(),
    }
}

pub fn cached_json(url: &str, sub: &str, ttl: Duration) -> Option<serde_json::Value> {
    let data = cached(url, sub, ttl, &[])?;
    serde_json::from_slice(&data).ok()
}
