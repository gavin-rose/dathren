const base = document.currentScript.dataset.base;
const catalog = JSON.parse(document.getElementById("catalog").textContent);
const form = document.getElementById("query");
const title = document.getElementById("result-title");
const chartWrap = document.getElementById("result-chart");
const status = document.getElementById("result-status");
const command = document.getElementById("command");
const cache = new Map();
let selection = [];

const W = 720, H = 280, LEFT = 64, RIGHT = 16, TOP = 16, BOTTOM = 40, MAX_TICKS = 6;
const SVG_NS = "http://www.w3.org/2000/svg";

const thousands = (v) => Math.round(v).toLocaleString("en-US");
const formatValue = (v, unit) => (unit === "deaths" ? thousands(v) : v.toFixed(1));
const compact = (v) => {
  const trim = (n) => String(Math.round(n * 10) / 10);
  if (v >= 1e6) return `${trim(v / 1e6)}M`;
  if (v >= 1e3) return `${trim(v / 1e3)}k`;
  return trim(v);
};
const periodLabel = (p) => {
  if (p.month) return `${p.year}-${String(p.month).padStart(2, "0")}`;
  if (p.quarter) return `${p.year}-Q${p.quarter}`;
  return `${p.year}`;
};
const best = (o) => o.adjusted_value ?? o.value;

function niceScale(max) {
  if (max <= 0) return [1, 1];
  const magnitude = 10 ** Math.floor(Math.log10(max / MAX_TICKS));
  for (const n of [1, 2, 2.5, 5, 10, 20]) {
    const step = n * magnitude;
    const top = step * Math.ceil(max / step);
    if (top / step <= MAX_TICKS) return [step, top];
  }
}

function el(name, attrs, text) {
  const node = document.createElementNS(SVG_NS, name);
  for (const [k, v] of Object.entries(attrs)) node.setAttribute(k, v);
  if (text !== undefined) node.textContent = text;
  return node;
}

function renderChart(rows, unit, label) {
  chartWrap.replaceChildren();
  delete chartWrap.dataset.points;
  if (rows.length < 2) return;

  const [step, top] = niceScale(Math.max(...rows.map((r) => r.v)));
  const plotW = W - LEFT - RIGHT, plotH = H - TOP - BOTTOM;
  const xAt = (i) => LEFT + (plotW * i) / (rows.length - 1);
  const yAt = (v) => TOP + plotH * (1 - v / top);
  const svg = el("svg", { class: "chart", viewBox: `0 0 ${W} ${H}`, role: "img", "aria-label": "Line chart of the selected measure" });

  const ticks = Math.round(top / step);
  for (let t = 0; t <= ticks; t++) {
    const y = yAt(step * t);
    svg.append(el("line", { class: t === 0 ? "baseline" : "grid", x1: LEFT, x2: W - RIGHT, y1: y, y2: y }));
    svg.append(el("text", { class: t % 2 ? "tick minor" : "tick", x: LEFT - 8, y: y + 4, "text-anchor": "end" }, compact(step * t)));
  }

  const yearStarts = rows.map((r, i) => (i === 0 || r.label.slice(0, 4) !== rows[i - 1].label.slice(0, 4) ? i : -1)).filter((i) => i >= 0);
  const every = Math.max(1, Math.ceil(yearStarts.length / 8));
  yearStarts.filter((_, n) => n % every === 0).forEach((i) => {
    svg.append(el("text", { class: "tick", x: xAt(i), y: H - 10, "text-anchor": "middle" }, rows[i].label.slice(0, 4)));
  });

  svg.append(el("path", { class: "series", d: rows.map((r, i) => `${i ? "L" : "M"}${xAt(i).toFixed(1)},${yAt(r.v).toFixed(1)}`).join("") }));
  const last = rows.length - 1;
  svg.append(el("circle", { class: "marker", cx: xAt(last), cy: yAt(rows[last].v), r: 4 }));
  svg.append(el("text", { class: "direct-label", x: xAt(last), y: yAt(rows[last].v) - 10, "text-anchor": "end" }, formatValue(rows[last].v, unit)));

  const hover = el("g", { class: "hover", hidden: "" });
  hover.append(el("line", { class: "crosshair", y1: TOP, y2: H - BOTTOM }), el("circle", { class: "marker", r: 4 }));
  svg.append(hover);

  const tip = document.createElement("div");
  tip.className = "tooltip";
  tip.setAttribute("role", "status");
  tip.hidden = true;
  chartWrap.append(svg, tip);
  chartWrap.dataset.periodLabel = label;
  chartWrap.dataset.points = JSON.stringify(rows.map((r, i) => ({ x: xAt(i), y: yAt(r.v), period: r.label, value: formatValue(r.v, unit) })));
  attachHover(chartWrap);
}

async function load(id) {
  if (!cache.has(id)) {
    const res = await fetch(`${base}data/${id}/us.json`);
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    cache.set(id, await res.json());
  }
  return cache.get(id);
}

function fillSelect(select, options, keep) {
  select.replaceChildren(...options.map(([value, label]) => new Option(label, value)));
  if (options.some(([value]) => value === keep)) select.value = keep;
}

async function update() {
  const meta = catalog.find((d) => d.id === form.source.value);
  fillSelect(form.measure, meta.measures.map((m) => [m.id, m.label]), form.measure.value);
  const measure = meta.measures.find((m) => m.id === form.measure.value);

  let data;
  try {
    data = await load(meta.id);
  } catch (err) {
    title.textContent = meta.title;
    status.textContent = `Couldn't load data: ${err.message}`;
    return;
  }
  const years = [...new Set(data.map((o) => o.period.year))].sort((a, b) => a - b);
  fillSelect(form.since, years.map((y) => [String(y), String(y)]), form.since.value || String(years[0]));
  const since = Number(form.since.value);

  selection = data.filter((o) => o.measure === measure.id && o.period.year >= since);
  const rows = selection.filter((o) => best(o) != null).map((o) => ({ label: periodLabel(o.period), v: best(o) }));

  title.textContent = `${measure.label}, United States`;
  renderChart(rows, measure.unit, meta.period_label);
  const suppressed = selection.length - rows.length;
  status.textContent = `${rows.length} periods since ${since}` + (suppressed ? `, ${suppressed} suppressed` : "") + `. ${meta.publisher}.`;
  command.textContent = `dathren fetch ${meta.id} --geo ${form.geo.value} --measure ${measure.id} --since ${since}`;
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
  const a = Object.assign(document.createElement("a"), { href: url, download: `${form.source.value}-${form.measure.value}.csv` });
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

fillSelect(form.source, catalog.map((d) => [d.id, d.title]));
form.addEventListener("change", update);
update();
