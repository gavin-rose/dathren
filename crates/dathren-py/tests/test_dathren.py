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


def test_cite_formats():
    c = dathren.cite("cdc-vsrr-overdose", accessed="2026-10-29")
    assert "Retrieved October 29, 2026" in c["apa"]
    assert c["bibtex"].startswith("@misc{cdcvsrroverdose2026,")
    assert "dathren" in c["dathren"]


def _row(geo, period, value, flags=(), measure="overdose_deaths", dataset="cdc-vsrr-county-overdose"):
    return {
        "dataset": dataset, "measure": measure, "geo": geo, "geo_name": None, "period": period,
        "value": value, "adjusted_value": None, "ci_low": None, "ci_high": None,
        "flags": list(flags), "note": None,
    }


def test_total_bounds_suppression_and_refuses_rates():
    rows = [_row("39001", "2026-03", 40.0), _row("39003", "2026-03", None, ["suppressed"])]
    t = dathren.total(rows)
    assert (t["reported"], t["low"], t["high"], t["suppressed_count"]) == (40.0, 41.0, 49.0, 1)
    with pytest.raises(ValueError, match="rate or percent"):
        dathren.total([_row("US", "2025-Q2", 13.7, measure="suicide", dataset="nchs-mortality-rates")])


def test_latest_accepts_dataframes():
    pandas = pytest.importorskip("pandas")
    df = pandas.DataFrame([_row("39001", "2026-02", 38.0), _row("39001", "2026-03", 40.0)])
    out = dathren.latest(df)
    assert isinstance(out, pandas.DataFrame)
    assert out["period"].tolist() == ["2026-03"]


@network
def test_fetch_warns_about_suppression():
    with pytest.warns(dathren.DathrenWarning, match="suppressed"):
        dathren.fetch("cdc-vsrr-county-overdose", geo="RI", since=2026)


@network
def test_per_capita_uses_census_population():
    rows = dathren.fetch("cdc-vsrr-county-overdose", geo="39049", since=2025, warn=False)
    rates = dathren.per_capita(rows)
    assert rates and all(r["measure"] == "overdose_deaths_per_100000" for r in rates)
    assert "Census population" in rates[-1]["note"]
