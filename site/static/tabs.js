const LANG_KEY = "dathren-lang";

for (const group of document.querySelectorAll(".tabs")) {
  const tabs = [...group.querySelectorAll('[role="tab"]')];
  const panels = [...group.querySelectorAll('[role="tabpanel"]')];
  const copy = group.querySelector(".copy-code");

  const select = (lang, focus) => {
    for (const tab of tabs) {
      const active = tab.dataset.lang === lang;
      tab.setAttribute("aria-selected", String(active));
      tab.tabIndex = active ? 0 : -1;
      if (active && focus) tab.focus();
    }
    for (const panel of panels) panel.hidden = panel.dataset.lang !== lang;
  };

  tabs.forEach((tab, i) => {
    tab.addEventListener("click", () => {
      select(tab.dataset.lang);
      try {
        localStorage.setItem(LANG_KEY, tab.dataset.lang);
      } catch {}
    });
    tab.addEventListener("keydown", (event) => {
      const step = { ArrowRight: 1, ArrowLeft: -1 }[event.key];
      if (step) select(tabs[(i + step + tabs.length) % tabs.length].dataset.lang, true);
    });
  });

  copy.addEventListener("click", async () => {
    const panel = panels.find((p) => !p.hidden);
    const text = [...panel.querySelector("code").childNodes]
      .filter((node) => !node.classList?.contains("prompt"))
      .map((node) => node.textContent)
      .join("")
      .replace(/^ /gm, "");
    const icon = copy.querySelector("span");
    try {
      await navigator.clipboard.writeText(text);
      icon.textContent = "check";
    } catch {
      icon.textContent = "error";
    }
    setTimeout(() => (icon.textContent = "content_copy"), 1500);
  });

  let saved = null;
  try {
    saved = localStorage.getItem(LANG_KEY);
  } catch {}
  if (tabs.some((t) => t.dataset.lang === saved)) select(saved);
}
