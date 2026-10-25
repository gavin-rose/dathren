import os

import pytest

import dathren

network = pytest.mark.skipif(os.environ.get("DATHREN_OFFLINE") == "1", reason="needs network")


def test_lists_datasets():
    ids = {d["id"] for d in dathren.datasets()}
    assert {"cdc-vsrr-overdose", "brfss-mental-health", "cdc-places-county"} <= ids


def test_info_includes_caveats():
    meta = dathren.info("cdc-vsrr-overdose")
    assert meta["caveats"]
    assert any(m["id"] == "synthetic_opioids" for m in meta["measures"])


def test_unknown_dataset_and_measure_raise_value_error():
    with pytest.raises(ValueError, match="unknown dataset"):
        dathren.info("nope")
    with pytest.raises(ValueError, match="unknown measure"):
        dathren.fetch("cdc-places-county", measure="nope")


@network
def test_fetch_returns_flat_records():
    rows = dathren.fetch("brfss-mental-health", geo="CA", measure="mental_distress")
    assert rows and all(r["geo"] == "CA" and r["measure"] == "mental_distress" for r in rows)
    assert isinstance(rows[0]["period"], str)
    assert rows[0]["ci_low"] <= rows[0]["value"] <= rows[0]["ci_high"]


@network
def test_fetch_df():
    pandas = pytest.importorskip("pandas")
    df = dathren.fetch_df("nchs-mortality-rates", geo="US", measure=["suicide", "homicide"], since=2025)
    assert isinstance(df, pandas.DataFrame)
    assert set(df["measure"]) == {"suicide", "homicide"}
