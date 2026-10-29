const test = require("node:test");
const assert = require("node:assert");
const dathren = require("..");

const online = process.env.DATHREN_OFFLINE !== "1";

test("lists datasets", () => {
  const ids = dathren.datasets().map((d) => d.id);
  assert.ok(ids.includes("cdc-vsrr-overdose"));
  assert.ok(ids.includes("cdc-places-county"));
});

test("unknown ids throw", async () => {
  assert.throws(() => dathren.info("nope"), /unknown dataset/);
  await assert.rejects(dathren.fetch("cdc-places-county", { measure: "nope" }), /unknown measure/);
});

test("fetches flat records", { skip: !online }, async () => {
  const rows = await dathren.fetch("brfss-mental-health", { geo: "CA", measure: "mental_distress" });
  assert.ok(rows.length > 0);
  for (const r of rows) {
    assert.strictEqual(r.geo, "CA");
    assert.strictEqual(typeof r.period, "string");
  }
});

const row = (geo, period, value, flags = [], measure = "overdose_deaths", dataset = "cdc-vsrr-county-overdose") => ({
  dataset, measure, geo, geo_name: null, period, value, adjusted_value: null, ci_low: null, ci_high: null, flags, note: null,
});

test("cite, latest, and total", () => {
  assert.match(dathren.cite("cdc-vsrr-overdose", { accessed: "2026-10-29" }).apa, /Retrieved October 29, 2026/);
  assert.deepStrictEqual(dathren.latest([row("39001", "2026-02", 38), row("39001", "2026-03", 40)]).map((r) => r.period), ["2026-03"]);
  const t = dathren.total([row("39001", "2026-03", 40), row("39003", "2026-03", null, ["suppressed"])]);
  assert.deepStrictEqual([t.low, t.high], [41, 49]);
  assert.throws(() => dathren.total([row("US", "2025-Q2", 13.7, [], "suicide", "nchs-mortality-rates")]), /rate or percent/);
});

test("perCapita uses Census population", { skip: !online }, async () => {
  const rows = await dathren.fetch("cdc-vsrr-county-overdose", { geo: "39049", since: 2026, warn: false });
  const rates = await dathren.perCapita(rows);
  assert.ok(rates.every((r) => r.measure === "overdose_deaths_per_100000"));
});
