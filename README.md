# OxyTrack

Self-hosted, privacy-respecting website traffic tracker.

## Features

- No personal data or IP addresses are logged
- No cookies
- Visits history with charts
- JSON API for visit history and counts
- Low memory usage (~12 MiB)
- First-class container support
- Only meaningful visits are counted; short visits and bot traffic are filtered out

## How it works

Add the following script tag to your website, replacing `OXYTRACK_BASE_URL` with the base URL of your OxyTrack instance:

```html
<script type="module" src="https://OXYTRACK_BASE_URL/count.js"></script>
```

The script calls `/register?path=PATH` to receive a visitor ID. After the minimum delay (`min_delay_secs`), it calls `/post-sleep/VISITOR_ID` to count the visit. When the visitor leaves the page, `/page-left/VISITOR_ID` is called to record the time spent.

### Path validation

On the first request to a new path, OxyTrack sends a request to that path prefixed by `tracked_origin_callback`. If the response status is 2xx, the path is accepted. Otherwise it is rejected.

### Binary

```sh
cargo install oxytrack --locked
```

Set the `OXYTRACK_CONFIG_FILE` environment variable to point to your config file (defaults to `config.toml` in the current directory).

## Configuration

The config file path is read from `OXYTRACK_CONFIG_FILE` (defaults to `config.toml`). Every option can also be set or overridden via environment variable.

| Parameter                 | Description                                                                                                        | Default          | Environment variable               |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------ | ---------------- | ---------------------------------- |
| `socket_address`          | Listening address. Use `127.0.0.1:8080` for local testing; `0.0.0.0:80` for containers.                          | `"0.0.0.0:80"`   | `OXYTRACK_SOCKET_ADDRESS`          |
| `base_url`                | Base URL of your OxyTrack instance. Used to build `count.js`.                                                     |                  | `OXYTRACK_BASE_URL`                |
| `tracked_origin`          | [Origin](https://developer.mozilla.org/en-US/docs/Glossary/Origin) of your tracked website (used for CORS).      |                  | `OXYTRACK_TRACKED_ORIGIN`          |
| `tracked_origin_callback` | Origin used for path validation requests. Useful when OxyTrack and your site are on the same local network.       | `tracked_origin` | `OXYTRACK_TRACKED_ORIGIN_CALLBACK` |
| `min_delay_secs`          | Seconds a visitor must spend on the page before the visit is counted.                                             | `19`             | `OXYTRACK_MIN_DELAY_SECS`          |
| `utc_offset.hours`        | Hours component of your UTC offset                                                                                 | `0`              | `OXYTRACK_UTC_OFFSET__HOURS`       |
| `utc_offset.minutes`      | Minutes component of your UTC offset                                                                               | `0`              | `OXYTRACK_UTC_OFFSET__MINUTES`     |

### Example `config.toml`

```toml
socket_address = "0.0.0.0:80"

base_url = "https://oxytrack.your_domain.com"
tracked_origin = "https://your_domain.com"
# Optional: use a local address for path validation
tracked_origin_callback = "http://website"

[utc_offset]
hours = 2
```

### Logging

OxyTrack logs to stdout at the `info` level by default. Set `RUST_LOG` to one of `off`, `error`, `warn`, `info`, `debug`, or `trace` to change the level.

## API

### Dashboard

| Endpoint           | Description                                          |
| ------------------ | ---------------------------------------------------- |
| `/`                | List of registered paths with visit counts           |
| `/stats?path=PATH` | Visit history and statistics for a specific path     |

### JSON API

| Endpoint                 | Description                              | Response                                                                                                                             |
| ------------------------ | ---------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| `/api/counts`            | Visit count for each registered path     | `[{"path": String, "count": i64}]`                                                                                                   |
| `/api/count?path=PATH`   | Visit count for a specific path          | `i64`                                                                                                                                |
| `/api/history?path=PATH` | Full visit history for a specific path   | `{"utc_offset": String, "visits": [{"registered_at": String, "referrer": Option<String>, "spent_time_seconds": Option<i64>}]}` |

### Tracking script endpoints

| Endpoint                                 | Description                                                          |
| ---------------------------------------- | -------------------------------------------------------------------- |
| `/register?path=PATH`                    | Register a visitor for the given path; returns a `VISITOR_ID`        |
| `/post-sleep/VISITOR_ID`                 | Count the visit after the minimum delay has elapsed                  |
| `/page-left/VISITOR_ID/TIME_ON_PAGE_SEC` | Record the total time spent on the page when the visitor leaves      |

## Limitations

**Concurrent visitors:** Counting may behave unexpectedly above 65,536 concurrent visitors, as visitor IDs are 16-bit and recycled periodically. This is unlikely to matter for a self-hosted single-site deployment.

**Single instance:** OxyTrack keeps an in-memory cache of pending visits for performance. Running multiple instances against the same database is not supported. If you need horizontal scaling, open an issue.
