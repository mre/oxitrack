(function () {
    function initChart(el) {
        if (!el || typeof echarts === 'undefined') return;
        var labels = JSON.parse(el.getAttribute('data-labels') || '[]');
        var counts = JSON.parse(el.getAttribute('data-counts') || '[]');
        if (!labels.length) return;

        var existing = echarts.getInstanceByDom(el);
        if (existing) existing.dispose();

        var isDark = document.documentElement.dataset.theme !== 'light';
        var cs = getComputedStyle(document.documentElement);
        var primary    = cs.getPropertyValue('--pico-primary').trim()     || '#0172ad';
        var mutedColor = cs.getPropertyValue('--pico-muted-color').trim() || '#888888';
        var splitColor = isDark ? 'rgba(255,255,255,0.07)' : 'rgba(0,0,0,0.07)';
        var tooltipBg  = isDark ? '#1e2328' : '#ffffff';
        var tooltipFg  = isDark ? '#e0e0e0' : '#333333';

        var instance = echarts.init(el, null, { renderer: 'canvas' });
        instance.setOption({
            backgroundColor: 'transparent',
            animation: false,
            grid: { top: 10, right: 10, bottom: 32, left: 42 },
            xAxis: {
                type: 'category',
                data: labels,
                axisLine: { show: false },
                axisTick: { show: false },
                axisLabel: {
                    color: mutedColor,
                    fontSize: 10,
                    interval: Math.max(0, Math.floor(labels.length / 10) - 1),
                    rotate: labels.length > 40 ? 35 : 0,
                    hideOverlap: true,
                },
                splitLine: { show: false },
            },
            yAxis: {
                type: 'value',
                minInterval: 1,
                axisLabel:  { color: mutedColor, fontSize: 10 },
                splitLine:  { lineStyle: { color: splitColor } },
                axisLine:   { show: false },
                axisTick:   { show: false },
            },
            tooltip: {
                trigger: 'axis',
                axisPointer: { type: 'shadow', shadowStyle: { color: 'rgba(150,150,150,0.08)' } },
                backgroundColor: tooltipBg,
                borderColor: splitColor,
                padding: [6, 10],
                textStyle: { color: tooltipFg, fontSize: 12 },
                formatter: function (p) {
                    return '<span style="color:' + mutedColor + ';font-size:11px">' + p[0].name + '</span><br>'
                         + '<strong>' + p[0].value + '</strong> visit' + (p[0].value === 1 ? '' : 's');
                },
            },
            series: [{
                type: 'bar',
                data: counts,
                itemStyle: { color: primary, borderRadius: [3, 3, 0, 0] },
                emphasis:  { itemStyle: { opacity: 1 } },
                barMaxWidth: 28,
                opacity: 0.88,
            }],
        });

        window.addEventListener('resize', function () { instance.resize(); });
    }

    function initAll() {
        document.querySelectorAll('.visits-chart[data-labels]').forEach(initChart);
    }

    document.addEventListener('DOMContentLoaded', initAll);
    document.addEventListener('htmx:afterSwap', initAll);
})();
