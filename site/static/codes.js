const input = document.getElementById("code-search");
const results = document.getElementById("search-results");
const local = JSON.parse(document.getElementById("local-codes").textContent);
const NLM = "https://clinicaltables.nlm.nih.gov/api";
const REMOTE = [
  { system: "ICD-10-CM", note: "US diagnosis codes. Death data uses the WHO's ICD-10 instead.", url: (q) => `${NLM}/icd10cm/v3/search?sf=code,name&df=code,name&maxList=8&terms=${q}` },
  { system: "HCPCS Level II", note: "Supplies, drugs, and services billed to Medicare and Medicaid.", url: (q) => `${NLM}/hcpcs/v3/search?df=code,display&maxList=8&terms=${q}` },
];
let controller = null;
let timer = null;

function el(tag, className, text) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text != null) node.textContent = text;
  return node;
}

function section(title, note, rows, total) {
  const wrap = el("div", "result-group");
  const head = el("div", "result-head");
  head.append(el("span", "result-title", title));
  head.append(el("span", "muted", total > rows.length ? `${rows.length} of ${total.toLocaleString("en-US")}` : `${rows.length}`));
  wrap.append(head);
  if (note) wrap.append(el("p", "muted result-note", note));
  const list = el("ul", "result-list");
  for (const r of rows) {
    const item = el("li");
    item.append(el("span", "code", r.code));
    if (r.href) {
      const link = el("a", null, r.label);
      link.href = r.href;
      item.append(link);
    } else {
      item.append(el("span", null, r.label));
    }
    list.append(item);
  }
  wrap.append(list);
  return wrap;
}

async function search(query) {
  controller?.abort();
  if (query.length < 2) {
    results.hidden = true;
    results.replaceChildren();
    return;
  }
  controller = new AbortController();
  const q = query.toLowerCase();
  const mine = local.filter((c) => c.code.toLowerCase().includes(q) || c.label.toLowerCase().includes(q));
  results.hidden = false;
  results.replaceChildren(el("div", "card-header", `Results for "${query}"`), el("p", "muted centered", "Searching..."));

  const groups = [];
  if (mine.length) groups.push(section("On dathren", "Codes behind our datasets and state FIPS codes.", mine.slice(0, 12), mine.length));
  const remote = await Promise.all(
    REMOTE.map(async (src) => {
      try {
        const res = await fetch(src.url(encodeURIComponent(query)), { signal: controller.signal });
        const [total, , , rows] = await res.json();
        return total ? section(src.system, src.note, rows.map(([code, label]) => ({ code, label })), total) : null;
      } catch (err) {
        if (err.name === "AbortError") throw err;
        return section(src.system, `Couldn't reach the National Library of Medicine. Try again in a moment.`, [], 0);
      }
    }),
  ).catch(() => null);
  if (!remote) return;
  groups.push(...remote.filter(Boolean));

  const body = groups.length ? groups : [el("p", "muted centered", "No matching codes.")];
  results.replaceChildren(el("div", "card-header", `Results for "${query}"`), ...body, el("p", "muted result-source", "ICD-10-CM and HCPCS results come live from the National Library of Medicine's Clinical Tables."));
}

input.addEventListener("input", () => {
  clearTimeout(timer);
  timer = setTimeout(() => search(input.value.trim()), 250);
});

const initial = new URLSearchParams(location.search).get("q");
if (initial) {
  input.value = initial;
  search(initial);
}
input.addEventListener("change", () => {
  const url = new URL(location.href);
  if (input.value.trim()) url.searchParams.set("q", input.value.trim());
  else url.searchParams.delete("q");
  history.replaceState(null, "", url);
});
