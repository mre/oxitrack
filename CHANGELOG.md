# Changelog

## 0.3.9

### Features

- Replace axis labels with a title to save space on small screens.
- Improve chart rendering performance by disabling parsing.

### Fixes

- Reduce the height of charts since we no longer have only one chart.

## 0.3.8

### Features

- Adjust padding on the stats page.
- Longer bar chart animation.

### Fixes

- Remove zoom and pan from the line chart because of scrolling issues on touch screens (see [this issue](https://github.com/chartjs/chartjs-plugin-zoom/issues/766)).

## 0.3.7

### Features

- Add bar chart of visits per day.
- Small improvements of the stats page like wider charts.

## 0.3.6

### Features

- Small chart optimizations

## 0.3.5

### Features

- Use `chartjs`

## 0.3.4

### Features

- Add the crate version to the CSS file request to prevent outdated cache.
- Add the crate version to the footer.
- Add more tests.

## 0.3.3

### Features

- Add tests.

### Fixes

- Fix a memory bug in unsafe code :/

## 0.3.2

### Features

- Add a link to the tracked website to the dashboard.
- Improve the history plot for smartphones.
- Add the page link to the stats.
- Less padding along the x-axis on large screens.

## 0.3.1

### Features

- Rename table `calls` to `visits` since we don't count each "call" anymore, only visits with a minimum delay.
- Round visits per day to two digits.

### Fixes

- Empty history is not an internal error.
- Make `path` in the `paths` table unique.
- Prevent showing paths in the dashboard with no counted visits.
- Handle a possible race condition on path insertion.

## 0.3.0

### Breaking

- Rename `DATA_DIR_OXITRAFFIC` to `OXITRAFFIC_DATA_DIR` to be consistent with other environment variables.
- Remove the `/dashboard` nesting.

### Features

- Show more stats.
- Improve the base template.
- Make the tab title clearer by removing "OxiTraffic | ".
- Exclude more files from the published crate.
