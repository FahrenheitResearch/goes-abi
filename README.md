# goes-abi

Standalone all-Rust GOES ABI renderer, native PNG generator, XYZ tile generator, and Python package.

This package is for people who want the GOES satellite rendering pieces without installing `rustwx`. It downloads public NOAA GOES ABI Level 2 NetCDF files, reads them with pure Rust dependencies, applies the ABI fixed-grid projection/scaling metadata, and writes PNGs plus JSON manifests.

## Features

- GOES-16, GOES-17, GOES-18, and GOES-19 ABI products.
- Native fixed-grid PNG renders for ABI bands and RGB products.
- Native crop/sequence rendering for workflow-friendly regional loops.
- XYZ Web Mercator tile generation from local ABI channel files.
- VIIRS active-fire detection ingest through NASA FIRMS CSV/API.
- Python bindings through `maturin` and a `goes_abi` Python module.
- No `rustwx` checkout, vendored dependency tree, C NetCDF, C HDF5, or Python geospatial stack required.

## Install

Install the Rust CLI directly from GitHub:

```powershell
cargo install --git https://github.com/FahrenheitResearch/goes-abi
```

Install the Python package directly from GitHub:

```powershell
python -m pip install "git+https://github.com/FahrenheitResearch/goes-abi"
```

Install from PyPI:

```powershell
python -m pip install goes-abi
```

Install the optional MCP server for Claude Desktop, Codex, and other MCP clients:

```powershell
python -m pip install "goes-abi[mcp]"
```

From a local checkout:

```powershell
cargo install --path .
```

For Python:

```powershell
python -m pip install .
```

For editable Python development:

```powershell
python -m pip install maturin
python -m maturin develop --features python
```

## CLI Examples

Print supported products and outputs:

```powershell
goes-abi capabilities
```

Render the latest GOES-19 CONUS Band 13 native PNG:

```powershell
goes-abi render `
  --satellite goes19 `
  --sector conus `
  --products goes_abi_band_13 `
  --width 1400 `
  --height 1100 `
  --out-dir out `
  --cache-dir cache
```

Render a full-disk native-resolution infrared frame:

```powershell
goes-abi render `
  --satellite goes19 `
  --sector full_disk `
  --products goes_abi_band_13 `
  --width 5424 `
  --height 5424 `
  --out-dir out `
  --cache-dir cache
```

Render a regional native crop sequence:

```powershell
goes-abi native-sequence `
  --satellite goes19 `
  --sector conus `
  --product geocolor `
  --bounds -127,-111,30,44.5 `
  --latest-count 6 `
  --out-dir out `
  --cache-dir cache
```

Generate XYZ tiles from local C01/C02/C03 channel files:

```powershell
goes-abi web-tiles `
  --channel1 cache\path\to\C01.nc `
  --channel2 cache\path\to\C02.nc `
  --channel3 cache\path\to\C03.nc `
  --out-dir tiles `
  --min-zoom 2 `
  --max-zoom 6
```

Fetch recent VIIRS active-fire detections from NASA FIRMS and write JSON plus GeoJSON:

```powershell
$env:FIRMS_MAP_KEY = "your_firms_map_key"
goes-abi viirs-fires `
  --source VIIRS_NOAA20_NRT `
  --west -127 `
  --east -111 `
  --south 30 `
  --north 44.5 `
  --day-range 1 `
  --min-confidence nominal `
  --out-dir out
```

Parse an existing FIRMS CSV without using a map key:

```powershell
goes-abi viirs-fires `
  --csv-path fires.csv `
  --min-frp 10 `
  --out-dir out
```

## Python Example

```python
import goes_abi

report = goes_abi.render_satellite(
    satellite="goes19",
    abi_product="ABI-L2-CMIPC",
    abi_sector="conus",
    domain_slug="goes_native",
    domain_label="GOES Native",
    bounds=(-127.0, -111.0, 30.0, 44.5),
    out_dir="out",
    cache_dir="cache",
    products=["goes_abi_band_13"],
    width=1400,
    height=1100,
    download_glm=False,
    png_compression="fast",
)

print(report["artifacts"][0]["png_path"])
```

Fetch VIIRS active-fire detections:

```python
import goes_abi

report = goes_abi.viirs_fires(
    source="VIIRS_NOAA20_NRT",
    bounds=(-127.0, -111.0, 30.0, 44.5),
    day_range=1,
    min_confidence="nominal",
    out_dir="out",
)

print(report["detection_count"])
print(report["geojson_path"])
```

## MCP Server

`goes-abi` includes an optional stdio MCP server. It exposes tools for capabilities, native PNG rendering, native sequence rendering, XYZ tile generation from local ABI channel files, VIIRS active-fire detection ingest, and a `take_a_break_wallpaper` tool that renders a 5120x1440 full-disk GOES wallpaper.

Use this command in MCP clients:

```powershell
goes-abi-mcp
```

Equivalent module form:

```powershell
python -m goes_abi.mcp_server
```

Example MCP server config:

```json
{
  "mcpServers": {
    "goes-abi": {
      "command": "goes-abi-mcp"
    }
  }
}
```

The wallpaper tool defaults to `goes_airmass_rgb` because it uses 2 km full-disk channels and is practical for agents to run on demand. For visible full-disk GeoColor, call it with `product="goes_geocolor"` and `allow_high_resolution_full_disk=true`; that path downloads and renders high-resolution visible channels and can take substantially more memory and time.

## VIIRS Fires

The VIIRS fire path uses NASA FIRMS active-fire CSV records from NOAA-20, NOAA-21, or Suomi-NPP VIIRS sources. It can fetch from the FIRMS Area API with a map key or parse a local FIRMS CSV. The output report preserves fire point metadata including latitude, longitude, brightness temperatures, scan/track size, acquisition date/time, satellite, instrument, confidence, version, FRP, day/night flag, and the original CSV row. When `out_dir` is provided it also writes GeoJSON for map overlays.

FIRMS Area API reference: <https://firms.modaps.eosdis.nasa.gov/api/area/csv>

Supported FIRMS VIIRS sources:

```text
VIIRS_NOAA20_NRT
VIIRS_NOAA20_SP
VIIRS_NOAA21_NRT
VIIRS_SNPP_NRT
VIIRS_SNPP_SP
```

Set one of these environment variables instead of passing `--map-key`:

```powershell
$env:FIRMS_MAP_KEY = "your_firms_map_key"
$env:NASA_FIRMS_MAP_KEY = "your_firms_map_key"
$env:GOES_ABI_FIRMS_MAP_KEY = "your_firms_map_key"
```

## Outputs

Every render or data ingest writes a JSON report next to the PNG/tile/vector output. GOES reports include scan time, source NOAA S3 keys/URLs, local cache paths, render timing, product metadata, geographic bounds, and generated artifact paths. VIIRS reports include FIRMS source metadata, filters, fire-detection summaries, detections, and optional GeoJSON paths.

## Development Checks

```powershell
cargo fmt --check
cargo test
cargo test --features python
cargo run --bin goes-abi -- capabilities
```
