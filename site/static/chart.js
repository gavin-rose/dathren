const W = 720, H = 280, LEFT = 64, RIGHT = 16, TOP = 16, BOTTOM = 40, MAX_TICKS = 6;
const SVG_NS = "http://www.w3.org/2000/svg";

const thousands = (v) => Math.round(v).toLocaleString("en-US");
const formatValue = (v, unit) => (unit === "deaths" ? thousands(v) : v.toFixed(1));
const periodLabel = (p) => {
  if (p.month) return `${p.year}-${String(p.month).padStart(2, "0")}`;
  if (p.quarter) return `${p.year}-Q${p.quarter}`;
  return `${p.year}`;
};
const best = (o) => o.adjusted_value ?? o.value;

function compact(v) {
  const trim = (n) => String(Math.round(n * 10) / 10);
  if (v >= 1e6) return `${trim(v / 1e6)}M`;
  if (v >= 1e3) return `${trim(v / 1e3)}k`;
  return trim(v);
}

function niceScale(max) {
  if (max <= 0) return [1, 1];
  const magnitude = 10 ** Math.floor(Math.log10(max / MAX_TICKS));
  for (const n of [1, 2, 2.5, 5, 10, 20]) {
    const step = n * magnitude;
    const top = step * Math.ceil(max / step);
    if (top / step <= MAX_TICKS) return [step, top];
  }
}

function svgEl(name, attrs, text) {
  const node = document.createElementNS(SVG_NS, name);
  for (const [k, v] of Object.entries(attrs)) if (v != null) node.setAttribute(k, v);
  if (text !== undefined) node.textContent = text;
  return node;
}

function toRows(observations) {
  return observations.filter((o) => best(o) != null).map((o) => ({ label: periodLabel(o.period), v: best(o) }));
}

function renderLineChart(wrap, series, { unit, periodLabel: label }) {
  wrap.replaceChildren();
  delete wrap.dataset.points;
  const [main, reference] = series;
  if (!main || main.rows.length < 2) return false;

  const rows = main.rows;
  const refByLabel = new Map((reference?.rows ?? []).map((r) => [r.label, r.v]));
  const values = rows.map((r) => r.v).concat([...refByLabel.values()]);
  const [step, top] = niceScale(Math.max(...values));
  const plotW = W - LEFT - RIGHT, plotH = H - TOP - BOTTOM;
  const xAt = (i) => LEFT + (plotW * i) / (rows.length - 1);
  const yAt = (v) => TOP + plotH * (1 - v / top);
  const svg = svgEl("svg", { class: "chart", viewBox: `0 0 ${W} ${H}`, role: "img", "aria-label": `Line chart of ${series.map((s) => s.name).join(" and ")}` });

  const ticks = Math.round(top / step);
  for (let t = 0; t <= ticks; t++) {
    const y = yAt(step * t);
    svg.append(svgEl("line", { class: t === 0 ? "baseline" : "grid", x1: LEFT, x2: W - RIGHT, y1: y, y2: y }));
    svg.append(svgEl("text", { class: t % 2 ? "tick minor" : "tick", x: LEFT - 8, y: y + 4, "text-anchor": "end" }, compact(step * t)));
  }

  const yearStarts = rows.map((r, i) => (i === 0 || r.label.slice(0, 4) !== rows[i - 1].label.slice(0, 4) ? i : -1)).filter((i) => i >= 0);
  const every = Math.max(1, Math.ceil(yearStarts.length / 8));
  yearStarts.filter((_, n) => n % every === 0).forEach((i) => {
    svg.append(svgEl("text", { class: "tick", x: xAt(i), y: H - 10, "text-anchor": "middle" }, rows[i].label.slice(0, 4)));
  });

  const last = rows.length - 1;
  const refLast = refByLabel.get(rows[last].label);
  if (refByLabel.size) {
    const path = rows
      .map((r, i) => [i, refByLabel.get(r.label)])
      .filter(([, v]) => v != null)
      .map(([i, v], n) => `${n ? "L" : "M"}${xAt(i).toFixed(1)},${yAt(v).toFixed(1)}`)
      .join("");
    svg.append(svgEl("path", { class: "series reference", d: path }));
  }
  svg.append(svgEl("path", { class: "series", d: rows.map((r, i) => `${i ? "L" : "M"}${xAt(i).toFixed(1)},${yAt(r.v).toFixed(1)}`).join("") }));

  const mainAbove = refLast == null || rows[last].v >= refLast;
  if (refLast != null) {
    svg.append(svgEl("circle", { class: "marker reference", cx: xAt(last), cy: yAt(refLast), r: 4 }));
    svg.append(svgEl("text", { class: "direct-label muted-label", x: xAt(last), y: yAt(refLast) + (mainAbove ? 20 : -10), "text-anchor": "end" }, `${reference.name} ${formatValue(refLast, unit)}`));
  }
  svg.append(svgEl("circle", { class: "marker", cx: xAt(last), cy: yAt(rows[last].v), r: 4 }));
  const mainText = refLast != null ? `${main.name} ${formatValue(rows[last].v, unit)}` : formatValue(rows[last].v, unit);
  svg.append(svgEl("text", { class: "direct-label", x: xAt(last), y: yAt(rows[last].v) + (mainAbove ? -10 : 20), "text-anchor": "end" }, mainText));

  const hover = svgEl("g", { class: "hover", hidden: "" });
  hover.append(svgEl("line", { class: "crosshair", y1: TOP, y2: H - BOTTOM }), svgEl("circle", { class: "marker", r: 4 }));
  svg.append(hover);

  const tip = document.createElement("div");
  tip.className = "tooltip";
  tip.setAttribute("role", "status");
  tip.hidden = true;
  wrap.append(svg, tip);
  wrap.dataset.periodLabel = label;
  wrap.dataset.points = JSON.stringify(
    rows.map((r, i) => {
      const ref = refByLabel.get(r.label);
      const lines = [{ name: main.name, value: formatValue(r.v, unit) }];
      if (ref != null) lines.push({ name: reference.name, value: formatValue(ref, unit) });
      return { x: xAt(i), y: yAt(r.v), period: r.label, value: formatValue(r.v, unit), lines: refByLabel.size ? lines : undefined };
    }),
  );
  attachHover(wrap);
  return true;
}

