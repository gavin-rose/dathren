"use strict";

const binding = require("./binding.js");

function datasets() {
  return JSON.parse(binding.datasetsJson());
}

function info(dataset) {
  return JSON.parse(binding.infoJson(dataset));
}

async function fetch(dataset, { geo, measure, since } = {}) {
  const measures = measure == null ? [] : [].concat(measure);
  return JSON.parse(await binding.fetchJson(dataset, { geo, measures, since }));
}

module.exports = { datasets, info, fetch };
