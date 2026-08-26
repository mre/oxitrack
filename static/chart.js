(function () {
  var ZOOM_REQUEST_DELAY_MS = 250;
  var CHART_SELECTOR = ".visits-chart[data-labels]";
  var zoomRequestTimer;
  var cursorTimers = new WeakMap();
  var chartResizeObserver =
    typeof ResizeObserver === "undefined"
      ? null
      : new ResizeObserver(function (entries) {
          entries.forEach(function (entry) {
            var instance = echarts.getInstanceByDom(entry.target);
            if (instance) instance.resize();
          });
        });

  document.addEventListener("htmx:beforeRequest", function (event) {
    var target = event.detail && event.detail.target;
    if (target && target.id === "stats-panel") {
      // A later preset or tab selection should win over a zoom still debouncing.
      clearTimeout(zoomRequestTimer);
    }
  });

  function clampIndex(index, labels) {
    return Math.max(0, Math.min(labels.length - 1, index));
  }

  function indexFromValue(value, labels) {
    if (value === undefined || value === null) return null;

    var labelIndex = labels.indexOf(String(value));
    if (labelIndex !== -1) return labelIndex;

    var numericValue = Number(value);
    if (!Number.isFinite(numericValue)) return null;
    return clampIndex(Math.round(numericValue), labels);
  }

  function indexFromPercent(percent, labels) {
    var numericPercent = Number(percent);
    if (!Number.isFinite(numericPercent)) return null;

    return clampIndex(
      Math.round((numericPercent / 100) * (labels.length - 1)),
      labels,
    );
  }

  function selectedIndices(params, labels) {
    var zoom = params.batch && params.batch.length ? params.batch[0] : params;
    var start = indexFromValue(zoom.startValue, labels);
    var end = indexFromValue(zoom.endValue, labels);

    if (start === null) start = indexFromPercent(zoom.start, labels);
    if (end === null) end = indexFromPercent(zoom.end, labels);
    if (start === null || end === null) return null;

    return {
      start: Math.min(start, end),
      end: Math.max(start, end),
    };
  }

  function selectedRange(el, bucketRanges, indices) {
    if (indices.start === 0 && indices.end === bucketRanges.length - 1) {
      return null;
    }

    var start = new URLSearchParams(bucketRanges[indices.start] || "");
    var end = new URLSearchParams(bucketRanges[indices.end] || "");
    var from = start.get("from");
    var to = end.get("to");
    if (!from || !to) return null;

    var fromHour = start.get("from_hour");
    var toHour = end.get("to_hour");
    if (fromHour || toHour) {
      fromHour = fromHour || "0";
      toHour = toHour || "23";
    }

    var current = new URLSearchParams(
      el.getAttribute("data-range-query") || "",
    );
    if (
      from === current.get("from") &&
      to === current.get("to") &&
      fromHour === current.get("from_hour") &&
      toHour === current.get("to_hour")
    ) {
      return null;
    }

    return {
      from: from,
      to: to,
      fromHour: fromHour,
      toHour: toHour,
    };
  }

  function refreshStats(source, range) {
    if (typeof htmx === "undefined") return;

    var endpoint =
      window.location.pathname === "/referrer" ? "/hx/referrer" : "/hx/stats";
    var url = new URL(endpoint, window.location.origin);
    url.search = window.location.search;
    url.searchParams.set("from", range.from);
    url.searchParams.set("to", range.to);
    if (range.fromHour && range.toHour) {
      url.searchParams.set("from_hour", range.fromHour);
      url.searchParams.set("to_hour", range.toHour);
    } else {
      url.searchParams.delete("from_hour");
      url.searchParams.delete("to_hour");
    }

    htmx.ajax("GET", url.pathname + url.search, {
      source: source,
      target: "#stats-panel",
      swap: "outerHTML",
    });
  }

  function disposeChart(el) {
    var cursorTimer = cursorTimers.get(el);
    if (cursorTimer !== undefined) {
      clearTimeout(cursorTimer);
      cursorTimers.delete(el);
    }
    if (chartResizeObserver) chartResizeObserver.unobserve(el);

    var instance = echarts.getInstanceByDom(el);
    if (instance) instance.dispose();
  }

  function initChart(el) {
    if (!el || typeof echarts === "undefined") return;
    var labels = JSON.parse(el.getAttribute("data-labels") || "[]");
    var counts = JSON.parse(el.getAttribute("data-counts") || "[]");
    var bucketRanges = JSON.parse(
      el.getAttribute("data-bucket-ranges") || "[]",
    );
    if (
      !labels.length ||
      counts.length !== labels.length ||
      bucketRanges.length !== labels.length
    ) {
      return;
    }

    disposeChart(el);

    var isDark = document.documentElement.dataset.theme !== "light";
    var cs = getComputedStyle(document.documentElement);
    var primary = cs.getPropertyValue("--pico-primary").trim() || "#0172ad";
    var mutedColor =
      cs.getPropertyValue("--pico-muted-color").trim() || "#888888";
    var splitColor = isDark ? "rgba(255,255,255,0.07)" : "rgba(0,0,0,0.07)";
    var tooltipBg = isDark ? "#1e2328" : "#ffffff";
    var tooltipFg = isDark ? "#e0e0e0" : "#333333";

    var instance = echarts.init(el, null, { renderer: "canvas" });

    instance.setOption({
      backgroundColor: "transparent",
      animation: false,
      grid: { top: 10, right: 10, bottom: 28, left: 42 },

      // toolbox must be show:true and rendered (even if off-screen) so that
      // the dataZoom select tool is fully initialised for takeGlobalCursor
      toolbox: {
        show: true,
        top: -9999,
        feature: {
          dataZoom: { yAxisIndex: false },
        },
      },

      // inside handles scroll-to-zoom and pinch-to-zoom on mobile;
      // the visible drag-to-select-zoom is driven by takeGlobalCursor below
      dataZoom: [
        {
          type: "inside",
          xAxisIndex: 0,
          filterMode: "none",
          zoomOnMouseWheel: true,
          moveOnMouseMove: false,
          moveOnMouseWheel: false,
        },
      ],

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

      tooltip: {
        trigger: "axis",
        axisPointer: {
          type: "shadow",
          shadowStyle: {
            color: isDark ? "rgba(255,255,255,0.04)" : "rgba(0,0,0,0.04)",
          },
        },
        backgroundColor: tooltipBg,
        borderColor: splitColor,
        padding: [6, 10],
        textStyle: { color: tooltipFg, fontSize: 12 },
        formatter: function (p) {
          var v = p[0].value;
          return (
            '<span style="font-size:11px;color:' +
            mutedColor +
            '">' +
            p[0].name +
            "</span><br><strong>" +
            v +
            "</strong> visit" +
            (v === 1 ? "" : "s")
          );
        },
      },

      series: [
        {
          type: "bar",
          data: counts,
          itemStyle: { color: primary, borderRadius: [3, 3, 0, 0] },
          emphasis: {
            itemStyle: {
              color: primary,
              opacity: 1,
              shadowBlur: 8,
              shadowColor: primary + "66",
            },
          },
          barMaxWidth: 32,
          opacity: 0.88,
        },
      ],
    });

    instance.on("datazoom", function (params) {
      var indices = selectedIndices(params, labels);
      if (!indices) return;

      clearTimeout(zoomRequestTimer);
      zoomRequestTimer = setTimeout(function () {
        var range = selectedRange(el, bucketRanges, indices);
        if (range) refreshStats(el, range);
      }, ZOOM_REQUEST_DELAY_MS);
    });

    // activate drag-to-zoom selection as the permanent default interaction;
    // needs a tick so ECharts has fully rendered the (off-screen) toolbox
    var cursorTimer = setTimeout(function () {
      cursorTimers.delete(el);
      if (echarts.getInstanceByDom(el) !== instance) return;

      instance.dispatchAction({
        type: "takeGlobalCursor",
        key: "dataZoomSelect",
        dataZoomSelectActive: true,
      });
    }, 0);
    cursorTimers.set(el, cursorTimer);

    if (chartResizeObserver) chartResizeObserver.observe(el);
  }

  function forEachChartWithin(root, callback) {
    if (!root) return;
    if (root.matches && root.matches(CHART_SELECTOR)) callback(root);
    if (root.querySelectorAll) {
      root.querySelectorAll(CHART_SELECTOR).forEach(callback);
    }
  }

  function initChartsWithin(root) {
    forEachChartWithin(root, initChart);
  }

  function disposeChartsWithin(root) {
    if (typeof echarts === "undefined") return;
    forEachChartWithin(root, disposeChart);
  }

  function resizeAllCharts() {
    if (typeof echarts === "undefined") return;
    document.querySelectorAll(CHART_SELECTOR).forEach(function (el) {
      var instance = echarts.getInstanceByDom(el);
      if (instance) instance.resize();
    });
  }

  function initAll() {
    initChartsWithin(document);
  }

  if (!chartResizeObserver) window.addEventListener("resize", resizeAllCharts);

  document.addEventListener("DOMContentLoaded", initAll);
  document.addEventListener("htmx:afterSettle", function (event) {
    initChartsWithin(event.detail.elt);
  });
  document.addEventListener("htmx:beforeCleanupElement", function (event) {
    disposeChartsWithin(event.detail.elt);
  });
  document.addEventListener("themechange", initAll);
})();
