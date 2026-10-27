const MAP_W = 975, MAP_H = 610;
const RAMP = ["#5179ab", "#6f97c6", "#93b6dc", "#bcd4ec", "#e4eef9"];
const topologies = new Map();

function loadTopology(url) {
  if (!topologies.has(url)) topologies.set(url, loadJson(url));
  return topologies.get(url);
}

function decodeArcs(topo) {
  const [sx, sy] = topo.transform.scale;
  const [tx, ty] = topo.transform.translate;
  return topo.arcs.map((arc) => {
    let x = 0, y = 0;
    return arc.map(([dx, dy]) => [(x += dx) * sx + tx, (y += dy) * sy + ty]);
  });
}

function rings(geometry) {
  if (geometry.type === "Polygon") return [geometry.arcs];
  if (geometry.type === "MultiPolygon") return geometry.arcs;
  return [];
}

function ringPoints(ring, arcs) {
  const points = [];
  ring.forEach((index, i) => {
    const arc = index >= 0 ? arcs[index] : arcs[~index].slice().reverse();
    points.push(...(i ? arc.slice(1) : arc));
  });
  return points;
}

function shapes(topo, object) {
  const arcs = decodeArcs(topo);
  return topo.objects[object].geometries.map((g) => {
    const polygons = rings(g).map((polygon) => polygon.map((ring) => ringPoints(ring, arcs)));
    const d = polygons.flat().map((pts) => "M" + pts.map(([x, y]) => `${x.toFixed(1)},${y.toFixed(1)}`).join("L") + "Z").join("");
    let best = null;
    for (const [outer] of polygons) {
      let area = 0, cx = 0, cy = 0;
      for (let i = 0, j = outer.length - 1; i < outer.length; j = i++) {
        const cross = outer[j][0] * outer[i][1] - outer[i][0] * outer[j][1];
        area += cross;
        cx += (outer[j][0] + outer[i][0]) * cross;
        cy += (outer[j][1] + outer[i][1]) * cross;
      }
      if (area && (!best || Math.abs(area) > Math.abs(best.area))) best = { area, x: cx / (3 * area), y: cy / (3 * area) };
    }
    const xs = polygons.flat(2).map((p) => p[0]);
    const ys = polygons.flat(2).map((p) => p[1]);
    return {
      id: g.id,
      name: g.properties?.name ?? g.id,
      d,
      centroid: best ? [best.x, best.y] : [0, 0],
      box: [Math.min(...xs), Math.min(...ys), Math.max(...xs), Math.max(...ys)],
    };
  });
}

function quantileBreaks(values, classes) {
  const sorted = [...values].sort((a, b) => a - b);
  const breaks = [];
  for (let i = 1; i < classes; i++) breaks.push(sorted[Math.floor((i * sorted.length) / classes)]);
  return [...new Set(breaks)];
}

function niceRound(v) {
  const magnitude = 10 ** Math.floor(Math.log10(v));
  return [1, 2, 5, 10].map((n) => n * magnitude).reverse().find((n) => n <= v) ?? magnitude;
}