function attachHover(wrap) {
  const points = JSON.parse(wrap.dataset.points);
  const svg = wrap.querySelector("svg");
  const hover = svg.querySelector(".hover");
  const line = hover.querySelector("line");
  const dot = hover.querySelector("circle");
  const tip = wrap.querySelector(".tooltip");

  const show = (event) => {
    const box = svg.getBoundingClientRect();
    const scale = svg.viewBox.baseVal.width / box.width;
    const x = (event.clientX - box.left) * scale;
    let nearest = points[0];
    for (const p of points) if (Math.abs(p.x - x) < Math.abs(nearest.x - x)) nearest = p;

    line.setAttribute("x1", nearest.x);
    line.setAttribute("x2", nearest.x);
    dot.setAttribute("cx", nearest.x);
    dot.setAttribute("cy", nearest.y);
    hover.removeAttribute("hidden");

    tip.replaceChildren();
    for (const entry of nearest.lines ?? [{ value: nearest.value }]) {
      const value = document.createElement("div");
      value.className = "v";
      value.textContent = entry.name ? `${entry.name}: ${entry.value}` : entry.value;
      tip.append(value);
    }
    const period = document.createElement("div");
    period.className = "p";
    period.textContent = `${wrap.dataset.periodLabel} ${nearest.period}`;
    tip.append(period);
    tip.hidden = false;

    const left = nearest.x / scale;
    const flip = left > box.width / 2;
    tip.style.left = flip ? "" : `${left + 12}px`;
    tip.style.right = flip ? `${box.width - left + 12}px` : "";
  };
  const hide = () => {
    hover.setAttribute("hidden", "");
    tip.hidden = true;
  };

  svg.addEventListener("pointermove", show);
  svg.addEventListener("pointerleave", hide);
}

function renderLegend(container, series) {
  container.replaceChildren();
  if (series.length < 2) return;
  for (const s of series) {
    const item = document.createElement("span");
    item.className = s.reference ? "legend-item reference" : "legend-item";
    const swatch = document.createElement("span");
    swatch.className = "swatch";
    item.append(swatch, document.createTextNode(s.name));
    container.append(item);
  }
}

async function loadJson(url) {
  const res = await fetch(url);
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
}

for (const wrap of document.querySelectorAll(".chart-wrap[data-points]")) attachHover(wrap);
