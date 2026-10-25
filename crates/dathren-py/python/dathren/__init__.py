"""Public health data, cleaned up and documented.

    >>> import dathren
    >>> rows = dathren.fetch("cdc-vsrr-overdose", geo="US", measure="synthetic_opioids", since=2024)
    >>> df = dathren.fetch_df("brfss-mental-health", measure="mental_distress")
"""

from __future__ import annotations

import json
from collections.abc import Iterable
from typing import Any, Optional, Union

from . import _core

__version__: str = _core.__version__
__all__ = ["datasets", "info", "fetch", "fetch_df"]


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
) -> list[dict[str, Any]]:
    """Fetch normalized observations straight from the publisher.

    `geo` is `US`, a state abbreviation, or a 5-digit county FIPS code. `measure`
    takes one id or several; see `info(dataset)["measures"]`. Suppressed values
    come back as `None` with `"suppressed"` in `flags`, never as zero.
    """
    measures = [measure] if isinstance(measure, str) else list(measure or [])
    return json.loads(_core.fetch_json(dataset, geo, measures, since))


def fetch_df(dataset: str, **kwargs: Any) -> "pandas.DataFrame":  # noqa: F821
    """Like `fetch`, as a pandas DataFrame. Needs `pip install dathren[pandas]`."""
    try:
        import pandas
    except ImportError as e:
        raise ImportError("fetch_df needs pandas: pip install 'dathren[pandas]'") from e
    return pandas.DataFrame(fetch(dataset, **kwargs))
