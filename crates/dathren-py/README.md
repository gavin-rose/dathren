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

Read each dataset's caveats before drawing conclusions:

```python
for c in dathren.info("cdc-places-county")["caveats"]:
    print(c["title"], "-", c["detail"])
```

Data is fetched live from the publisher on every call. Browse datasets, charts,
and caveats at [dathren.debugr.net](https://dathren.debugr.net).
