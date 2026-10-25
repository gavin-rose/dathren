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
