from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from . import capabilities, render_native_sequence, render_satellite, render_web_tiles, viirs_fires


def main() -> None:
    parser = argparse.ArgumentParser(prog="goes-abi")
    sub = parser.add_subparsers(dest="command", required=True)

    sub.add_parser("capabilities")

    render = sub.add_parser("render")
    _add_common_satellite_args(render)
    render.add_argument("--products", default="goes_abi_band_13")
    render.add_argument("--width", type=int, default=1400)
    render.add_argument("--height", type=int, default=1100)
    render.add_argument("--allow-high-resolution-full-disk", action="store_true")

    seq = sub.add_parser("native-sequence")
    _add_common_satellite_args(seq)
    seq.add_argument("--product", default="geocolor")
    seq.add_argument("--latest-count", type=int, default=1)
    seq.add_argument("--downsample", type=float, default=1.0)
    seq.add_argument("--max-width", type=int)
    seq.add_argument("--max-height", type=int)

    tiles = sub.add_parser("web-tiles")
    tiles.add_argument("--channel1", type=Path, required=True)
    tiles.add_argument("--channel2", type=Path, required=True)
    tiles.add_argument("--channel3", type=Path, required=True)
    tiles.add_argument("--channel13", type=Path)
    tiles.add_argument("--out-dir", type=Path, required=True)
    tiles.add_argument("--name", default="goes_geocolor_webmercator")
    tiles.add_argument("--bounds", default="-165,-5,-70,70")
    tiles.add_argument("--min-zoom", type=int, default=2)
    tiles.add_argument("--max-zoom", type=int, default=5)
    tiles.add_argument("--tile-size", type=int, default=256)
    tiles.add_argument("--opacity", type=float, default=0.82)
    tiles.add_argument("--layer", choices=["geocolor", "clouds"], default="geocolor")
    tiles.add_argument("--opaque-clouds", action="store_true")
    tiles.add_argument("--base-url")

    fires = sub.add_parser("viirs-fires")
    fires.add_argument("--source", default="VIIRS_NOAA20_NRT")
    fires.add_argument("--map-key")
    fires.add_argument("--csv-path", type=Path)
    fires.add_argument("--bounds", default="-127,-111,30,44.5")
    fires.add_argument("--world", action="store_true")
    fires.add_argument("--day-range", type=int, default=1)
    fires.add_argument("--date")
    fires.add_argument("--out-dir", type=Path)
    fires.add_argument("--no-geojson", action="store_true")
    fires.add_argument("--min-confidence")
    fires.add_argument("--min-frp", type=float)
    fires.add_argument("--limit", type=int)

    args = parser.parse_args()
    if args.command == "capabilities":
        result: dict[str, Any] = capabilities()
    elif args.command == "render":
        result = render_satellite(
            satellite=args.satellite,
            abi_product="ABI-L2-CMIPC",
            abi_sector=args.sector,
            domain_slug=args.domain,
            domain_label=args.label,
            bounds=_parse_bounds(args.bounds),
            out_dir=args.out_dir,
            cache_dir=args.cache_dir,
            products=[item.strip() for item in args.products.split(",") if item.strip()],
            width=args.width,
            height=args.height,
            auto_bounds=True,
            download_glm=False,
            allow_high_resolution_full_disk=args.allow_high_resolution_full_disk,
            png_compression="fast",
        )
    elif args.command == "native-sequence":
        result = render_native_sequence(
            satellite=args.satellite,
            abi_product="ABI-L2-CMIPC",
            abi_sector=args.sector,
            product=args.product,
            domain_slug=args.domain,
            domain_label=args.label,
            bounds=_parse_bounds(args.bounds),
            out_dir=args.out_dir,
            cache_dir=args.cache_dir,
            latest_count=args.latest_count,
            downsample=args.downsample,
            max_width=args.max_width,
            max_height=args.max_height,
            png_compression="fast",
        )
    elif args.command == "web-tiles":
        result = render_web_tiles(
            channel1=args.channel1,
            channel2=args.channel2,
            channel3=args.channel3,
            channel13=args.channel13,
            out_dir=args.out_dir,
            name=args.name,
            bounds=_parse_bounds(args.bounds),
            min_zoom=args.min_zoom,
            max_zoom=args.max_zoom,
            tile_size=args.tile_size,
            opacity=args.opacity,
            layer=args.layer,
            opaque_clouds=args.opaque_clouds,
            base_url=args.base_url,
            png_compression="fast",
        )
    else:
        result = viirs_fires(
            source=args.source,
            map_key=args.map_key,
            csv_path=args.csv_path,
            bounds=_parse_bounds(args.bounds),
            world=args.world,
            day_range=args.day_range,
            date=args.date,
            out_dir=args.out_dir,
            write_geojson=not args.no_geojson,
            min_confidence=args.min_confidence,
            min_frp=args.min_frp,
            limit=args.limit,
        )
    print(json.dumps(result, indent=2))


def _add_common_satellite_args(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("--satellite", default="goes19")
    parser.add_argument("--sector", default="conus")
    parser.add_argument("--domain", default="goes_native")
    parser.add_argument("--label", default="GOES Native")
    parser.add_argument("--bounds", default="-127,-111,30,44.5")
    parser.add_argument("--out-dir", type=Path, required=True)
    parser.add_argument("--cache-dir", type=Path, default=Path("goes_abi_cache"))


def _parse_bounds(raw: str) -> tuple[float, float, float, float]:
    parts = [float(part.strip()) for part in raw.split(",")]
    if len(parts) != 4:
        raise SystemExit("--bounds must be west,east,south,north")
    return (parts[0], parts[1], parts[2], parts[3])


if __name__ == "__main__":
    main()
