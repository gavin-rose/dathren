# dathren

Public health data, cleaned up and documented.

dathren fetches datasets from agencies like the CDC and returns them in one
consistent shape, with the quirks marked instead of hidden: suppressed cells,
provisional counts, and adjusted values all come through as explicit fields.

```sh
npm install dathren
```

```js
const dathren = require("dathren"); // or: import dathren from "dathren"

const rows = await dathren.fetch("cdc-vsrr-overdose", {
  geo: "US",
  measure: "synthetic_opioids",
  since: 2024,
});
// { dataset, measure, geo: "US", geo_name: "United States", period: "2026-05",
//   value: 33959, adjusted_value: 34507, flags: ["provisional", "underreported"], ... }

for (const c of dathren.info("cdc-places-county").caveats) {
  console.log(`${c.title}: ${c.detail}`);
}
```

Helpers for the steps that usually go wrong by hand:

```js
const ohio = await dathren.fetch("cdc-vsrr-county-overdose", { geo: "OH", since: 2026 });

dathren.total(dathren.latest(ohio));
// { reported: 2135, low: 2180, high: 2540, reported_count: 43, suppressed_count: 45 }

const rates = await dathren.perCapita(ohio); // deaths per 100,000, using Census population
console.log(dathren.cite("cdc-vsrr-county-overdose").apa);
```

`total` counts each suppressed county as somewhere from 1 to 9 instead of zero,
and refuses rates. `fetch` emits a `DathrenWarning` when results include
suppressed or modeled values.

Requests run on a background thread and return promises, so they won't block
the event loop. This package is for Node.js and other server-side runtimes.
In the browser, use the JSON files at [dathren.debugr.net](https://dathren.debugr.net).
