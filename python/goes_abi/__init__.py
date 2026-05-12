from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from ._goes_abi import (
    capabilities_json_py as _capabilities_json,
    render_native_sequence_json_py as _render_native_sequence_json,
    render_satellite_json_py as _render_satellite_json,
    render_web_tiles_json_py as _render_web_tiles_json,
    viirs_fires_json_py as _viirs_fires_json,
)


def capabilities() -> dict[str, Any]:
    return json.loads(_capabilities_json())


def render_satellite(**request: Any) -> dict[str, Any]:
    return json.loads(_render_satellite_json(_json_request(request)))


def render_native_sequence(**request: Any) -> dict[str, Any]:
    return json.loads(_render_native_sequence_json(_json_request(request)))


def render_web_tiles(**request: Any) -> dict[str, Any]:
    return json.loads(_render_web_tiles_json(_json_request(request)))


def viirs_fires(**request: Any) -> dict[str, Any]:
    return json.loads(_viirs_fires_json(_json_request(request)))


def _json_request(request: dict[str, Any]) -> str:
    def default(value: Any) -> str:
        if isinstance(value, Path):
            return str(value)
        raise TypeError(f"Object of type {type(value).__name__} is not JSON serializable")

    return json.dumps(request, default=default)


__all__ = [
    "capabilities",
    "render_satellite",
    "render_native_sequence",
    "render_web_tiles",
    "viirs_fires",
]
