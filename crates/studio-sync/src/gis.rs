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

/// Ground elevations (m, NAVD88) at each (lat, lon), None where 3DEP has no data, and the
/// finest source resolution seen (m). Asks 1,000 points at a time.
pub fn elevations(http: &dyn Http, pts: &[(f64, f64)]) -> SyncResult<(Vec<Option<f64>>, f64)> {
    let mut out = vec![None; pts.len()];
    let mut finest = f64::INFINITY;
    for (chunk_index, chunk) in pts.chunks(1000).enumerate() {
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
        let resp = http
            .send(Request {
                method: "POST",
                url: USGS.into(),
                headers: vec![(
                    "Content-Type".into(),
                    "application/x-www-form-urlencoded".into(),
                )],
                body: body.into_bytes(),
            })
            .map_err(|e| SyncError::Network(format!("USGS elevation service: {e}")))?;
        if resp.status >= 400 {
            return Err(SyncError::Api(format!(
                "USGS elevation service returned {}",
                resp.status
            )));
        }
        let (vals, res) = parse_samples(&resp.body, chunk.len())?;
        finest = finest.min(res);
        let base = chunk_index * 1000;
        for (k, v) in vals.into_iter().enumerate() {
            out[base + k] = v;
        }
    }
    Ok((out, if finest.is_finite() { finest } else { 10.0 }))
}

/// The USGS getSamples response: values by locationId (m), and the finest resolution.
pub fn parse_samples(body: &[u8], n: usize) -> SyncResult<(Vec<Option<f64>>, f64)> {
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
        .ok_or_else(|| SyncError::Api("no parcel found at that spot".into()))?;
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
    fn elevations_batch_by_a_thousand() {
        let fake = Fake(
            Mutex::new(vec![]),
            br#"{"samples":[{"locationId":0,"value":"3","resolution":1}]}"#.to_vec(),
        );
        let pts = vec![(37.0, -122.0); 2500];
        let (v, _) = elevations(&fake, &pts).unwrap();
        let reqs = fake.0.lock().unwrap();
        assert_eq!(reqs.len(), 3);
        assert_eq!(
            (v[0], v[1000], v[2000], v[1]),
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