async function createMap(root, { base, level, mode, unit, periodLabel, stateNames, names = {} }) {
  const topo = await loadTopology(`${base}geo/${level === "county" ? "counties" : "states"}-albers-10m.json`);
  const places = shapes(topo, level === "county" ? "counties" : "states");
  const stateShapes = level === "county" ? shapes(topo, "states") : places;
  const byId = new Map(places.map((p) => [p.id, p]));
  const svg = svgEl("svg", { class: "map", viewBox: `0 0 ${MAP_W} ${MAP_H}`, role: "img" });
  const defs = svgEl("defs", {});
  const hatch = svgEl("pattern", { id: `hatch-${level}`, width: 6, height: 6, patternUnits: "userSpaceOnUse", patternTransform: "rotate(45)" });
  hatch.append(svgEl("rect", { width: 6, height: 6, class: "no-data-fill" }), svgEl("line", { x1: 0, y1: 0, x2: 0, y2: 6, class: "no-data-line" }));
  defs.append(hatch);
  const shapeLayer = svgEl("g", {});
  const bubbleLayer = svgEl("g", { class: "bubbles" });
  const borderLayer = svgEl("g", { class: "state-borders" });
  svg.append(defs, shapeLayer, borderLayer, bubbleLayer);
  const tip = document.createElement("div");
  tip.className = "tooltip map-tip";
  tip.hidden = true;
  const legend = root.querySelector(".map-legend");
  const canvas = root.querySelector(".map-canvas");
  canvas.replaceChildren(svg, tip);
  if (level === "county") for (const s of stateShapes) borderLayer.append(svgEl("path", { d: s.d }));

  const nameOf = (id) => (level === "county" ? names[id] ?? `${byId.get(id)?.name ?? id}, ${stateNames[id.slice(0, 2)]?.abbr ?? ""}` : stateNames[id]?.name ?? byId.get(id)?.name ?? id);
  let current = null;

  function showTip(event, id) {
    const box = canvas.getBoundingClientRect();
    const row = current?.values[id];
    tip.replaceChildren();
    const name = document.createElement("div");
    name.className = "v";
    name.textContent = nameOf(id);
    const value = document.createElement("div");
    value.textContent = row?.[0] != null ? formatValue(row[0], unit) + (row[1] != null ? ` (95% CI ${formatValue(row[1], unit)} to ${formatValue(row[2], unit)})` : "") : row ? "Suppressed" : "No data";
    const period = document.createElement("div");
    period.className = "p";
    period.textContent = `${periodLabel} ${row?.[3] ?? current.period}`;
    tip.append(name, value, period);
    tip.hidden = false;
    const x = event.clientX - box.left, y = event.clientY - box.top;
    const flip = x > box.width / 2;
    tip.style.left = flip ? "" : `${x + 14}px`;
    tip.style.right = flip ? `${box.width - x + 14}px` : "";
    tip.style.top = `${y + 14}px`;
  }
  svg.addEventListener("pointerleave", () => (tip.hidden = true));

  function bind(node, id) {
    node.addEventListener("pointermove", (e) => showTip(e, id));
    node.addEventListener("click", () => root.dispatchEvent(new CustomEvent("mapselect", { detail: { id, abbr: stateNames[id]?.abbr } })));
  }

  function render(measure) {
    current = measure;
    shapeLayer.replaceChildren();
    bubbleLayer.replaceChildren();
    legend.replaceChildren();
    const reported = Object.values(measure.values).map((r) => r[0]).filter((v) => v != null);
    const noData = `url(#hatch-${level})`;

    if (mode === "choropleth") {
      const breaks = quantileBreaks(reported, RAMP.length);
      const colorOf = (v) => (v == null ? noData : RAMP[breaks.filter((b) => v >= b).length]);
      for (const p of places) {
        const row = measure.values[p.id];
        const path = svgEl("path", { d: p.d, class: "place", fill: row ? colorOf(row[0]) : noData, "data-id": p.id });
        bind(path, p.id);
        shapeLayer.append(path);
      }
      const edges = [Math.min(...reported), ...breaks, Math.max(...reported)];
      RAMP.slice(0, edges.length - 1).forEach((color, i) => {
        const item = document.createElement("span");
        item.className = "legend-item";
        const swatch = document.createElement("span");
        swatch.className = "swatch-box";
        swatch.style.background = color;
        item.append(swatch, document.createTextNode(`${formatValue(edges[i], unit)} to ${formatValue(edges[i + 1], unit)}`));
        legend.append(item);
      });
    } else {
      const max = Math.max(...reported);
      const maxR = level === "county" ? 16 : 30;
      const radius = (v) => maxR * Math.sqrt(v / max);
      for (const p of places) {
        const path = svgEl("path", { d: p.d, class: "place base", fill: measure.values[p.id] && measure.values[p.id][0] == null ? noData : null, "data-id": p.id });
        bind(path, p.id);
        shapeLayer.append(path);
      }
      const circles = Object.entries(measure.values)
        .filter(([id, r]) => r[0] && byId.has(id))
        .sort((a, b) => b[1][0] - a[1][0]);
      for (const [id, r] of circles) {
        const [cx, cy] = byId.get(id).centroid;
        const circle = svgEl("circle", { cx, cy, r: radius(r[0]), class: "bubble" });
        bind(circle, id);
        bubbleLayer.append(circle);
      }
      for (const v of [niceRound(max), niceRound(max / 4)]) {
        const item = document.createElement("span");
        item.className = "legend-item";
        const size = 2 * radius(v);
        const swatch = document.createElement("span");
        swatch.className = "swatch-circle";
        swatch.style.width = swatch.style.height = `${size}px`;
        item.append(swatch, document.createTextNode(formatValue(v, unit)));
        legend.append(item);
      }
    }
    const missing = document.createElement("span");
    missing.className = "legend-item";
    const swatch = document.createElement("span");
    swatch.className = "swatch-box hatched";
    missing.append(swatch, document.createTextNode("Suppressed or no data"));
    legend.append(missing);
    svg.setAttribute("aria-label", `Map of ${Object.keys(measure.values).length} places for ${measure.period}; see the table below for values`);
  }

  function zoomTo(stateFips) {
    const shape = stateFips && stateShapes.find((s) => s.id === stateFips);
    let view = [0, 0, MAP_W, MAP_H];
    if (shape) {
      const [x0, y0, x1, y1] = shape.box;
      const pad = Math.max(x1 - x0, y1 - y0) * 0.08;
      view = [x0 - pad, y0 - pad, x1 - x0 + 2 * pad, y1 - y0 + 2 * pad];
    }
    svg.setAttribute("viewBox", view.join(" "));
    hatch.setAttribute("patternTransform", `rotate(45) scale(${Math.max(view[2] / MAP_W, view[3] / MAP_H)})`);
    for (const node of shapeLayer.children) node.classList.toggle("dim", Boolean(shape) && !node.dataset.id.startsWith(stateFips));
  }

  function table(measure, stateFips) {
    return Object.entries(measure.values)
      .filter(([id]) => !stateFips || id.startsWith(stateFips))
      .map(([id, r]) => ({ id, name: nameOf(id), value: r[0], lo: r[1], hi: r[2] }))
      .sort((a, b) => (b.value ?? -Infinity) - (a.value ?? -Infinity));
  }

  function undrawn(measure) {
    return Object.keys(measure.values).filter((id) => !byId.has(id)).map(nameOf);
  }

  return { render, zoomTo, table, nameOf, undrawn };
}

function fillMapTable(body, rows, unit) {
  const cell = (text, className) => {
    const td = document.createElement("td");
    td.textContent = text;
    if (className) td.className = className;
    return td;
  };
  const hasCi = rows.some((r) => r.lo != null);
  body.replaceChildren(
    ...rows.map((r) => {
      const tr = document.createElement("tr");
      tr.append(cell(r.name), cell(r.value == null ? "Suppressed" : formatValue(r.value, unit), "num"));
      if (hasCi) tr.append(cell(r.lo != null ? `${formatValue(r.lo, unit)} to ${formatValue(r.hi, unit)}` : "", "num muted"));
      return tr;
    }),
  );
  return hasCi;
}
