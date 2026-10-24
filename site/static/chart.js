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
    const value = document.createElement("div");
    value.className = "v";
    value.textContent = nearest.value;
    const period = document.createElement("div");
    period.className = "p";
    period.textContent = `12 months ending ${nearest.period}`;
    tip.append(value, period);
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

for (const wrap of document.querySelectorAll(".chart-wrap[data-points]")) attachHover(wrap);
