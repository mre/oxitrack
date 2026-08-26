const test = require("node:test");
const assert = require("node:assert/strict");

const {
  selectedIndices,
  selectedRange,
  selectedRangeUrl,
} = require("../static/chart.js");

const labels = ["alpha", "beta", "gamma", "delta", "epsilon"];

test("selectedIndices handles direct and batched ECharts payloads", () => {
  assert.deepEqual(
    selectedIndices({ startValue: "gamma", endValue: "beta" }, labels),
    { start: 1, end: 2 },
  );
  assert.deepEqual(
    selectedIndices({ batch: [{ startValue: 4, endValue: 1 }] }, labels),
    { start: 1, end: 4 },
  );
});

test("selectedIndices falls back to percentages and rejects incomplete payloads", () => {
  assert.deepEqual(selectedIndices({ start: 25, end: 75 }, labels), {
    start: 1,
    end: 3,
  });
  assert.equal(selectedIndices({ start: 25 }, labels), null);
});

test("selectedRange combines server-provided bucket boundaries", () => {
  const bucketRanges = [
    "from=2026-05-01&to=2026-05-31",
    "from=2026-06-01&to=2026-06-30",
    "from=2026-07-01&to=2026-07-18&from_hour=0&to_hour=16",
  ];

  assert.deepEqual(selectedRange("", bucketRanges, { start: 1, end: 2 }), {
    from: "2026-06-01",
    to: "2026-07-18",
    fromHour: "0",
    toHour: "16",
  });
});

test("selectedRange ignores full-range and unchanged selections", () => {
  const bucketRanges = [
    "from=2026-05-01&to=2026-05-31",
    "from=2026-06-01&to=2026-06-30",
    "from=2026-07-01&to=2026-07-31",
  ];

  assert.equal(
    selectedRange("", bucketRanges, { start: 0, end: 2 }),
    null,
  );
  assert.equal(
    selectedRange(
      "from=2026-06-01&to=2026-07-31",
      bucketRanges,
      { start: 1, end: 2 },
    ),
    null,
  );
});

test("selectedRangeUrl preserves server-owned filters and replaces the range", () => {
  assert.equal(
    selectedRangeUrl(
      "/hx/stats?path=%2Fdocs&view=referrers&from=2026-01-01&to=2026-01-31",
      {
        from: "2026-03-24",
        to: "2026-03-24",
        fromHour: "14",
        toHour: "16",
      },
    ),
    "/hx/stats?path=%2Fdocs&view=referrers&from=2026-03-24&to=2026-03-24&from_hour=14&to_hour=16",
  );
});

test("selectedRangeUrl removes stale hour bounds for a date-only range", () => {
  assert.equal(
    selectedRangeUrl(
      "/hx/referrer?domain=example.com&from_hour=4&to_hour=8",
      {
        from: "2026-03-01",
        to: "2026-03-31",
        fromHour: null,
        toHour: null,
      },
    ),
    "/hx/referrer?domain=example.com&from=2026-03-01&to=2026-03-31",
  );
});
