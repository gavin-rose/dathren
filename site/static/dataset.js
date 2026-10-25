const page = document.getElementById("dataset").dataset;
const dataUrl = (name) => `${page.base}data/${page.id}/${name}.json`;
const cache = new Map();
const load = (name) => {
  if (!cache.has(name)) cache.set(name, loadJson(dataUrl(name)).catch((err) => (cache.delete(name), Promise.reject(err))));
  return cache.get(name);
};

function cell(text, className) {
  const td = document.createElement("td");
  td.textContent = text;
  if (className) td.className = className;
  return td;
}

const picker = document.getElementById("geo-picker");
if (picker) {
  const wrap = document.getElementById("main-chart");
  const legend = document.getElementById("chart-legend");
  const title = document.getElementById("chart-title");
  const value = document.getElementById("stat-value");
  const detail = document.getElementById("stat-detail");
  const table = document.getElementById("chart-table");
  const hasReported = Boolean(table?.dataset.reported);

  picker.addEventListener("change", async () => {
    const geo = picker.value;
    const name = picker.selectedOptions[0].textContent;
    let us, own;
    try {
      us = await load("us");
      own = geo === "US" ? us : await load(`state/${geo}`);
    } catch (err) {
      detail.textContent = `Couldn't load data: ${err.message}`;
      return;
    }
    const pick = (data) => data.filter((o) => o.measure === page.measure);
    const series = [{ name, rows: toRows(pick(own)) }];
    if (geo !== "US" && page.unit !== "deaths") series.push({ name: "US", rows: toRows(pick(us)), reference: true });

    title.textContent = `${page.label}, ${name}`;
    renderLegend(legend, series);
    if (!renderLineChart(wrap, series, { unit: page.unit, periodLabel: page.periodLabel })) {
      const note = document.createElement("p");
      note.className = "muted centered";
      note.textContent = `Not enough reported data to chart ${name}.`;
      wrap.replaceChildren(note);
    }

    const reported = pick(own).filter((o) => best(o) != null);
    const latest = reported.at(-1);
    value.textContent = latest ? formatValue(best(latest), page.unit) : "No data";
    const unit = page.unit === "deaths" ? "" : `${page.unit}, `;
    const soFar = latest?.adjusted_value != null && latest.value != null ? `, ${formatValue(latest.value, page.unit)} reported so far` : "";
    detail.textContent = latest ? `${unit}${page.periodLabel.toLowerCase()} ${periodLabel(latest.period)}${soFar}` : "";

    const url = new URL(location.href);
    if (geo === "US") url.searchParams.delete("geo");
    else url.searchParams.set("geo", geo);
    history.replaceState(null, "", url);

    if (table) {
      table.replaceChildren(
        ...reported.reverse().map((o) => {
          const tr = document.createElement("tr");
          tr.append(cell(periodLabel(o.period)), cell(formatValue(best(o), page.unit), "num"));
          if (hasReported) tr.append(cell(o.adjusted_value != null && o.value != null ? formatValue(o.value, page.unit) : "", "num"));
          return tr;
        }),
      );
    }
  });
}

const initialGeo = new URLSearchParams(location.search).get("geo")?.toUpperCase();
if (picker && initialGeo && [...picker.options].some((o) => o.value === initialGeo)) {
  picker.value = initialGeo;
  picker.dispatchEvent(new Event("change"));
}

const filter = document.getElementById("rank-filter");
if (filter) {
  const body = document.getElementById("rank-body");
  const note = document.getElementById("rank-note");
  const states = JSON.parse(filter.dataset.states);
  const nationwide = [...body.children];
  const nationwideNote = note.textContent;
  const hasCi = body.querySelector("td:nth-child(4)") !== null;

  filter.addEventListener("change", async () => {
    const url = new URL(location.href);
    if (filter.value) url.searchParams.set("geo", filter.value);
    else url.searchParams.delete("geo");
    history.replaceState(null, "", url);
    if (!filter.value) {
      body.replaceChildren(...nationwide);
      note.textContent = nationwideNote;
      return;
    }
    const state = states.find((s) => s.abbr === filter.value);
    let latest;
    try {
      latest = await load("latest");
    } catch (err) {
      note.textContent = `Couldn't load data: ${err.message}`;
      return;
    }
    const counties = latest.filter((o) => o.geo.length === 5 && o.geo.startsWith(state.fips));
    const reported = counties.filter((o) => o.value != null).sort((a, b) => b.value - a.value);
    const suppressed = counties.length - reported.length;
    body.replaceChildren(
      ...reported.map((o, i) => {
        const tr = document.createElement("tr");
        tr.append(cell(String(i + 1), "rank"), cell(o.geo_name ?? o.geo), cell(formatValue(o.value, page.unit), "num"));
        if (hasCi) tr.append(cell(o.ci_low != null ? `${formatValue(o.ci_low, page.unit)} to ${formatValue(o.ci_high, page.unit)}` : "", "num muted"));
        return tr;
      }),
    );
    note.textContent = `${reported.length} ${reported.length === 1 ? "county" : "counties"} in ${state.name}` + (suppressed ? `, plus ${suppressed} suppressed.` : ".");
  });

  if (initialGeo && [...filter.options].some((o) => o.value === initialGeo)) {
    filter.value = initialGeo;
    filter.dispatchEvent(new Event("change"));
  }
}
