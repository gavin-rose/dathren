"use strict";

const binding = require("./binding.js");

function datasets() {
  return JSON.parse(binding.datasetsJson());
}

function info(dataset) {
  return JSON.parse(binding.infoJson(dataset));
}

async function fetch(dataset, { geo, measure, since, warn = true } = {}) {
  const measures = measure == null ? [] : [].concat(measure);
  const rows = JSON.parse(await binding.fetchJson(dataset, { geo, measures, since }));
  if (warn) warnFlags(rows, dataset);
  return rows;
}

function cite(dataset, { accessed } = {}) {
  return JSON.parse(binding.citeJson(dataset, accessed));
}

function latest(rows) {
  return JSON.parse(binding.latestJson(JSON.stringify(rows)));
}

function total(rows, { suppressed = [1, 9] } = {}) {
  return JSON.parse(binding.totalJson(JSON.stringify(rows), suppressed[0], suppressed[1]));
}

async function perCapita(rows, { per = 100000 } = {}) {
  return JSON.parse(await binding.perCapitaJson(JSON.stringify(rows), per));
}

function warnFlags(rows, dataset) {
  const notes = ["suppressed", "modeled"]
    .map((flag) => [flag, rows.filter((r) => r.flags.includes(flag)).length])
    .filter(([, n]) => n)
    .map(([flag, n]) => `${n} ${flag}`);
  if (notes.length) {
    process.emitWarning(
      `${dataset}: ${notes.join(", ")} of ${rows.length} values. Suppressed values are null, not zero. See info("${dataset}").caveats.`,
      { type: "DathrenWarning" },
    );
  }
}

module.exports = { datasets, info, fetch, cite, latest, total, perCapita };
