# dathren

Public health data, cleaned up and documented.

dathren fetches datasets from agencies like the CDC and returns them in one
consistent shape, with the quirks marked instead of hidden: suppressed cells,
provisional counts, and adjusted values all come through as explicit fields.

```sh
pip install dathren            # or: pip install 'dathren[pandas]'
```

```python
import dathren

for d in dathren.datasets():
    print(d["id"], "-", d["title"])

rows = dathren.fetch("cdc-vsrr-overdose", geo="US", measure="synthetic_opioids", since=2024)
rows[-1]
# {'dataset': 'cdc-vsrr-overdose', 'measure': 'synthetic_opioids', 'geo': 'US',
#  'geo_name': 'United States', 'period': '2026-05', 'value': 33959.0,
#  'adjusted_value': 34507.0, 'flags': ['provisional', 'underreported'], ...}

df = dathren.fetch_df("brfss-mental-health", measure="mental_distress")
```

Helpers for the steps that usually go wrong by hand. They take a list of rows
or a DataFrame and give back the same kind:

```python
ohio = dathren.fetch("cdc-vsrr-county-overdose", geo="OH", since=2026)

dathren.total(dathren.latest(ohio))
# {'reported': 2135.0, 'low': 2180.0, 'high': 2540.0,
#  'reported_count': 43, 'suppressed_count': 45}

rates = dathren.per_capita(ohio)          # deaths per 100,000, using Census population
print(dathren.cite("cdc-vsrr-county-overdose")["apa"])
```

`total` counts each suppressed county as somewhere from 1 to 9 instead of zero,
and refuses rates. `fetch` raises a `DathrenWarning` when results include
suppressed or modeled values; pass `warn=False` to silence it.

Read each dataset's caveats before drawing conclusions:

```python
for c in dathren.info("cdc-places-county")["caveats"]:
    print(c["title"], "-", c["detail"])
```

Data is fetched live from the publisher on every call. Browse datasets, charts,
and caveats at [dathren.debugr.net](https://dathren.debugr.net).
