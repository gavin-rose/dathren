"""Public health data, cleaned up and documented.

    >>> import dathren
    >>> rows = dathren.fetch("cdc-vsrr-county-overdose", geo="OH")
    >>> dathren.total(dathren.latest(rows))      # suppressed counties as a range
    >>> rates = dathren.per_capita(rows)          # deaths per 100,000 residents
    >>> print(dathren.cite("cdc-vsrr-county-overdose")["apa"])
"""

from __future__ import annotations

import json
import math
import warnings
from collections.abc import Iterable
from typing import Any, Optional, Union

from . import _core

__version__: str = _core.__version__
__all__ = [
    "DathrenWarning",
    "cite",
    "datasets",
    "fetch",
    "fetch_df",
    "info",
    "latest",
    "per_capita",
    "total",
]


class DathrenWarning(UserWarning):
    """Raised for data caveats, like suppressed or modeled values, found while fetching."""


Rows = Union[list[dict[str, Any]], "pandas.DataFrame"]  # noqa: F821


def datasets() -> list[dict[str, Any]]:
    """Every dataset dathren knows about, with its measures and caveats."""
    return json.loads(_core.datasets_json())


def info(dataset: str) -> dict[str, Any]:
    """One dataset's description, measures, and caveats."""
    return json.loads(_core.info_json(dataset))


def fetch(
    dataset: str,
    *,
    geo: Optional[str] = None,
    measure: Union[str, Iterable[str], None] = None,
    since: Optional[int] = None,
    warn: bool = True,
) -> list[dict[str, Any]]:
    """Fetch normalized observations straight from the publisher.

    `geo` is `US`, a state abbreviation, a 5-digit county FIPS code, or a country code.
    `measure` takes one id or several; see `info(dataset)["measures"]`. Suppressed values
    come back as `None` with `"suppressed"` in `flags`, never as zero. Set `warn=False` to
    silence the `DathrenWarning` about suppressed and modeled values.
    """
    measures = [measure] if isinstance(measure, str) else list(measure or [])
    rows = json.loads(_core.fetch_json(dataset, geo, measures, since))
    if warn:
        _warn_flags(rows, dataset)
    return rows


def fetch_df(dataset: str, **kwargs: Any) -> "pandas.DataFrame":  # noqa: F821
    """Like `fetch`, as a pandas DataFrame. Needs `pip install 'dathren[pandas]'`."""
    return _pandas().DataFrame(fetch(dataset, **kwargs))


def cite(dataset: str, *, accessed: Optional[str] = None) -> dict[str, str]:
    """Citations for a dataset: `apa`, `bibtex`, `text`, and `dathren` for citing this tool.

    `accessed` is a `YYYY-MM-DD` date and defaults to today.
    """
    return json.loads(_core.cite_json(dataset, accessed))


def latest(rows: Rows) -> Rows:
    """Each place's newest value for each measure.

    Newer rows the publisher never filled in are skipped, so a state that missed a survey
    year shows its previous one. Suppressed rows are kept, so a stale number never stands
    in for a current suppression.
    """
    return _same_shape(rows, json.loads(_core.latest_json(_to_json(rows))))


def total(rows: Rows, *, suppressed: tuple[float, float] = (1, 9)) -> dict[str, Any]:
    """Sum one count measure, with suppressed cells as a range instead of zeros.

    Returns `reported` (the sum of known values), `low` and `high` (the sum if every
    suppressed cell held its smallest or largest possible value), and how many cells were
    reported or suppressed. NCHS suppresses counts of 1 to 9, the default range. Raises
    `ValueError` for rates or for more than one measure, since measures can overlap.
    """
    return json.loads(_core.total_json(_to_json(rows), float(suppressed[0]), float(suppressed[1])))


def per_capita(rows: Rows, *, per: int = 100_000) -> Rows:
    """Turn counts into rates per `per` residents with Census population for the same year.

    Works for the US, states, and counties. Measures are renamed, e.g. `overdose_deaths`
    becomes `overdose_deaths_per_100000`, and each row's `note` says which population year
    was used. Raises `ValueError` if the rows are already rates.
    """
    return _same_shape(rows, json.loads(_core.per_capita_json(_to_json(rows), float(per))))


def _pandas():
    try:
        import pandas
    except ImportError as e:
        raise ImportError("this needs pandas: pip install 'dathren[pandas]'") from e
    return pandas


def _is_frame(rows: Any) -> bool:
    return type(rows).__name__ == "DataFrame" and hasattr(rows, "to_dict")


def _clean(value: Any) -> Any:
    if isinstance(value, float) and math.isnan(value):
        return None
    if hasattr(value, "tolist"):
        return value.tolist()
    return value


def _to_json(rows: Rows) -> str:
    records = rows.to_dict("records") if _is_frame(rows) else list(rows)
    return json.dumps([{k: _clean(v) for k, v in r.items()} for r in records])


def _same_shape(original: Rows, records: list[dict[str, Any]]) -> Rows:
    return _pandas().DataFrame(records) if _is_frame(original) else records


def _warn_flags(rows: list[dict[str, Any]], dataset: str) -> None:
    counts = {flag: sum(flag in r["flags"] for r in rows) for flag in ("suppressed", "modeled")}
    notes = [f"{n} {flag}" for flag, n in counts.items() if n]
    if notes:
        warnings.warn(
            f"{dataset}: {', '.join(notes)} of {len(rows)} values. "
            f"Suppressed values are None, not zero. See dathren.info({dataset!r})['caveats'].",
            DathrenWarning,
            stacklevel=3,
        )
