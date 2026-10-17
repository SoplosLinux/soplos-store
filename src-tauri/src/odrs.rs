// Ratings from the Open Desktop Ratings Service (odrs.gnome.org), the
// same service GNOME Software and Discover use. One bulk download covers
// every app and is cached for a day.

use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Duration;

use crate::http;

const URL: &str = "https://odrs.gnome.org/1.0/reviews/api/ratings";

#[derive(Clone, Copy)]
pub struct Rating {
    pub average: f32,
    pub count: u32,
    /// Reviews with 1, 2, 3, 4 and 5 stars.
    pub stars: [u32; 5],
}

fn ratings() -> &'static HashMap<String, Rating> {
    static RATINGS: OnceLock<HashMap<String, Rating>> = OnceLock::new();
    RATINGS.get_or_init(|| {
        let mut map: HashMap<String, Rating> = HashMap::new();
        let Some(json) = http::cached_json(URL, "odrs", Duration::from_secs(24 * 3600)) else {
            return map;
        };
        let Some(entries) = json.as_object() else {
            return map;
        };
        for (id, entry) in entries {
            // star0 counts reviews without a rating, so it is left out.
            let mut stars = [0u32; 5];
            for (n, slot) in stars.iter_mut().enumerate() {
                *slot = entry.get(format!("star{}", n + 1)).and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            }
            let count: u32 = stars.iter().sum();
            if count == 0 {
                continue;
            }
            let weighted: u32 = stars.iter().enumerate().map(|(n, c)| (n as u32 + 1) * c).sum();
            let average = (weighted as f32 / count as f32 * 10.0).round() / 10.0;
            // Keys are lowercased: ODRS and Flathub do not always agree on
            // case (org.mozilla.Firefox vs org.mozilla.firefox).
            let key = normalize(id).to_lowercase();
            if map.get(&key).is_none_or(|existing| existing.count < count) {
                map.insert(key, Rating { average, count, stars });
            }
        }
        map
    })
}

pub fn normalize(id: &str) -> String {
    id.trim_end_matches(".desktop").to_string()
}

pub fn rating(id: &str) -> Option<Rating> {
    ratings().get(&normalize(id).to_lowercase()).copied()
}

pub fn lookup(id: &str) -> (Option<f32>, u32) {
    match rating(id) {
        Some(found) => (Some(found.average), found.count),
        None => (None, 0),
    }
}
