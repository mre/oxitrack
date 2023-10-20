# Changelog

## 0.5.0

### Breaking

- **Remove the visits line chart** because it causes huge performance hits when the number of visits grows.
- Add a **filter to only show the last 60 days**. This filter is applied by default but one can choose the **"all time" filter** to display all visits over time, but these might be grouped into months or even years instead of days depending on the total number of days.
- Add a filter for the last 2 days which groups into hours.

### Features

- Show the average spent time as a combosition of minutes and seconds instead of only seconds if the amount of seconds is more than 60.
- Hide the referrer table if there are no referrers.

### Fixes

- Fix the average spent time by considering the minimum delay.

## 0.4.8

### Features

- Record the total time spent on a page and show the average.

### Fixes

- Encode the path sent for registration.

## 0.4.7

### Features

- Improve the performance of serving static files.
- Compress static files.
- Reduce binary size.

## 0.4.6

### Fixes

- Encode the sent referrer origin in the query parameters.

## 0.4.0 - 0.4.5

### Breaking

- **The configuration format changed from YAML to [TOML](https://toml.io)**. YAML is better than TOML for deep nestings and lists. But these are not used in the configuration and TOML is simpler and less error prone. For the migration, rename your `config.yaml` file to `config.toml` and adjust the content with inspiration from the configuration example in the [`README`](README).
- A new configuration value `base_url` has to be added to the `config.toml` file. You have to set it to the base URL of your OxiTraffic instance.
- Replace the snippet `oxitraffic.js` with the [`count.js`](templates/count.js) script. You just have to add this script tag to your website now (after replacing `OXITRAFFIC_BASE_URL` with the base URL of your OxiTraffic instance):
  ```html
  <script type="module" src="https://OXITRAFFIC_BASE_URL/count.js"></script>
  ```

### Features

- Show referrer domains on the stats page.
- Improve the quality of the frontend code by using TypeScript :)

### Fixes

- Fix counting first timestamp twice in the bar chart.

## 0.3.10

### Fixes

- Fix bar chart on small screens.

## 0.3.9

### Features

- Replace axis labels with a title to save space on small screens.
- Rank pages on their number of visits and show their percentage of total site visits.

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
