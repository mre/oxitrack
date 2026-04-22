(function () {
  var PRESETS = {
    today:  function (t) { return [t, t]; },
    "7":    function (t) { return [daysAgo(t, 7),  t]; },
    "30":   function (t) { return [daysAgo(t, 30), t]; },
    "90":   function (t) { return [daysAgo(t, 90), t]; },
    "365":  function (t) { return [daysAgo(t, 365), t]; },
    "all":  function ()  { return ["", ""]; },
  };

  function toISO(d) {
    return d.getFullYear() + "-"
      + String(d.getMonth() + 1).padStart(2, "0") + "-"
      + String(d.getDate()).padStart(2, "0");
  }

  function daysAgo(todayStr, n) {
    // subtract n days from today string to avoid DST issues
    var d = new Date(todayStr + "T12:00:00");
    d.setDate(d.getDate() - n);
    return toISO(d);
  }

  function getPath() {
    var el = document.getElementById("hx-path");
    return el ? el.value : "";
  }

  function buildUrl(from, to) {
    var path = getPath();
    var params = [];
    if (from) params.push("from=" + from);
    if (to)   params.push("to="   + to);
    if (path) params.push("path=" + encodeURIComponent(path));
    return "/hx/stats" + (params.length ? "?" + params.join("&") : "");
  }

  function markActive() {
    var panel = document.getElementById("stats-panel");
    if (!panel) return;
    var curFrom = panel.getAttribute("data-from") || "";
    var curTo   = panel.getAttribute("data-to")   || "";
    var today   = toISO(new Date());

    document.querySelectorAll("[data-preset]").forEach(function (btn) {
      var key    = btn.getAttribute("data-preset");
      var fn     = PRESETS[key];
      var dates  = fn ? fn(today) : null;
      var active = dates && dates[0] === curFrom && dates[1] === curTo;
      if (active) {
        btn.removeAttribute("class");          // remove outline/secondary
        btn.setAttribute("aria-current", "page");
      } else {
        btn.setAttribute("class", "outline secondary");
        btn.removeAttribute("aria-current");
      }
    });
  }

  function init() {
    document.querySelectorAll("[data-preset]").forEach(function (btn) {
      if (btn._fb) return;
      btn._fb = true;
      btn.addEventListener("click", function () {
        var key   = this.getAttribute("data-preset");
        var today = toISO(new Date());
        var dates = PRESETS[key] ? PRESETS[key](today) : ["", ""];
        htmx.ajax("GET", buildUrl(dates[0], dates[1]),
                  { target: "#stats-panel", swap: "outerHTML" });
      });
    });
    markActive();
  }

  document.addEventListener("DOMContentLoaded", init);
  document.addEventListener("htmx:afterSettle", init);
})();
