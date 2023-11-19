# Changelog

## 0.9.0

### BREAKING CHANGES

- Require a second parameter for the endpoint `/page-left` which is the amount of time spent on the page in seconds (see the first feature below). This is not a breaking change for you if you don't use a custom JS script. You should be using the `count.js` script.

### Features

- Only report the spent time on page when the page is not hidden. If a visitor opens the page and leaves the tab open in the background for a while, the time measurement will stop.
- Hide the table when its body is empty. This feature was removed when the tables were made responsive to time filter buttons. But it is back now.
- Add protection against spam that uses registration IDs. Registration IDs are no longer serial. Before that, the first visitor got the ID 0, the next ID was 1 and so on. An evil person could send random requests to the first couple of IDs which leads to incorrect counting. This is not that easy anymore since IDs are random now.
- The UTC offset is now displayed as `+/-HH:MM` instead of `+/-HH:MM:SS`.

### Fixes

- Fix bugs related to timezones.
- Fix error when using a time filter without visits inside the related time interval.

## 0.8.1

### Features

- Verify that a referrer domain exists before inserting it to prevent submitting random domains.
- Couple the referrers table to the time filter buttons in the stats page for a specific path. Just like in on the homepage with the visits table.

## 0.8.0

### BREAKING CHANGES

- There is no data directory anymore. Require the environment variable **`OXITRAFFIC_CONFIG_FILE`** to point to the config file. The environment variable `OXITRAFFIC_DATA_DIR` is deprecated. The **container image** expects the config file at the path **`/volumes/config.toml`**. You should mount the config file as a read only volume (a volume doesn't have to be a directory, it can be a file).
- Add the **config value `logs_dir`** where log files will be placed in. The default is `/var/log/oxitraffic`, but you can change it (you need to change it if OxiTraffic doesn't have write permission for that directory). For the **container image**, you should mount a volume at `/var/log/oxitraffic` if you want to persist the logs. You can delete the old log file `oxitraffic.log`.

### Features

- Don't log every new request.

## 0.7.0

### BREAKING CHANGES

- **If you already use OxiTraffic, you have to be at least on version 0.5 before updating to 0.7!** If you update from a version below 0.5, database issues will occur. Please update to 0.6.1 and then to 0.7. If you are using OxiTraffic for the first time or having a version >= 0.5, you can directly use 0.7.

### Features

- Add the API endpoint `/api/count?path=PATH` to get the visits count for a specific path.

### Fixes

- Fix a potential problem with PostgreSQL permissions on the table `pg_depend` while running migrations. This table is not touched anymore.

## 0.6.1

### Fixes

- Fix the logo on devices that are missing the used font.

## 0.6.0

### BREAKING CHANGES

- Include the spent time in seconds `spent_time_secs` instead of `left_at` in the API endpoint `/api/history?path=PATH`.

### Features

- Add a visits chart to the homepage which shows visits over time from all pages.
- Couple the visits table on the homepage to the datetime filters of the chart (e.g. The "Last 60 days" filter would show the visits in the table from the last 60 days).
- Add logo
- Show the configured global UTC offset in the footer.
- Add a small crab to the footer 🦀🥰

### Fixes

- Fix being off by one day with the time filters.

## 0.5.0

### BREAKING CHANGES

- **Remove the visits line chart** because it causes huge performance hits when the number of visits grows.
- Add a **filter to only show the last 60 days**. This filter is applied by default but one can choose the **"all time" filter** to display all visits over time, but these might be grouped into months or even years instead of days depending on the total number of days.
- Add a filter for the last 2 days which groups into hours.
- Change the return type of the API endpoint `/api/history?path=PATH` (see [README](README.md#json-api)).

### Features

- Show the average spent time as a combosition of minutes and seconds instead of only seconds if the amount of seconds is more than 60.
- Hide the referrer table if there are no referrers.
- Add percentage to the referrer table.

### Fixes

- Fix the average spent time by considering the minimum delay.
- Fix the chart visits stepsize (no more floats on the y-axis).

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

### BREAKING CHANGES

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

### BREAKING CHANGES

- Rename `DATA_DIR_OXITRAFFIC` to `OXITRAFFIC_DATA_DIR` to be consistent with other environment variables.
- Remove the `/dashboard` nesting.

### Features

- Show more stats.
- Improve the base template.
- Make the tab title clearer by removing "OxiTraffic | ".
- Exclude more files from the published crate.
