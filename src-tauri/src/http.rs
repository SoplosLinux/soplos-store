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
