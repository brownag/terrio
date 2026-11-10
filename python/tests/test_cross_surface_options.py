"""The same option rule gives the same result on every surface (orchestration 4).

Each test names the divergence it guards: ACCUM without observer coordinates, the CLI and Python
agreeing on cumulative pixels, and clip layer selection and return types.
"""

import subprocess
from pathlib import Path

import numpy as np
import pytest

import terrio

REPO = Path(__file__).resolve().parents[2]
CLI = REPO / "target" / "debug" / "terrio"


def ridge_dem(path):
    dem = terrio.Raster.from_array(
        np.array([[0, 0, 10, 0, 0]], dtype="float64"), (0, 10, 0, 10, 0, -10), crs="EPSG:32611"
    )
    terrio.write_raster(dem, str(path))
    return str(path)


def test_accum_without_observer_matches_the_cli_pixels(tmp_path):
    """ACCUM needs no observer coordinates in Python, and its pixels match the CLI's."""
    if not CLI.exists():
        pytest.skip("build the CLI first: cargo build -p terrio-cli --bin terrio")
    dem = ridge_dem(tmp_path / "dem.tif")
    out = tmp_path / "cli_accum.tif"
    subprocess.run(
        [str(CLI), "viewshed", dem, str(out), "--om", "ACCUM", "--os", "2", "--cc", "0"],
        check=True,
        capture_output=True,
    )
    python_accum = terrio.viewshed(dem, om="ACCUM", os=2, cc=0)
    np.testing.assert_array_equal(python_accum.read(1), terrio.open(str(out)).read(1))


def square():
    return {
        "type": "Feature",
        "properties": {},
        "geometry": {"type": "Polygon", "coordinates": [[[0, 0], [2, 0], [2, 2], [0, 2], [0, 0]]]},
    }


def test_clip_missing_layer_name_is_an_error_on_a_path(tmp_path):
    path = str(tmp_path / "two.gpkg")
    terrio.write_vector(
        [
            terrio.Layer.from_geojson(square(), name="roads"),
            terrio.Layer.from_geojson(square(), name="parcels"),
        ],
        path,
    )
    with pytest.raises(terrio.InvalidArgumentError, match="missing"):
        terrio.clip_vector(path, bbox=[0, 0, 1, 1], layer="missing")


def test_clip_returns_a_dataset_for_two_layers_on_every_path(tmp_path):
    roads = terrio.Layer.from_geojson(square(), name="roads")
    parcels = terrio.Layer.from_geojson(square(), name="parcels")
    path = str(tmp_path / "two.gpkg")
    terrio.write_vector([roads, parcels], path)
    assert isinstance(terrio.clip_vector(path, bbox=[0, 0, 1, 1]), terrio.VectorDataset)
    assert isinstance(
        terrio.clip_vector([roads, parcels], bbox=[0, 0, 1, 1], layers=["roads"]),
        terrio.VectorDataset,
    )
