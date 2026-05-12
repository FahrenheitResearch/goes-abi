from __future__ import annotations

import argparse
import os
import sys
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

from . import capabilities, render_native_sequence, render_satellite, render_web_tiles, viirs_fires

try:
    from mcp.server.fastmcp import FastMCP
except ModuleNotFoundError:
    FastMCP = None  # type: ignore[assignment]


Bounds = Tuple[float, float, float, float]


def build_server() -> Any:
    if FastMCP is None:
        raise RuntimeError(
            "The MCP server requires the optional MCP dependencies. "
            'Install them with: python -m pip install "goes-abi[mcp]"'
        )

    mcp = FastMCP(
        "goes-abi",
        instructions=(
            "All-Rust GOES ABI rendering tools. These tools download public NOAA "
            "GOES ABI files, cache them locally, and write PNGs plus JSON reports."
        ),
    )

    @mcp.tool()
    def goes_abi_capabilities() -> Dict[str, Any]:
        """Return supported satellites, sectors, products, and output modes."""

        return capabilities()

    @mcp.tool()
    def render_satellite_frame(
        satellite: str = "goes19",
        sector: str = "conus",
        products: Optional[List[str]] = None,
        bounds: Optional[List[float]] = None,
        out_dir: Optional[str] = None,
        cache_dir: Optional[str] = None,
        width: int = 1400,
        height: int = 1100,
        scan_lookback_hours: int = 6,
        discovery_retries: int = 1,
        retry_sleep_ms: int = 1000,
        allow_high_resolution_full_disk: bool = False,
    ) -> Dict[str, Any]:
        """Render the latest GOES ABI frame to native PNG artifacts."""

        request = _satellite_request(
            satellite=satellite,
            sector=sector,
            products=products or ["goes_abi_band_13"],
            bounds=_coerce_bounds(bounds, _default_bounds_for_sector(sector)),
            out_dir=out_dir or str(_default_output_dir("goes-abi-renders")),
            cache_dir=cache_dir or str(_default_cache_dir()),
            width=width,
            height=height,
            scan_lookback_hours=scan_lookback_hours,
            discovery_retries=discovery_retries,
            retry_sleep_ms=retry_sleep_ms,
            allow_high_resolution_full_disk=allow_high_resolution_full_disk,
        )
        return render_satellite(**request)

    @mcp.tool()
    def render_native_sequence_frames(
        satellite: str = "goes19",
        sector: str = "conus",
        product: str = "geocolor",
        bounds: Optional[List[float]] = None,
        out_dir: Optional[str] = None,
        cache_dir: Optional[str] = None,
        latest_count: int = 1,
        scan_lookback_hours: int = 6,
        downsample: float = 1.0,
        max_width: Optional[int] = None,
        max_height: Optional[int] = None,
        discovery_retries: int = 1,
        retry_sleep_ms: int = 1000,
    ) -> Dict[str, Any]:
        """Render one or more regional native fixed-grid PNG frames."""

        return render_native_sequence(
            satellite=satellite,
            abi_product=_abi_product_for_sector(sector),
            abi_sector=sector,
            product=product,
            domain_slug="mcp_native_sequence",
            domain_label="MCP Native Sequence",
            bounds=_coerce_bounds(bounds, _default_bounds_for_sector(sector)),
            out_dir=out_dir or str(_default_output_dir("goes-abi-sequences")),
            cache_dir=cache_dir or str(_default_cache_dir()),
            latest_count=latest_count,
            scan_lookback_hours=scan_lookback_hours,
            use_cache=True,
            downsample=downsample,
            max_width=max_width,
            max_height=max_height,
            discovery_retries=discovery_retries,
            retry_sleep_ms=retry_sleep_ms,
            png_compression="fast",
        )

    @mcp.tool()
    def render_xyz_web_tiles(
        channel1: str,
        channel2: str,
        channel3: str,
        channel13: Optional[str] = None,
        out_dir: Optional[str] = None,
        name: str = "goes_geocolor_webmercator",
        bounds: Optional[List[float]] = None,
        min_zoom: int = 2,
        max_zoom: int = 5,
        tile_size: int = 256,
        opacity: float = 0.82,
        layer: str = "geocolor",
        opaque_clouds: bool = False,
        base_url: Optional[str] = None,
    ) -> Dict[str, Any]:
        """Generate XYZ Web Mercator tiles from local ABI C01/C02/C03 files."""

        return render_web_tiles(
            channel1=channel1,
            channel2=channel2,
            channel3=channel3,
            channel13=channel13,
            out_dir=out_dir or str(_default_output_dir("goes-abi-tiles")),
            name=name,
            bounds=_coerce_bounds(bounds, (-165.0, -5.0, -70.0, 70.0)),
            min_zoom=min_zoom,
            max_zoom=max_zoom,
            tile_size=tile_size,
            opacity=opacity,
            layer=layer,
            opaque_clouds=opaque_clouds,
            base_url=base_url,
            png_compression="fast",
        )

    @mcp.tool()
    def viirs_active_fires(
        source: str = "VIIRS_NOAA20_NRT",
        map_key: Optional[str] = None,
        csv_path: Optional[str] = None,
        bounds: Optional[List[float]] = None,
        world: bool = False,
        day_range: int = 1,
        date: Optional[str] = None,
        out_dir: Optional[str] = None,
        min_confidence: Optional[str] = None,
        min_frp: Optional[float] = None,
        limit: Optional[int] = None,
    ) -> Dict[str, Any]:
        """Fetch or parse VIIRS active-fire detections through NASA FIRMS CSV."""

        return viirs_fires(
            source=source,
            map_key=map_key,
            csv_path=csv_path,
            bounds=_coerce_bounds(bounds, _default_bounds_for_sector("conus")),
            world=world,
            day_range=day_range,
            date=date,
            out_dir=out_dir or str(_default_output_dir("goes-abi-viirs-fires")),
            write_geojson=True,
            min_confidence=min_confidence,
            min_frp=min_frp,
            limit=limit,
        )

    @mcp.tool()
    def take_a_break_wallpaper(
        satellite: str = "goes19",
        product: str = "goes_airmass_rgb",
        out_dir: Optional[str] = None,
        cache_dir: Optional[str] = None,
        width: int = 5120,
        height: int = 1440,
        scan_lookback_hours: int = 24,
        discovery_retries: int = 1,
        retry_sleep_ms: int = 1000,
        allow_high_resolution_full_disk: bool = False,
    ) -> Dict[str, Any]:
        """Render a calm full-disk GOES wallpaper for a 5120x1440 display."""

        request = _satellite_request(
            satellite=satellite,
            sector="full_disk",
            products=[product],
            bounds=(-180.0, 180.0, -90.0, 90.0),
            out_dir=out_dir or str(_default_output_dir("goes-abi-break")),
            cache_dir=cache_dir or str(_default_cache_dir()),
            width=width,
            height=height,
            scan_lookback_hours=scan_lookback_hours,
            discovery_retries=discovery_retries,
            retry_sleep_ms=retry_sleep_ms,
            allow_high_resolution_full_disk=allow_high_resolution_full_disk,
            domain_slug="earth_break_wallpaper",
            domain_label="Earth Break Wallpaper",
        )
        report = render_satellite(**request)
        artifacts = report.get("artifacts") or []
        if artifacts:
            report["wallpaper_path"] = artifacts[0].get("png_path")
        report["wallpaper_size"] = [width, height]
        report["note"] = (
            "Default product uses 2 km full-disk ABI channels. For visible GeoColor, "
            'use product="goes_geocolor" with allow_high_resolution_full_disk=true.'
        )
        return report

    return mcp


