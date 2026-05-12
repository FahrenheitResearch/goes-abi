# goes19

Standalone all-Rust GOES ABI renderer, native PNG generator, XYZ tile generator, and Python package.

This package is for people who want the GOES satellite rendering pieces without installing `rustwx`. It downloads public NOAA GOES ABI Level 2 NetCDF files, reads them with pure Rust dependencies, applies the ABI fixed-grid projection/scaling metadata, and writes PNGs plus JSON manifests.

## Features

- GOES-16, GOES-17, GOES-18, and GOES-19 ABI products.
- Native fixed-grid PNG renders for ABI bands and RGB products.
- Native crop/sequence rendering for workflow-friendly regional loops.
- XYZ Web Mercator tile generation from local ABI channel files.
- Python bindings through `maturin` and a `goes19` Python module.
- No `rustwx` checkout, vendored dependency tree, C NetCDF, C HDF5, or Python geospatial stack required.

## Install

From a checkout:

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
goes19 capabilities
```

Render the latest GOES-19 CONUS Band 13 native PNG:

```powershell
goes19 render `
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
goes19 render `
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
goes19 native-sequence `
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
goes19 web-tiles `
  --channel1 cache\path\to\C01.nc `
  --channel2 cache\path\to\C02.nc `
  --channel3 cache\path\to\C03.nc `
  --out-dir tiles `
  --min-zoom 2 `
  --max-zoom 6
```

## Python Example

```python
import goes19

report = goes19.render_satellite(
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

## Outputs

Every render writes a JSON report next to the PNG/tile output. Reports include scan time, source NOAA S3 keys/URLs, local cache paths, render timing, product metadata, geographic bounds, and generated artifact paths.

## Development Checks

```powershell
cargo fmt --check
cargo test
cargo test --features python
cargo run --bin goes19 -- capabilities
```
