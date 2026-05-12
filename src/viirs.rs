use chrono::{DateTime, Datelike, NaiveDate, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::Instant;

use crate::web_tiles::atomic_write_json;

const FIRMS_AREA_BASE_URL: &str = "https://firms.modaps.eosdis.nasa.gov/api/area/csv";
const DEFAULT_SOURCE: &str = "VIIRS_NOAA20_NRT";
const DEFAULT_DAY_RANGE: u8 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViirsFireRequest {
    #[serde(default = "default_source")]
    pub source: String,
    #[serde(default)]
    pub map_key: Option<String>,
    #[serde(default)]
    pub csv_path: Option<PathBuf>,
    #[serde(default)]
    pub csv_text: Option<String>,
    #[serde(default = "default_bounds")]
    pub bounds: (f64, f64, f64, f64),
    #[serde(default)]
    pub world: bool,
    #[serde(default = "default_day_range")]
    pub day_range: u8,
    #[serde(default)]
    pub date: Option<NaiveDate>,
    #[serde(default)]
    pub out_dir: Option<PathBuf>,
    #[serde(default = "default_true")]
    pub write_geojson: bool,
    #[serde(default)]
    pub min_confidence: Option<String>,
    #[serde(default)]
    pub min_frp: Option<f64>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViirsFireReport {
    pub ok: bool,
    pub schema: String,
    pub generated_at_utc: DateTime<Utc>,
    pub source: String,
    pub source_kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
    pub bounds: (f64, f64, f64, f64),
    pub world: bool,
    pub day_range: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<NaiveDate>,
    pub filters: ViirsFireFilters,
    pub detection_count: usize,
    pub detections: Vec<ViirsFireDetection>,
    pub summary: ViirsFireSummary,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report_path: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geojson_path: Option<PathBuf>,
    pub timing_ms: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViirsFireFilters {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_confidence: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_frp: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ViirsFireSummary {
    pub by_confidence: BTreeMap<String, usize>,
    pub by_satellite: BTreeMap<String, usize>,
    pub by_day_night: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViirsFireDetection {
    pub latitude: f64,
    pub longitude: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brightness_ti4_k: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brightness_ti5_k: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scan_km: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track_km: Option<f64>,
    pub acquisition_date: NaiveDate,
    pub acquisition_time_utc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acquisition_datetime_utc: Option<DateTime<Utc>>,
    pub satellite: String,
    pub instrument: String,
    pub confidence: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frp_mw: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub day_night: Option<String>,
    pub source: String,
    pub raw: BTreeMap<String, String>,
}

struct SourceCsv {
    source_kind: String,
    source_url: Option<String>,
    text: String,
}

pub fn run_viirs_fire_detection(
    request: &ViirsFireRequest,
) -> Result<ViirsFireReport, Box<dyn Error>> {
    let start = Instant::now();
    validate_request(request)?;
    let source = normalize_source(&request.source)?;
    let source_csv = read_source_csv(request, &source)?;
    let mut detections = parse_firms_viirs_csv(&source_csv.text, &source)?;
    detections.retain(|detection| detection_matches_request(detection, request));
    if let Some(limit) = request.limit {
        detections.truncate(limit);
    }
    let summary = summarize(&detections);

    let (report_path, geojson_path) = if let Some(out_dir) = &request.out_dir {
        let run_dir = out_dir
            .join("viirs_fire")
            .join(source.to_ascii_lowercase())
            .join(Utc::now().format("%Y%m%dT%H%M%SZ").to_string());
        fs::create_dir_all(&run_dir)?;
        let geojson_path = if request.write_geojson {
            let path = run_dir.join("viirs_fire_detections.geojson");
            atomic_write_json(&path, &viirs_fire_geojson(&detections))?;
            Some(path)
        } else {
            None
        };
        (Some(run_dir.join("viirs_fire_report.json")), geojson_path)
    } else {
        (None, None)
    };

    let report = ViirsFireReport {
        ok: true,
        schema: "goes_abi.viirs_fire_report.v1".to_string(),
        generated_at_utc: Utc::now(),
        source,
        source_kind: source_csv.source_kind,
        source_url: source_csv.source_url,
        bounds: request.bounds,
        world: request.world,
        day_range: request.day_range,
        date: request.date,
        filters: ViirsFireFilters {
            min_confidence: request.min_confidence.clone(),
            min_frp: request.min_frp,
            limit: request.limit,
        },
        detection_count: detections.len(),
        detections,
        summary,
        report_path: report_path.clone(),
        geojson_path,
        timing_ms: start.elapsed().as_millis(),
    };
    if let Some(path) = &report_path {
        atomic_write_json(path, &report)?;
    }
    Ok(report)
}

pub fn parse_firms_viirs_csv(
    csv_text: &str,
    source: &str,
) -> Result<Vec<ViirsFireDetection>, Box<dyn Error>> {
    let mut reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(csv_text.as_bytes());
    let headers = reader.headers()?.clone();
    let header_index = headers
        .iter()
        .enumerate()
        .map(|(idx, name)| (name.trim().to_ascii_lowercase(), idx))
        .collect::<HashMap<_, _>>();
    for required in ["latitude", "longitude", "acq_date", "acq_time"] {
        if !header_index.contains_key(required) {
            return Err(boxed_error(format!(
                "FIRMS CSV is missing required column '{required}'"
            )));
        }
    }

    let mut detections = Vec::new();
    for record in reader.records() {
        let record = record?;
        let raw = headers
            .iter()
            .zip(record.iter())
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect::<BTreeMap<_, _>>();
        let latitude = required_f64(&record, &header_index, "latitude")?;
        let longitude = required_f64(&record, &header_index, "longitude")?;
        let acquisition_date = required_date(&record, &header_index, "acq_date")?;
        let acquisition_time_utc =
            normalize_acquisition_time(field(&record, &header_index, "acq_time").unwrap_or(""))?;
        let acquisition_datetime_utc =
            acquisition_datetime(acquisition_date, &acquisition_time_utc);

        detections.push(ViirsFireDetection {
            latitude,
            longitude,
            brightness_ti4_k: optional_f64(&record, &header_index, "bright_ti4"),
            brightness_ti5_k: optional_f64(&record, &header_index, "bright_ti5"),
            scan_km: optional_f64(&record, &header_index, "scan"),
            track_km: optional_f64(&record, &header_index, "track"),
            acquisition_date,
            acquisition_time_utc,
            acquisition_datetime_utc,
            satellite: optional_string(&record, &header_index, "satellite")
                .unwrap_or_else(|| "unknown".to_string()),
            instrument: optional_string(&record, &header_index, "instrument")
                .unwrap_or_else(|| "VIIRS".to_string()),
            confidence: optional_string(&record, &header_index, "confidence")
                .unwrap_or_else(|| "unknown".to_string()),
            version: optional_string(&record, &header_index, "version")
                .unwrap_or_else(|| "unknown".to_string()),
            frp_mw: optional_f64(&record, &header_index, "frp"),
            day_night: optional_string(&record, &header_index, "daynight"),
            source: source.to_string(),
            raw,
        });
    }
    Ok(detections)
}

pub fn viirs_fire_geojson(detections: &[ViirsFireDetection]) -> serde_json::Value {
    let features = detections
        .iter()
        .map(|detection| {
            json!({
                "type": "Feature",
                "geometry": {
                    "type": "Point",
                    "coordinates": [detection.longitude, detection.latitude]
                },
                "properties": {
                    "source": detection.source,
                    "satellite": detection.satellite,
                    "instrument": detection.instrument,
                    "confidence": detection.confidence,
                    "version": detection.version,
                    "frp_mw": detection.frp_mw,
                    "brightness_ti4_k": detection.brightness_ti4_k,
                    "brightness_ti5_k": detection.brightness_ti5_k,
                    "scan_km": detection.scan_km,
                    "track_km": detection.track_km,
                    "acquisition_date": detection.acquisition_date.to_string(),
                    "acquisition_time_utc": detection.acquisition_time_utc,
                    "acquisition_datetime_utc": detection.acquisition_datetime_utc,
                    "day_night": detection.day_night
                }
            })
        })
        .collect::<Vec<_>>();
    json!({
        "type": "FeatureCollection",
        "features": features
    })
}

fn read_source_csv(request: &ViirsFireRequest, source: &str) -> Result<SourceCsv, Box<dyn Error>> {
    if let Some(text) = request
        .csv_text
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        return Ok(SourceCsv {
            source_kind: "csv_text".to_string(),
            source_url: None,
            text: text.clone(),
        });
    }
    if let Some(path) = &request.csv_path {
        return Ok(SourceCsv {
            source_kind: "csv_path".to_string(),
            source_url: None,
            text: fs::read_to_string(path)?,
        });
    }
    let map_key = request
        .map_key
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .or_else(|| std::env::var("FIRMS_MAP_KEY").ok())
        .or_else(|| std::env::var("NASA_FIRMS_MAP_KEY").ok())
        .or_else(|| std::env::var("GOES_ABI_FIRMS_MAP_KEY").ok())
        .ok_or_else(|| {
            boxed_error(
                "VIIRS FIRMS fetch requires map_key, FIRMS_MAP_KEY, NASA_FIRMS_MAP_KEY, GOES_ABI_FIRMS_MAP_KEY, csv_path, or csv_text",
            )
        })?;
    let url = firms_area_url(
        FIRMS_AREA_BASE_URL,
        &map_key,
        source,
        request.bounds,
        request.world,
        request.day_range,
        request.date,
    );
    let redacted_url = firms_area_url(
        FIRMS_AREA_BASE_URL,
        "MAP_KEY",
        source,
        request.bounds,
        request.world,
        request.day_range,
        request.date,
    );
    let mut response = build_agent().get(&url).call()?;
    let text = response.body_mut().read_to_string()?;
    Ok(SourceCsv {
        source_kind: "firms_area_csv".to_string(),
        source_url: Some(redacted_url),
        text,
    })
}

fn firms_area_url(
    base_url: &str,
    map_key: &str,
    source: &str,
    bounds: (f64, f64, f64, f64),
    world: bool,
    day_range: u8,
    date: Option<NaiveDate>,
) -> String {
    let area = if world {
        "world".to_string()
    } else {
        let (west, east, south, north) = bounds;
        format!("{west},{south},{east},{north}")
    };
    let mut url = format!(
        "{}/{}/{}/{}/{}",
        base_url.trim_end_matches('/'),
        url_path_encode(map_key),
        source,
        area,
        day_range
    );
    if let Some(date) = date {
        url.push('/');
        url.push_str(&date.to_string());
    }
    url
}

fn detection_matches_request(detection: &ViirsFireDetection, request: &ViirsFireRequest) -> bool {
    if !request.world && !inside_bounds(detection.longitude, detection.latitude, request.bounds) {
        return false;
    }
    if let Some(min_frp) = request.min_frp {
        if detection.frp_mw.unwrap_or(f64::NEG_INFINITY) < min_frp {
            return false;
        }
    }
    if let Some(min_confidence) = &request.min_confidence {
        let Some(min_rank) = confidence_rank(min_confidence) else {
            return false;
        };
        let Some(rank) = confidence_rank(&detection.confidence) else {
            return false;
        };
        if rank < min_rank {
            return false;
        }
    }
    true
}

fn summarize(detections: &[ViirsFireDetection]) -> ViirsFireSummary {
    let mut summary = ViirsFireSummary::default();
    for detection in detections {
        *summary
            .by_confidence
            .entry(detection.confidence.clone())
            .or_insert(0) += 1;
        *summary
            .by_satellite
            .entry(detection.satellite.clone())
            .or_insert(0) += 1;
        let day_night = detection
            .day_night
            .clone()
            .unwrap_or_else(|| "unknown".to_string());
        *summary.by_day_night.entry(day_night).or_insert(0) += 1;
    }
    summary
}

fn validate_request(request: &ViirsFireRequest) -> Result<(), Box<dyn Error>> {
    normalize_source(&request.source)?;
    if request.day_range == 0 || request.day_range > 5 {
        return Err(boxed_error(
            "FIRMS Area API day_range must be between 1 and 5",
        ));
    }
    if !request.world {
        let (west, east, south, north) = request.bounds;
        if !(west.is_finite()
            && east.is_finite()
            && south.is_finite()
            && north.is_finite()
            && west < east
            && south < north
            && (-180.0..=180.0).contains(&west)
            && (-180.0..=180.0).contains(&east)
            && (-90.0..=90.0).contains(&south)
            && (-90.0..=90.0).contains(&north))
        {
            return Err(boxed_error(
                "VIIRS bounds must be finite [west,east,south,north] with west < east and south < north",
            ));
        }
    }
    if let Some(min_confidence) = &request.min_confidence {
        confidence_rank(min_confidence)
            .ok_or_else(|| boxed_error("min_confidence must be one of low, nominal, or high"))?;
    }
    Ok(())
}

fn normalize_source(source: &str) -> Result<String, Box<dyn Error>> {
    let normalized = source
        .trim()
        .to_ascii_uppercase()
        .replace('-', "_")
        .replace(' ', "_");
    let source = match normalized.as_str() {
        "VIIRS_NOAA20_NRT" | "NOAA20_NRT" | "NOAA_20_NRT" | "N20_NRT" => "VIIRS_NOAA20_NRT",
        "VIIRS_NOAA20_SP" | "NOAA20_SP" | "NOAA_20_SP" | "N20_SP" => "VIIRS_NOAA20_SP",
        "VIIRS_NOAA21_NRT" | "NOAA21_NRT" | "NOAA_21_NRT" | "N21_NRT" => "VIIRS_NOAA21_NRT",
        "VIIRS_SNPP_NRT" | "SNPP_NRT" | "S_NPP_NRT" | "SUOMI_NPP_NRT" => "VIIRS_SNPP_NRT",
        "VIIRS_SNPP_SP" | "SNPP_SP" | "S_NPP_SP" | "SUOMI_NPP_SP" => "VIIRS_SNPP_SP",
        _ => {
            return Err(boxed_error(format!(
                "unsupported VIIRS fire source '{source}', expected VIIRS_NOAA20_NRT, VIIRS_NOAA20_SP, VIIRS_NOAA21_NRT, VIIRS_SNPP_NRT, or VIIRS_SNPP_SP"
            )));
        }
    };
    Ok(source.to_string())
}

fn inside_bounds(lon: f64, lat: f64, bounds: (f64, f64, f64, f64)) -> bool {
    let (west, east, south, north) = bounds;
    lon >= west && lon <= east && lat >= south && lat <= north
}

fn field<'a>(
    record: &'a csv::StringRecord,
    header_index: &HashMap<String, usize>,
    name: &str,
) -> Option<&'a str> {
    header_index
        .get(name)
        .and_then(|idx| record.get(*idx))
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn required_f64(
    record: &csv::StringRecord,
    header_index: &HashMap<String, usize>,
    name: &str,
) -> Result<f64, Box<dyn Error>> {
    let raw = field(record, header_index, name)
        .ok_or_else(|| boxed_error(format!("FIRMS CSV row is missing '{name}'")))?;
    raw.parse::<f64>()
        .map_err(|err| boxed_error(format!("invalid {name} value '{raw}': {err}")))
}

fn optional_f64(
    record: &csv::StringRecord,
    header_index: &HashMap<String, usize>,
    name: &str,
) -> Option<f64> {
    field(record, header_index, name).and_then(|value| value.parse::<f64>().ok())
}

fn optional_string(
    record: &csv::StringRecord,
    header_index: &HashMap<String, usize>,
    name: &str,
) -> Option<String> {
    field(record, header_index, name).map(ToString::to_string)
}

fn required_date(
    record: &csv::StringRecord,
    header_index: &HashMap<String, usize>,
    name: &str,
) -> Result<NaiveDate, Box<dyn Error>> {
    let raw = field(record, header_index, name)
        .ok_or_else(|| boxed_error(format!("FIRMS CSV row is missing '{name}'")))?;
    NaiveDate::parse_from_str(raw, "%Y-%m-%d")
        .map_err(|err| boxed_error(format!("invalid {name} value '{raw}': {err}")))
}

fn normalize_acquisition_time(raw: &str) -> Result<String, Box<dyn Error>> {
    let digits = raw.trim();
    if digits.is_empty() || digits.len() > 4 || !digits.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(boxed_error(format!("invalid acq_time value '{raw}'")));
    }
    let padded = format!("{digits:0>4}");
    let hour = padded[..2].parse::<u32>()?;
    let minute = padded[2..].parse::<u32>()?;
    if hour > 23 || minute > 59 {
        return Err(boxed_error(format!("invalid acq_time value '{raw}'")));
    }
    Ok(padded)
}

fn acquisition_datetime(date: NaiveDate, hhmm: &str) -> Option<DateTime<Utc>> {
    let hour = hhmm.get(..2)?.parse::<u32>().ok()?;
    let minute = hhmm.get(2..)?.parse::<u32>().ok()?;
    Utc.with_ymd_and_hms(date.year(), date.month(), date.day(), hour, minute, 0)
        .single()
}

fn confidence_rank(confidence: &str) -> Option<u8> {
    match confidence.trim().to_ascii_lowercase().as_str() {
        "l" | "low" => Some(0),
        "n" | "nominal" => Some(1),
        "h" | "high" => Some(2),
        _ => None,
    }
}

fn build_agent() -> ureq::Agent {
    static CRYPTO_PROVIDER: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    CRYPTO_PROVIDER.get_or_init(|| {
        rustls::crypto::CryptoProvider::install_default(rustls_rustcrypto::provider()).ok();
    });
    let crypto = std::sync::Arc::new(rustls_rustcrypto::provider());
    ureq::Agent::config_builder()
        .tls_config(
            ureq::tls::TlsConfig::builder()
                .provider(ureq::tls::TlsProvider::Rustls)
                .root_certs(ureq::tls::RootCerts::WebPki)
                .unversioned_rustls_crypto_provider(crypto)
                .build(),
        )
        .build()
        .new_agent()
}

fn url_path_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn default_source() -> String {
    DEFAULT_SOURCE.to_string()
}

fn default_bounds() -> (f64, f64, f64, f64) {
    (-127.0, -111.0, 30.0, 44.5)
}

fn default_day_range() -> u8 {
    DEFAULT_DAY_RANGE
}

fn default_true() -> bool {
    true
}

fn boxed_error(message: impl Into<String>) -> Box<dyn Error> {
    Box::new(io::Error::new(io::ErrorKind::InvalidInput, message.into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_CSV: &str = "latitude,longitude,bright_ti4,scan,track,acq_date,acq_time,satellite,instrument,confidence,version,bright_ti5,frp,daynight\n\
34.25,-118.42,356.1,0.41,0.45,2026-05-12,0530,N20,VIIRS,high,2.0NRT,291.2,42.5,N\n\
35.10,-120.00,320.0,0.50,0.52,2026-05-12,45,N21,VIIRS,low,2.0NRT,284.1,2.0,D\n";

    #[test]
    fn parses_firms_viirs_csv() {
        let detections = parse_firms_viirs_csv(SAMPLE_CSV, "VIIRS_NOAA20_NRT").unwrap();
        assert_eq!(detections.len(), 2);
        assert_eq!(detections[0].acquisition_time_utc, "0530");
        assert_eq!(detections[1].acquisition_time_utc, "0045");
        assert_eq!(detections[0].frp_mw, Some(42.5));
    }

    #[test]
    fn filters_by_confidence_frp_and_bounds() {
        let request = ViirsFireRequest {
            csv_text: Some(SAMPLE_CSV.to_string()),
            min_confidence: Some("nominal".to_string()),
            min_frp: Some(10.0),
            bounds: (-119.0, -117.0, 33.0, 35.0),
            ..ViirsFireRequest {
                source: default_source(),
                map_key: None,
                csv_path: None,
                csv_text: None,
                bounds: default_bounds(),
                world: false,
                day_range: 1,
                date: None,
                out_dir: None,
                write_geojson: true,
                min_confidence: None,
                min_frp: None,
                limit: None,
            }
        };
        let report = run_viirs_fire_detection(&request).unwrap();
        assert_eq!(report.detection_count, 1);
        assert_eq!(report.detections[0].confidence, "high");
    }

    #[test]
    fn builds_firms_area_url_with_bbox_order() {
        let date = NaiveDate::from_ymd_opt(2026, 5, 12).unwrap();
        let url = firms_area_url(
            FIRMS_AREA_BASE_URL,
            "abc 123",
            "VIIRS_NOAA20_NRT",
            (-127.0, -111.0, 30.0, 44.5),
            false,
            2,
            Some(date),
        );
        assert_eq!(
            url,
            "https://firms.modaps.eosdis.nasa.gov/api/area/csv/abc%20123/VIIRS_NOAA20_NRT/-127,30,-111,44.5/2/2026-05-12"
        );
    }

    #[test]
    fn emits_geojson_points() {
        let detections = parse_firms_viirs_csv(SAMPLE_CSV, "VIIRS_NOAA20_NRT").unwrap();
        let geojson = viirs_fire_geojson(&detections);
        assert_eq!(geojson["type"], "FeatureCollection");
        assert_eq!(geojson["features"].as_array().unwrap().len(), 2);
        assert_eq!(
            geojson["features"][0]["geometry"]["coordinates"][0],
            -118.42
        );
    }
}
