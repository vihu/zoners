// zoners: the page works without this file. It adds the live turn, label spacing, the
// date picker button, the ticking clock and the browser's own zone as the default source.
// Listeners sit on document because htmx replaces #stage after every request.
(() => {
  const $ = (s) => document.querySelector(s);
  const pad = (n) => String(n).padStart(2, "0");
  const hm = (m) => pad(Math.floor(m / 60)) + ":" + pad(Math.floor(m % 60));
  // Keep in step with Band::of_hour in src/band.rs.
  const band = (h) => (h >= 9 && h < 17 ? "work" : h >= 7 && h < 22 ? "awake" : "night");
  const BANDS = ["work", "awake", "night"];

  // First visit: the server only knows its own TZ, the browser knows where the reader is.
  // Only `zone` goes in the URL, so the page keeps meaning "now".
  if (!new URLSearchParams(location.search).has("zone") && $("#zone")) {
    const here = Intl.DateTimeFormat().resolvedOptions().timeZone;
    if (here && here !== $("#zone").value) return location.replace("?zone=" + encodeURIComponent(here));
  }

  // Turning the slider is answered locally so it stays smooth. Offsets are taken as fixed for
  // the day; the request on release brings back the server's answer, which knows about DST.
  function turn(t) {
    const stage = $("#stage");
    stage.removeAttribute("data-live"); // a chosen moment is no longer "now": colons stop blinking, refreshes stop
    const src = +stage.dataset.srcOff;
    const days = stage.dataset.days.split("|");
    $("#time").value = $("#captime").textContent = hm(t);
    $("#hand").style.setProperty("--a", t / 4 - 180 + "deg");
    for (const el of stage.querySelectorAll("[data-off]")) {
      const local = t + (el.dataset.off - src) / 60;
      const shift = Math.floor(local / 1440);
      const minutes = ((local % 1440) + 1440) % 1440;
      const name = band(Math.floor(minutes / 60));
      el.style.setProperty("--a", local / 4 - 180 + "deg");
      el.classList.remove(...BANDS);
      if (el.matches(".lbl")) {
        el.classList.add(name);
        el.querySelector("span").textContent = hm(minutes);
        const date = el.querySelector("em");
        date.textContent = days[shift + 1];
        date.hidden = shift === 0;
        el.style.setProperty("--extra", el.querySelectorAll("b").length - 1 + (shift ? 1 : 0));
      } else if (el.matches("tr")) {
        el.classList.add(name);
        el.querySelector(".tm").textContent = hm(minutes);
        el.querySelector(".bw").textContent = name;
        const day = el.querySelector(".dy");
        day.textContent = days[shift + 1];
        day.classList.toggle("other", shift !== 0);
      }
    }
    layout();
  }

  // Lift every second label where neighbours crowd near the top or bottom of the dial.
  // ponytail: labels one hour apart can still touch on the sides of a phone-sized dial,
  // and without JS nothing is lifted. Move this to the server if either matters.
  function layout() {
    const labels = [...document.querySelectorAll(".lbl")].map((el) => {
      const a = parseFloat(el.style.getPropertyValue("--a"));
      return { el, at: (((a + 180) * 4) % 1440 + 1440) % 1440, side: Math.abs(Math.sin((a * Math.PI) / 180)) };
    });
    labels.sort((p, q) => p.at - q.at);
    let prev = labels.length > 1 ? { ...labels[labels.length - 1], at: labels[labels.length - 1].at - 1440, tier: 0 } : null;
    for (const label of labels) {
      label.tier = prev && label.at - prev.at < 80 && label.side < 0.62 && !prev.tier ? 1 : 0;
      label.el.style.setProperty("--tier", label.tier);
      prev = label;
    }
  }

  document.addEventListener("input", (e) => {
    if (e.target.id === "turn") turn(+e.target.value);
  });
  document.addEventListener("change", (e) => {
    if (e.target.id === "flip") {
      const url = new URL(location);
      if (e.target.checked) url.searchParams.set("view", "table");
      else url.searchParams.delete("view");
      history.replaceState(history.state, "", url);
    } else if (e.target.id === "turn" || e.target.closest("#f")) {
      $("#f").requestSubmit(); // htmx answers the submit; without it this is a plain GET
    }
  });
  document.addEventListener("click", (e) => {
    if (e.target.closest("#datebtn")) $("#date").showPicker?.();
    if (e.target.closest("#theme")) {
      // Flip whatever is showing. Landing back on the system's scheme drops the override,
      // so the page follows the system again.
      const root = document.documentElement;
      const system = matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
      const next = (root.dataset.theme || system) === "dark" ? "light" : "dark";
      if (next === system) delete root.dataset.theme;
      else root.dataset.theme = next;
      try {
        if (next === system) localStorage.removeItem("theme");
        else localStorage.setItem("theme", next);
      } catch {} // private mode: the choice lasts for this page only
    }
  });
  document.addEventListener("htmx:afterSettle", layout);
  // Following the clock: a request that changes neither time nor date leaves both out,
  // so switching only the zone does not pin the moment.
  document.addEventListener("htmx:configRequest", (e) => {
    const [time, date] = [$("#time"), $("#date")];
    if ($("#stage").hasAttribute("data-live") && time.value === time.defaultValue && date.value === date.defaultValue) {
      delete e.detail.parameters.time;
      delete e.detail.parameters.date;
    }
  });

  // Live: with no time or date in the URL the page means "now". One request a minute keeps it
  // there, and the server stays the only place that knows about dates and DST.
  function refresh() {
    const editing = $("#f")?.contains(document.activeElement) || document.activeElement?.id === "turn";
    if (window.htmx && $("#stage")?.hasAttribute("data-live") && !document.hidden && !editing)
      htmx.ajax("GET", location.href, { target: "#stage", select: "#stage", swap: "outerHTML" });
  }
  const nextMinute = () => setTimeout(() => (refresh(), nextMinute()), 60000 - (Date.now() % 60000) + 250);
  nextMinute();
  document.addEventListener("visibilitychange", refresh);
  layout();
})();
