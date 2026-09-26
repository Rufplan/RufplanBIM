//! Library texture maps (ADR-029): downloaded once from Poly Haven (CC0) and kept in this
//! computer's cache, so they work offline afterwards.

use std::path::{Path, PathBuf};

use crate::{Http, Request, SyncError, SyncResult};

/// Where a map is cached: `<cache>/<set>/<file name of the URL>`.
pub fn cache_path(cache: &Path, set: &str, url: &str) -> PathBuf {
    let file = url.rsplit('/').next().unwrap_or("map.jpg");
    cache.join(set).join(file)
}

/// A texture map's JPEG bytes: from the cache, else downloaded (and then cached).
pub fn texture_map(http: &dyn Http, cache: &Path, set: &str, url: &str) -> SyncResult<Vec<u8>> {
    let path = cache_path(cache, set, url);
    if let Ok(bytes) = std::fs::read(&path) {
        if bytes.starts_with(&[0xFF, 0xD8]) {
            return Ok(bytes);
        }
    }
    let resp = http
        .send(Request {
            method: "GET",
            url: url.into(),
            headers: vec![],
            body: vec![],
        })
        .map_err(|e| {
            SyncError::Network(format!(
                "couldn't download the {set} texture (needs internet the first time): {e}"
            ))
        })?;
    if resp.status != 200 || !resp.body.starts_with(&[0xFF, 0xD8]) {
        return Err(SyncError::Api(format!(
            "the {set} texture didn't download (Poly Haven returned {})",
            resp.status
        )));
    }
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    // Write beside, then rename: a half-written file never looks cached.
    let tmp = path.with_extension("part");
    if std::fs::write(&tmp, &resp.body).is_ok() {
        let _ = std::fs::rename(&tmp, &path);
    }
    Ok(resp.body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Response;
    use std::sync::Mutex;

    struct Once(Mutex<usize>, Vec<u8>, u16);
    impl Http for Once {
        fn send(&self, _: Request) -> Result<Response, String> {
            *self.0.lock().unwrap() += 1;
            Ok(Response {
                status: self.2,
                body: self.1.clone(),
            })
        }
    }

    #[test]
    fn maps_download_once_then_come_from_the_cache() {
        let dir = std::env::temp_dir().join(format!("rufplan-tex-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let url = "https://dl.polyhaven.org/file/ph-assets/Textures/jpg/2k/wood_floor/wood_floor_diff_2k.jpg";
        let http = Once(Mutex::new(0), vec![0xFF, 0xD8, 0xFF, 1, 2, 3], 200);
        let a = texture_map(&http, &dir, "wood_floor", url).unwrap();
        let b = texture_map(&http, &dir, "wood_floor", url).unwrap();
        assert_eq!(a, b);
        assert_eq!(
            *http.0.lock().unwrap(),
            1,
            "the second read is from the cache"
        );
        assert!(dir
            .join("wood_floor")
            .join("wood_floor_diff_2k.jpg")
            .exists());
        // An error page is not cached or used.
        let bad = Once(Mutex::new(0), b"<html>not found</html>".to_vec(), 404);
        assert!(texture_map(&bad, &dir, "plywood", "https://x/plywood_diff_2k.jpg").is_err());
        assert!(!dir.join("plywood").join("plywood_diff_2k.jpg").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
