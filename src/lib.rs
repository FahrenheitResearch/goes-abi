use std::path::PathBuf;

pub mod png;
pub mod satellite;
pub mod web_tiles;

pub use png::{Color, PngCompressionMode, PngWriteOptions};
pub use satellite::{
    GoesNativeSequenceReport, GoesNativeSequenceRequest, GoesSatelliteBatchReport,
    GoesSatelliteBatchRequest, run_goes_native_sequence, run_goes_satellite_batch,
};
pub use web_tiles::{GoesWebTilesReport, GoesWebTilesRequest, run_goes_web_tiles};

pub fn render_satellite_json(request_json: &str) -> Result<String, Box<dyn std::error::Error>> {
    let request: GoesSatelliteBatchRequest = serde_json::from_str(request_json)?;
    let report = run_goes_satellite_batch(&request)?;
    Ok(serde_json::to_string_pretty(&report)?)
}

pub fn render_native_sequence_json(
    request_json: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let request: GoesNativeSequenceRequest = serde_json::from_str(request_json)?;
    let report = run_goes_native_sequence(&request)?;
    Ok(serde_json::to_string_pretty(&report)?)
}

pub fn render_web_tiles_json(request_json: &str) -> Result<String, Box<dyn std::error::Error>> {
    let request: GoesWebTilesRequest = serde_json::from_str(request_json)?;
    let report = run_goes_web_tiles(&request)?;
    Ok(serde_json::to_string_pretty(&report)?)
}

pub fn capabilities_json() -> String {
    serde_json::json!({
        "package": "goes19",
        "schema": "goes19.capabilities.v1",
        "satellites": ["goes16", "goes17", "goes18", "goes19"],
        "sectors": ["conus", "full_disk", "meso1", "meso2"],
        "batch_products": [
            "goes_abi_band_01", "goes_abi_band_02", "goes_abi_band_03", "goes_abi_band_04",
            "goes_abi_band_05", "goes_abi_band_06", "goes_abi_band_07", "goes_abi_band_08",
            "goes_abi_band_09", "goes_abi_band_10", "goes_abi_band_11", "goes_abi_band_12",
            "goes_abi_band_13", "goes_abi_band_14", "goes_abi_band_15", "goes_abi_band_16",
            "goes_geocolor", "goes_airmass_rgb", "goes_sandwich_rgb",
            "goes_day_night_cloud_micro_combo_rgb", "goes_fire_temperature_rgb", "goes_dust_rgb"
        ],
        "native_sequence_products": [
            "geocolor", "airmass", "dust", "fire_temperature", "sandwich",
            "day_night_cloud_micro_combo", "band_13", "C13"
        ],
        "outputs": ["native_png", "native_sequence_png", "xyz_webmercator_tiles", "json_manifest"],
        "default_cache_dir_env": "GOES19_CACHE_DIR"
    })
    .to_string()
}

pub fn default_cache_dir() -> PathBuf {
    std::env::var_os("GOES19_CACHE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("goes19_cache"))
}

#[cfg(feature = "python")]
mod python {
    use super::*;
    use pyo3::exceptions::PyRuntimeError;
    use pyo3::prelude::*;

    fn py_err(err: Box<dyn std::error::Error>) -> PyErr {
        PyRuntimeError::new_err(err.to_string())
    }

    #[pyfunction]
    fn capabilities_json_py() -> PyResult<String> {
        Ok(capabilities_json())
    }

    #[pyfunction]
    fn render_satellite_json_py(request_json: &str) -> PyResult<String> {
        render_satellite_json(request_json).map_err(py_err)
    }

    #[pyfunction]
    fn render_native_sequence_json_py(request_json: &str) -> PyResult<String> {
        render_native_sequence_json(request_json).map_err(py_err)
    }

    #[pyfunction]
    fn render_web_tiles_json_py(request_json: &str) -> PyResult<String> {
        render_web_tiles_json(request_json).map_err(py_err)
    }

    #[pymodule]
    fn _goes19(module: &Bound<'_, PyModule>) -> PyResult<()> {
        module.add_function(wrap_pyfunction!(capabilities_json_py, module)?)?;
        module.add_function(wrap_pyfunction!(render_satellite_json_py, module)?)?;
        module.add_function(wrap_pyfunction!(render_native_sequence_json_py, module)?)?;
        module.add_function(wrap_pyfunction!(render_web_tiles_json_py, module)?)?;
        Ok(())
    }
}
