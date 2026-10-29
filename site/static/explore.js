const base = document.currentScript.dataset.base;
const stateNames = Object.fromEntries(JSON.parse(document.currentScript.dataset.states).map((s) => [s.fips, s]));
const mapRoot = document.getElementById("result-map");
const viewTabs = [...document.querySelectorAll(".view-bar [role=tab]")];
let view = new URLSearchParams(location.search).get("view") === "map" ? "map" : "chart";
const catalog = JSON.parse(document.getElementById("catalog").textContent);
const form = document.getElementById("query");
const title = document.getElementById("result-title");
const legend = document.getElementById("result-legend");
const chartWrap = document.getElementById("result-chart");
const status = document.getElementById("result-status");
const command = document.getElementById("command");
const cache = new Map();
let selection = [];

function load(id, geo) {
  const url = geo === "US" ? `${base}data/${id}/us.json` : `${base}data/${id}/state/${geo}.json`;
  if (!cache.has(url)) cache.set(url, loadJson(url).catch((err) => (cache.delete(url), Promise.reject(err))));
  return cache.get(url);
}

function fillSelect(select, options, keep) {
  select.replaceChildren(...options.map(([value, label]) => new Option(label, value)));
  if (options.some(([value]) => value === keep)) select.value = keep;
}

async function update() {
  const { meta, states } = catalog.find((d) => d.meta.id === form.source.value);
  const mappable = states.length > 0;
  viewTabs.find((t) => t.dataset.view === "map").hidden = !mappable;
  if (!mappable) view = "chart";
  fillSelect(form.measure, meta.measures.map((m) => [m.id, m.label]), form.measure.value);
  fillSelect(form.geo, [["US", "United States"], ...states.map((s) => [s.abbr, s.name])], form.geo.value);
  const measure = meta.measures.find((m) => m.id === form.measure.value);
  const geo = form.geo.value;
  const geoName = geo === "US" ? "United States" : states.find((s) => s.abbr === geo).name;
  const countable = isCount(measure.unit);
  form.compare.disabled = geo === "US" || countable;
  form.compare.closest("label").title = countable ? "Counts aren't comparable across places of different sizes. Pick a rate to compare with the US." : "";

  let data, national;
  try {
    [data, national] = await Promise.all([load(meta.id, geo), !form.compare.disabled && form.compare.checked ? load(meta.id, "US") : null]);
  } catch (err) {
    title.textContent = meta.title;
    status.textContent = `Couldn't load data: ${err.message}`;
    return;
  }
  const years = [...new Set(data.map((o) => o.period.year))].sort((a, b) => a - b);
  fillSelect(form.since, years.map((y) => [String(y), String(y)]), form.since.value || String(years[0]));
  const since = Number(form.since.value);
  const pick = (rows) => rows.filter((o) => o.measure === measure.id && o.period.year >= since);

  selection = pick(data);
  const series = [{ name: geoName, rows: toRows(selection) }];
  if (national) series.push({ name: "US", rows: toRows(pick(national)), reference: true });

  for (const tab of viewTabs) {
    tab.setAttribute("aria-selected", String(tab.dataset.view === view));
    tab.tabIndex = tab.dataset.view === view ? 0 : -1;
  }
  chartWrap.hidden = legend.hidden = view === "map";
  mapRoot.hidden = view !== "map";
  command.textContent = `dathren fetch ${meta.id} --geo ${geo} --measure ${measure.id} --since ${since}`;
  const url = new URL(location.href);
  url.search = new URLSearchParams({ dataset: meta.id, measure: measure.id, geo, since, compare: form.compare.checked ? "1" : "0", ...(view === "map" && { view }) });
  history.replaceState(null, "", url);

  if (view === "map") {
    title.textContent = `${measure.label}, by state`;
    try {
      const data = await loadJson(`${base}data/${meta.id}/map.json`);
      const latest = data.measures[measure.id];
      const map = await createMap(mapRoot, { base, level: "state", mode: isCount(measure.unit) ? "bubbles" : "choropleth", unit: measure.unit, periodLabel: meta.period_label, stateNames });
      map.render(latest);
      status.textContent = `${meta.period_label} ${latest.period}. Click a state to chart it.` + (latest.note ? ` Older data where newer isn't available: ${latest.note}.` : "");
    } catch (err) {
      status.textContent = `Couldn't load the map: ${err.message}`;
    }
    return;
  }

  title.textContent = `${measure.label}, ${geoName}`;
  renderLegend(legend, series);
  const drawn = renderLineChart(chartWrap, series, { unit: measure.unit, periodLabel: meta.period_label });
  const shown = series[0].rows.length;
  const suppressed = selection.length - shown;
  status.textContent = drawn
    ? `${shown} periods since ${since}` + (suppressed ? `, ${suppressed} suppressed` : "") + `. ${meta.publisher}.`
    : `Not enough reported data to chart ${measure.label.toLowerCase()} for ${geoName}` + (suppressed ? ` (${suppressed} periods suppressed).` : ".");
}

function toCsv(rows) {
  const quote = (s) => (/[",\n]/.test(s) ? `"${s.replaceAll('"', '""')}"` : s);
  const lines = [["dataset", "measure", "geo", "geo_name", "period", "value", "adjusted_value", "ci_low", "ci_high", "flags"].join(",")];
  for (const o of rows) {
    lines.push([o.dataset, o.measure, o.geo, o.geo_name ?? "", periodLabel(o.period), o.value ?? "", o.adjusted_value ?? "", o.ci_low ?? "", o.ci_high ?? "", o.flags.join(";")].map((v) => quote(String(v))).join(","));
  }
  return lines.join("\n") + "\n";
}

document.getElementById("download").addEventListener("click", () => {
  const url = URL.createObjectURL(new Blob([toCsv(selection)], { type: "text/csv" }));
  const name = `${form.source.value}-${form.geo.value}-${form.measure.value}.csv`.toLowerCase();
  const a = Object.assign(document.createElement("a"), { href: url, download: name });
  a.click();
  URL.revokeObjectURL(url);
});

document.getElementById("copy").addEventListener("click", async (event) => {
  const label = event.currentTarget.querySelector("span");
  try {
    await navigator.clipboard.writeText(command.textContent);
    label.textContent = "Copied";
  } catch {
    label.textContent = "Copy failed";
  }
  setTimeout(() => (label.textContent = "Copy command"), 1500);
});

const params = new URLSearchParams(location.search);
fillSelect(form.source, catalog.map((d) => [d.meta.id, d.meta.title]), params.get("dataset"));
const initial = catalog.find((d) => d.meta.id === form.source.value);
fillSelect(form.measure, initial.meta.measures.map((m) => [m.id, m.label]), params.get("measure"));
fillSelect(form.geo, [["US", "United States"], ...initial.states.map((s) => [s.abbr, s.name])], params.get("geo")?.toUpperCase());
if (params.has("since")) form.since.replaceChildren(new Option(params.get("since"), params.get("since")));
form.compare.checked = params.get("compare") !== "0";
form.addEventListener("change", update);
for (const tab of viewTabs) {
  tab.addEventListener("click", () => {
    view = tab.dataset.view;
    update();
  });
}
mapRoot.addEventListener("mapselect", (event) => {
  const abbr = event.detail.abbr;
  if (![...form.geo.options].some((o) => o.value === abbr)) return;
  form.geo.value = abbr;
  view = "chart";
  update();
});
update();
