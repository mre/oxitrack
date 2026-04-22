(function () {
  function initChart(el) {
    if (!el || typeof echarts === "undefined") return;
    var labels = JSON.parse(el.getAttribute("data-labels") || "[]");
    var counts = JSON.parse(el.getAttribute("data-counts") || "[]");
    if (!labels.length) return;

    var existing = echarts.getInstanceByDom(el);
    if (existing) existing.dispose();

    var isDark = document.documentElement.dataset.theme !== "light";
    var cs = getComputedStyle(document.documentElement);
    var primary    = cs.getPropertyValue("--pico-primary").trim()     || "#0172ad";
    var mutedColor = cs.getPropertyValue("--pico-muted-color").trim() || "#888888";
    var splitColor = isDark ? "rgba(255,255,255,0.07)" : "rgba(0,0,0,0.07)";
    var tooltipBg  = isDark ? "#1e2328" : "#ffffff";
    var tooltipFg  = isDark ? "#e0e0e0" : "#333333";

    var instance = echarts.init(el, null, { renderer: "canvas" });
    instance.setOption({
      backgroundColor: "transparent",
      animation: false,
      grid: { top: 10, right: 10, bottom: 28, left: 42 },
      xAxis: {
        type: "category",
        data: labels,
        axisLine: { lineStyle: { color: splitColor } },
        axisTick: { show: false },
        axisLabel: { color: mutedColor, fontSize: 10, hideOverlap: true },
        splitLine: { show: false },
      },
      yAxis: {
        type: "value",
        minInterval: 1,
        axisLabel: { color: mutedColor, fontSize: 10 },
        splitLine: { lineStyle: { color: splitColor } },
        axisLine: { show: false },
        axisTick: { show: false },
      },
      // scroll to zoom only — no slider, no drag-pan
      dataZoom: [{
        type: "inside",
        xAxisIndex: 0,
        filterMode: "none",
        zoomOnMouseWheel: true,
        moveOnMouseMove: false,
        moveOnMouseWheel: false,
      }],
      tooltip: {
        trigger: "axis",
        axisPointer: {
          type: "shadow",
          shadowStyle: { color: isDark ? "rgba(255,255,255,0.04)" : "rgba(0,0,0,0.04)" },
        },
        backgroundColor: tooltipBg,
        borderColor: splitColor,
        padding: [6, 10],
        textStyle: { color: tooltipFg, fontSize: 12 },
        formatter: function (p) {
          var v = p[0].value;
          return '<span style="font-size:11px;color:' + mutedColor + '">' + p[0].name + "</span>"
               + "<br><strong>" + v + "</strong> visit" + (v === 1 ? "" : "s");
        },
      },
      series: [{
        type: "bar",
        data: counts,
        itemStyle: { color: primary, borderRadius: [3, 3, 0, 0] },
        emphasis: {
          itemStyle: { color: primary, opacity: 1, shadowBlur: 8, shadowColor: primary + "66" },
        },
        barMaxWidth: 32,
        opacity: 0.88,
      }],
    });

    window.addEventListener("resize", function () { instance.resize(); });
  }

  function initAll() {
    document.querySelectorAll(".visits-chart[data-labels]").forEach(initChart);
  }

  document.addEventListener("DOMContentLoaded", initAll);
  document.addEventListener("htmx:afterSettle", initAll);
  document.addEventListener("themechange", initAll);
})();
