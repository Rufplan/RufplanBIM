//! Site data from the web (ADR-023): parcel boundaries from Regrid (the owner's token) and
//! ground elevations from USGS 3DEP (free, public). Parsing is separate from fetching so
//! it's testable offline.

use serde_json::Value;

use crate::{Http, Request, SyncError, SyncResult};

/// A parcel from the parcel service: its outer boundary as (lat, lon), and its record.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Parcel {
    pub ring: Vec<(f64, f64)>,
    pub apn: String,
    pub owner: String,
    pub address: String,
    pub acres: f64,
}

/// Percent-encodes a query or form value.
pub fn enc(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

const USGS: &str =
    "https://elevation.nationalmap.gov/arcgis/rest/services/3DEPElevation/ImageServer/getSamples";

/// Elevations of a batch of points (m, None where there is no data), and the finest
/// source resolution among them (m).
pub type Samples = (Vec<Option<f64>>, f64);

/// Points per USGS request: small enough that its gateway doesn't time out when busy.
const BATCH: usize = 500;
/// Requests in flight at once.
const WORKERS: usize = 3;
/// Tries per batch before giving up.
const ATTEMPTS: u32 = 4;

/// Ground elevations (m, NAVD88) at each (lat, lon), None where 3DEP has no data, and the
/// finest source resolution seen (m).
pub fn elevations(http: &dyn Http, pts: &[(f64, f64)]) -> SyncResult<Samples> {
    elevations_with(http, pts, &|_, _| {}, std::time::Duration::from_secs(3))
}

/// One batch, retried when USGS is busy: a network error, a 5xx or 429, or a 200 carrying
/// an error. Other 4xx answers are final.
fn batch(
    http: &dyn Http,
    chunk: &[(f64, f64)],
    backoff: std::time::Duration,
) -> SyncResult<Samples> {
    let coords: Vec<String> = chunk
        .iter()
        .map(|(la, lo)| format!("[{lo:.8},{la:.8}]"))
        .collect();
    let geometry = format!(
        "{{\"points\":[{}],\"spatialReference\":{{\"wkid\":4326}}}}",
        coords.join(",")
    );
    let body = format!(
        "geometry={}&geometryType=esriGeometryMultipoint&returnFirstValueOnly=true&interpolation=RSP_BilinearInterpolation&f=json",
        enc(&geometry)
    );
    let mut last = String::new();
    for attempt in 0..ATTEMPTS {
        if attempt > 0 {
            std::thread::sleep(backoff * 2u32.pow(attempt - 1));
        }
        let resp = http.send(Request {
            method: "POST",
            url: USGS.into(),
            headers: vec![(
                "Content-Type".into(),
                "application/x-www-form-urlencoded".into(),
            )],
            body: body.clone().into_bytes(),
        });
        match resp {
            Err(e) => last = e,
            Ok(r) if r.status >= 500 || r.status == 429 => last = format!("returned {}", r.status),
            Ok(r) if r.status >= 400 => {
                return Err(SyncError::Api(format!(
                    "USGS elevation service returned {}",
                    r.status
                )))
            }
            Ok(r) => match parse_samples(&r.body, chunk.len()) {
                Ok(v) => return Ok(v),
                Err(e) => last = e.to_string(),
            },
        }
    }
    Err(SyncError::Network(format!(
        "USGS's elevation service isn't responding ({last}). It is often busy for a few \
         minutes; try Get Topography again shortly. Nothing was changed."
    )))
}

/// [`elevations`], reporting (batches done, batches) as it goes. Batches run a few at a
/// time; each waits `backoff`, then twice that, and so on between tries.
pub fn elevations_with(
    http: &dyn Http,
    pts: &[(f64, f64)],
    progress: &(dyn Fn(usize, usize) + Sync),
    backoff: std::time::Duration,
) -> SyncResult<Samples> {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::Mutex;
    let chunks: Vec<&[(f64, f64)]> = pts.chunks(BATCH).collect();
    let total = chunks.len();
    let next = AtomicUsize::new(0);
    let done = AtomicUsize::new(0);
    let failed = AtomicBool::new(false);
    let results: Mutex<Vec<Option<Samples>>> = Mutex::new(vec![None; total]);
    let error: Mutex<Option<SyncError>> = Mutex::new(None);
    progress(0, total);
    std::thread::scope(|s| {
        for _ in 0..WORKERS.min(total.max(1)) {
            s.spawn(|| loop {
                if failed.load(Ordering::Relaxed) {
                    return;
                }
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some(chunk) = chunks.get(i) else { return };
                match batch(http, chunk, backoff) {
                    Ok(v) => {
                        if let Ok(mut r) = results.lock() {
                            r[i] = Some(v);
                        }
                        progress(done.fetch_add(1, Ordering::Relaxed) + 1, total);
                    }
                    Err(e) => {
                        failed.store(true, Ordering::Relaxed);
                        if let Ok(mut slot) = error.lock() {
                            slot.get_or_insert(e);
                        }
                        return;
                    }
                }
            });
        }
    });
    if let Some(e) = error.into_inner().ok().flatten() {
        return Err(e);
    }
    let mut out = Vec::with_capacity(pts.len());
    let mut finest = f64::INFINITY;
    for r in results
        .into_inner()
        .unwrap_or_default()
        .into_iter()
        .flatten()
    {
        finest = finest.min(r.1);
        out.extend(r.0);
    }
    Ok((out, if finest.is_finite() { finest } else { 10.0 }))
}

/// The USGS getSamples response: values by locationId (m), and the finest resolution.
pub fn parse_samples(body: &[u8], n: usize) -> SyncResult<Samples> {
    let v: Value = serde_json::from_slice(body)
        .map_err(|e| SyncError::Decode(format!("USGS elevations: {e}")))?;
    if let Some(msg) = v.pointer("/error/message").and_then(Value::as_str) {
        return Err(SyncError::Api(format!("USGS elevation service: {msg}")));
    }
    let samples = v
        .get("samples")
        .and_then(Value::as_array)
        .ok_or_else(|| SyncError::Decode("USGS elevations: no samples".into()))?;
    let mut out = vec![None; n];
    let mut finest = f64::INFINITY;
    for s in samples {
        let Some(i) = s.get("locationId").and_then(Value::as_u64) else {
            continue;
        };
        let value = match s.get("value") {
            Some(Value::String(t)) => t.parse::<f64>().ok(),
            Some(Value::Number(x)) => x.as_f64(),
            _ => None,
        };
        // 3DEP reports missing data as NoData or a large negative number.
        let value = value.filter(|z| *z > -1000.0 && *z < 10_000.0);
        if let (Some(slot), Some(z)) = (out.get_mut(i as usize), value) {
            *slot = Some(z);
        }
        if let Some(r) = s.get("resolution").and_then(Value::as_f64) {
            finest = finest.min(r);
        }
    }
    Ok((out, finest))
}

/// The parcel containing (lat, lon), from Regrid.
pub fn regrid_parcel(http: &dyn Http, token: &str, lat: f64, lon: f64) -> SyncResult<Parcel> {
    if token.trim().is_empty() {
        return Err(SyncError::Api(
            "add your Regrid token (Site > API Keys) to look up parcels".into(),
        ));
    }
    let url = format!(
        "https://app.regrid.com/api/v2/parcels/point?lat={lat:.8}&lon={lon:.8}&token={}&return_geometry=true&limit=1",
        enc(token.trim())
    );
    let resp = http
        .send(Request {
            method: "GET",
            url,
            headers: vec![("Accept".into(), "application/json".into())],
            body: vec![],
        })
        .map_err(|e| SyncError::Network(format!("Regrid: {e}")))?;
    match resp.status {
        401 | 403 => {
            return Err(SyncError::Api(
                "Regrid refused the token; check it in Site > API Keys".into(),
            ))
        }
        s if s >= 400 => return Err(SyncError::Api(format!("Regrid returned {s}"))),
        _ => {}
    }
    parse_regrid(&resp.body)
}

fn text(v: Option<&Value>) -> String {
    match v {
        Some(Value::String(s)) => s.trim().to_owned(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}

/// Regrid's response: the first parcel feature's outer ring and its fields.
pub fn parse_regrid(body: &[u8]) -> SyncResult<Parcel> {
    let v: Value =
        serde_json::from_slice(body).map_err(|e| SyncError::Decode(format!("Regrid: {e}")))?;
    let features = v
        .pointer("/parcels/features")
        .or_else(|| v.get("features"))
        .and_then(Value::as_array)
        .ok_or_else(|| SyncError::Decode("Regrid: no parcels in the response".into()))?;
    let f = features
        .first()
        .ok_or_else(|| SyncError::Api(
            "Regrid has no parcel at that spot. Click inside a lot, not on a street. A Regrid trial token only covers Regrid's sample counties; parcels elsewhere need a paid plan."
                .into(),
        ))?;
    let geom = f
        .get("geometry")
        .ok_or_else(|| SyncError::Decode("Regrid: the parcel has no geometry".into()))?;
    let polys: Vec<&Value> = match geom.get("type").and_then(Value::as_str) {
        Some("Polygon") => geom.get("coordinates").into_iter().collect(),
        Some("MultiPolygon") => geom
            .get("coordinates")
            .and_then(Value::as_array)
            .map(|a| a.iter().collect())
            .unwrap_or_default(),
        _ => vec![],
    };
    // The largest outer ring (by planar area in degrees, enough to compare).
    let ring = polys
        .iter()
        .filter_map(|p| p.get(0).and_then(Value::as_array))
        .map(|r| {
            r.iter()
                .filter_map(|c| Some((c.get(1)?.as_f64()?, c.get(0)?.as_f64()?)))
                .collect::<Vec<(f64, f64)>>()
        })
        .max_by(|a, b| ring_area(a).total_cmp(&ring_area(b)))
        .filter(|r| r.len() >= 3)
        .ok_or_else(|| SyncError::Decode("Regrid: the parcel's boundary is empty".into()))?;
    let props = f.get("properties");
    let fields = props.and_then(|p| p.get("fields")).or(props);
    let field = |k: &str| text(fields.and_then(|f| f.get(k)));
    let address = {
        let a = field("address");
        if a.is_empty() {
            text(props.and_then(|p| p.get("headline")))
        } else {
            let city = field("scity");
            let st = field("state2");
            [a, city, st]
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(", ")
        }
    };
    let acres = fields
        .and_then(|f| f.get("ll_gisacre").or_else(|| f.get("gisacre")))
        .and_then(|a| {
            a.as_f64()
                .or_else(|| a.as_str().and_then(|s| s.parse().ok()))
        })
        .unwrap_or(0.0);
    Ok(Parcel {
        ring,
        apn: field("parcelnumb"),
        owner: field("owner"),
        address,
        acres,
    })
}

/// The Static Maps request for a satellite image centered at (lat, lon) (ADR-026): `width`
/// x `height` map pixels at `zoom`, fetched at scale 2, without labels.
pub fn static_map_url(key: &str, lat: f64, lon: f64, zoom: u32, width: u32, height: u32) -> String {
    format!(
        "https://maps.googleapis.com/maps/api/staticmap?center={lat:.7},{lon:.7}&zoom={zoom}&size={width}x{height}&scale=2&maptype=satellite&format=jpg&key={}",
        enc(key.trim())
    )
}

/// A satellite image (JPEG or PNG bytes) from Google's Maps Static API. Shown, never stored:
/// Google's terms don't allow keeping its imagery in project files.
pub fn static_map(
    http: &dyn Http,
    key: &str,
    lat: f64,
    lon: f64,
    zoom: u32,
    width: u32,
    height: u32,
) -> SyncResult<Vec<u8>> {
    if key.trim().is_empty() {
        return Err(SyncError::Api(
            "add your Google Maps key (Site > API Keys) for the satellite overlay".into(),
        ));
    }
    let resp = http
        .send(Request {
            method: "GET",
            url: static_map_url(key, lat, lon, zoom, width, height),
            headers: vec![],
            body: vec![],
        })
        .map_err(|e| SyncError::Network(format!("Google Maps: {e}")))?;
    let image = resp.body.starts_with(&[0xFF, 0xD8]) || resp.body.starts_with(b"\x89PNG");
    match resp.status {
        200 if image => Ok(resp.body),
        401 | 403 => Err(SyncError::Api(
            "Google refused the satellite image: enable the Maps Static API for your key in Google Cloud (APIs & Services > Library), and allow it in the key's API restrictions".into(),
        )),
        s => Err(SyncError::Api(format!(
            "Google Maps returned {s}: {}",
            String::from_utf8_lossy(&resp.body).chars().take(160).collect::<String>()
        ))),
    }
}

fn ring_area(r: &[(f64, f64)]) -> f64 {
    let n = r.len();
    (0..n)
        .map(|i| {
            let (a, b) = (r[i], r[(i + 1) % n]);
            a.1 * b.0 - b.1 * a.0
        })
        .sum::<f64>()
        .abs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Response;
    use std::sync::Mutex;

    struct Fake(Mutex<Vec<Request>>, Vec<u8>);
    impl Http for Fake {
        fn send(&self, req: Request) -> Result<Response, String> {
            let n = self.0.lock().map(|mut v| {
                v.push(req);
                v.len()
            });
            let _ = n;
            Ok(Response {
                status: 200,
                body: self.1.clone(),
            })
        }
    }

    #[test]
    fn satellite_image_request_and_errors() {
        let url = static_map_url(" k&ey ", 37.7773, -122.462, 20, 390, 400);
        assert_eq!(
            url,
            "https://maps.googleapis.com/maps/api/staticmap?center=37.7773000,-122.4620000&zoom=20&size=390x400&scale=2&maptype=satellite&format=jpg&key=k%26ey"
        );
        let jpeg = Fake(Mutex::new(vec![]), vec![0xFF, 0xD8, 0xFF, 0xE0, 1, 2]);
        let img = static_map(&jpeg, "key", 37.0, -122.0, 19, 100, 100).unwrap();
        assert_eq!(img.len(), 6);
        let sent = &jpeg.0.lock().unwrap()[0];
        assert!(sent.url.contains("maptype=satellite") && sent.url.contains("zoom=19"));
        // A 200 with an error page instead of an image is an error, not a picture.
        let text = Fake(
            Mutex::new(vec![]),
            b"The Google Maps Platform server rejected".to_vec(),
        );
        assert!(static_map(&text, "key", 37.0, -122.0, 19, 100, 100).is_err());
        assert!(static_map(&jpeg, " ", 37.0, -122.0, 19, 100, 100).is_err());
    }

    /// Answers with the scripted statuses in turn (then 200s), with a one-sample body.
    struct Flaky(Mutex<Vec<u16>>, Mutex<usize>);
    impl Http for Flaky {
        fn send(&self, req: Request) -> Result<Response, String> {
            *self.1.lock().unwrap() += 1;
            let status = {
                let mut s = self.0.lock().unwrap();
                if s.is_empty() {
                    200
                } else {
                    s.remove(0)
                }
            };
            let n = String::from_utf8_lossy(&req.body)
                .matches("%5B-")
                .count()
                .max(1);
            let samples: Vec<String> = (0..n)
                .map(|i| format!(r#"{{"locationId":{i},"value":"7","resolution":1}}"#))
                .collect();
            Ok(Response {
                status,
                body: format!(r#"{{"samples":[{}]}}"#, samples.join(",")).into_bytes(),
            })
        }
    }

    #[test]
    fn busy_usgs_is_retried_then_explained() {
        let pts: Vec<(f64, f64)> = (0..1200)
            .map(|i| (37.0, -122.0 - f64::from(i) * 1e-5))
            .collect();
        // Two 502s, then answers: all three batches arrive, with progress.
        let http = Flaky(Mutex::new(vec![502, 502]), Mutex::new(0));
        let seen = Mutex::new(vec![]);
        let (v, res) = elevations_with(
            &http,
            &pts,
            &|d, n| seen.lock().unwrap().push((d, n)),
            std::time::Duration::ZERO,
        )
        .unwrap();
        assert_eq!(v.len(), 1200);
        assert!(v.iter().all(|z| *z == Some(7.0)));
        assert_eq!(res, 1.0);
        assert_eq!(*http.1.lock().unwrap(), 5, "3 batches + 2 retries");
        let seen = seen.into_inner().unwrap();
        assert_eq!(seen.first(), Some(&(0, 3)));
        assert_eq!(seen.last(), Some(&(3, 3)));
        // Down for good: a clear message after four tries.
        let down = Flaky(Mutex::new(vec![502; 50]), Mutex::new(0));
        let e = elevations_with(&down, &pts[..10], &|_, _| {}, std::time::Duration::ZERO)
            .unwrap_err()
            .to_string();
        assert!(e.contains("busy") && e.contains("502"), "{e}");
        assert_eq!(*down.1.lock().unwrap(), 4);
        // A 400 is not retried.
        let bad = Flaky(Mutex::new(vec![400]), Mutex::new(0));
        assert!(elevations_with(&bad, &pts[..10], &|_, _| {}, std::time::Duration::ZERO).is_err());
        assert_eq!(*bad.1.lock().unwrap(), 1);
    }

    #[test]
    fn usgs_samples_by_location_with_gaps() {
        let body = br#"{"samples":[
            {"locationId":0,"value":"16.5","resolution":1},
            {"locationId":2,"value":"NoData","resolution":10},
            {"locationId":1,"value":"15.25","resolution":10}]}"#;
        let (v, res) = parse_samples(body, 3).unwrap();
        assert_eq!(v, [Some(16.5), Some(15.25), None]);
        assert_eq!(res, 1.0);
        assert!(parse_samples(br#"{"error":{"message":"bad"}}"#, 1).is_err());
    }

    #[test]
    fn elevations_batch_by_five_hundred() {
        let fake = Fake(
            Mutex::new(vec![]),
            br#"{"samples":[{"locationId":0,"value":"3","resolution":1}]}"#.to_vec(),
        );
        let pts = vec![(37.0, -122.0); 2500];
        let (v, _) = elevations(&fake, &pts).unwrap();
        let reqs = fake.0.lock().unwrap();
        assert_eq!(reqs.len(), 5);
        assert_eq!(
            (v[0], v[500], v[2000], v[1]),
            (Some(3.0), Some(3.0), Some(3.0), None)
        );
        let body = String::from_utf8(reqs[0].body.clone()).unwrap();
        assert!(body.starts_with("geometry=%7B%22points%22"));
    }

    #[test]
    fn regrid_parcels_parse_polygon_fields() {
        let body = br#"{"parcels":{"type":"FeatureCollection","features":[{"type":"Feature",
            "geometry":{"type":"MultiPolygon","coordinates":[
              [[[-122.4194,37.7749],[-122.4190,37.7749],[-122.4190,37.7752],[-122.4194,37.7752],[-122.4194,37.7749]]],
              [[[-122.5,37.7],[-122.49999,37.7],[-122.49999,37.70001],[-122.5,37.7]]]]},
            "properties":{"headline":"1 Main St","fields":{"parcelnumb":"3702-001","owner":"SMITH JANE",
              "address":"1 MAIN ST","scity":"SAN FRANCISCO","state2":"CA","ll_gisacre":0.29}}}]}}"#;
        let p = parse_regrid(body).unwrap();
        assert_eq!(p.ring.len(), 5);
        assert_eq!(p.ring[0], (37.7749, -122.4194));
        assert_eq!(
            (p.apn.as_str(), p.owner.as_str()),
            ("3702-001", "SMITH JANE")
        );
        assert_eq!(p.address, "1 MAIN ST, SAN FRANCISCO, CA");
        assert!((p.acres - 0.29).abs() < 1e-9);
        assert!(parse_regrid(br#"{"parcels":{"features":[]}}"#).is_err());
        let fake = Fake(Mutex::new(vec![]), body.to_vec());
        assert!(regrid_parcel(&fake, "", 1.0, 1.0).is_err(), "needs a token");
        let _ = regrid_parcel(&fake, "tok en", 37.7, -122.4).unwrap();
        assert!(fake.0.lock().unwrap()[0].url.contains("token=tok%20en"));
    }

    /// Live check against USGS: \`cargo test -p studio-sync live_usgs -- --ignored\`.
    #[test]
    #[ignore]
    fn live_usgs() {
        let (v, res) = elevations(&crate::UreqHttp::default(), &[(37.7749, -122.4194)]).unwrap();
        assert!(v[0].is_some_and(|z| (0.0..100.0).contains(&z)) && res <= 10.0);
    }
}