def main() -> None:
    parser = argparse.ArgumentParser(prog="goes-abi-mcp")
    parser.add_argument(
        "--transport",
        choices=["stdio", "sse", "streamable-http"],
        default="stdio",
    )
    args = parser.parse_args()
    try:
        build_server().run(args.transport)
    except RuntimeError as exc:
        print(str(exc), file=sys.stderr)
        raise SystemExit(2) from exc


def _satellite_request(
    *,
    satellite: str,
    sector: str,
    products: List[str],
    bounds: Bounds,
    out_dir: str,
    cache_dir: str,
    width: int,
    height: int,
    scan_lookback_hours: int,
    discovery_retries: int,
    retry_sleep_ms: int,
    allow_high_resolution_full_disk: bool,
    domain_slug: str = "mcp_render",
    domain_label: str = "MCP Render",
) -> Dict[str, Any]:
    return {
        "satellite": satellite,
        "abi_product": _abi_product_for_sector(sector),
        "abi_sector": sector,
        "domain_slug": domain_slug,
        "domain_label": domain_label,
        "bounds": bounds,
        "out_dir": out_dir,
        "cache_dir": cache_dir,
        "products": products,
        "width": int(width),
        "height": int(height),
        "scan_lookback_hours": int(scan_lookback_hours),
        "discovery_retries": int(discovery_retries),
        "retry_sleep_ms": int(retry_sleep_ms),
        "use_cache": True,
        "download_glm": False,
        "auto_bounds": True,
        "allow_high_resolution_full_disk": bool(allow_high_resolution_full_disk),
        "png_compression": "fast",
    }


def _abi_product_for_sector(sector: str) -> str:
    normalized = sector.strip().lower().replace("-", "_").replace(" ", "_")
    if normalized in {"full", "full_disk", "fulldisk", "full_disc", "fulldisc", "fd", "f"}:
        return "ABI-L2-CMIPF"
    if normalized in {"meso1", "mesoscale1", "mesoscale_1", "m1"}:
        return "ABI-L2-CMIPM1"
    if normalized in {"meso2", "mesoscale2", "mesoscale_2", "m2"}:
        return "ABI-L2-CMIPM2"
    if normalized in {"meso", "mesoscale"}:
        return "ABI-L2-CMIPM"
    return "ABI-L2-CMIPC"


def _default_bounds_for_sector(sector: str) -> Bounds:
    normalized = sector.strip().lower().replace("-", "_").replace(" ", "_")
    if normalized in {"full", "full_disk", "fulldisk", "full_disc", "fulldisc", "fd", "f"}:
        return (-180.0, 180.0, -90.0, 90.0)
    return (-127.0, -111.0, 30.0, 44.5)


def _coerce_bounds(raw: Optional[List[float]], fallback: Bounds) -> Bounds:
    if raw is None:
        return fallback
    if len(raw) != 4:
        raise ValueError("bounds must contain exactly four numbers: west,east,south,north")
    west, east, south, north = [float(value) for value in raw]
    return (west, east, south, north)


def _default_output_dir(name: str) -> Path:
    downloads = Path.home() / "Downloads"
    return downloads.joinpath(name) if downloads.exists() else Path.cwd().joinpath(name)


def _default_cache_dir() -> Path:
    raw = os.environ.get("GOES_ABI_CACHE_DIR")
    if raw:
        return Path(raw)
    return Path.home() / ".cache" / "goes-abi"


if __name__ == "__main__":
    main()
