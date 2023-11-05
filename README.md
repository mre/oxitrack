<p align="center">
  <img height="100" src="https://codeberg.org/mo8it/oxitraffic/raw/branch/main/static/logo.svg" />
</p>
<h1 align="center">OxiTraffic</h1>
<p align="center">Self-hosted, simple and privacy respecting website traffic tracker 🌐</p>
<h2 align="center">➡️ <a href="https://oxitraffic.mo8it.com">Demo<a/> ⬅️</h2>

## Features

- Short visits are not counted ❌
  - Only meaningful visits are counted ✅
  - Makes it less likely to count visits by web bots 🤖
- Respects privacy (no personal data or IP is logged) 🥷🏼
- Self-hosted 🕊️
- Visualization of visits history 📈
- API for visits history and count 💻️
- Low memory usage (about 12 MB) 🏅
- First class container support 📦️
- Asynchronous and multithreaded 🔀
- Informative tracing (logging) to stdout and to a log file 📜
- Free & open source (AGPLv3) 🆓
- Written in Rust (**oxi**dized) 🦀

## Demo

[Here is a demo](https://oxitraffic.mo8it.com) which tracks my own website (mo8it.com).

## How it works

You add the following script tag to your website after replacing `OXITRAFFIC_BASE_URL` with the base URL of your OxiTraffic instance:

```html
<script type="module" src="https://OXITRAFFIC_BASE_URL/count.js"></script>
```

It runs the tiny script [`count.js`](templates/count.js).

The script calls `/register?path=PATH` to receive a visitor ID.
`PATH` is the path of the page you are on.

This ID is used after the minimum delay (configuration option `min_delay_secs`) to call `/post-sleep/VISITOR_ID` which leads to counting that visit.

When the page is left, a request is sent to `/page-left/VISITOR_ID` to record the total spent time.

### Path validation

How does OxiTraffic know if a newly requested path is a valid one for your tracked website?

Only for the first request to a new path, OxiTraffic sends a request to that path prefixed by the configuration option `tracked_origin_callback`.
If the status code is in the range 200-299 (success), the path is added to the database.
Otherwise, the request is rejected.

## Setup

### Data directory

The binary expects the environment variable `OXITRAFFIC_DATA_DIR` to point to a directory that stores the TOML configuration file `config.toml`.

The log file `oxitraffic.log` will be also placed in that directory.

## Hosting

### Containerized

You can use the container image published on [Docker Hub](https://hub.docker.com/r/mo8it/oxitraffic).

You can pull that image using Docker:

```fish
docker pull mo8it/oxitraffic:latest
```

Or using Podman:

```fish
podman pull docker.io/mo8it/oxitraffic:latest
```

The container expects the data directory to be mounted as a **volume** at `/volumes/data` inside the container.

By default, the container listens on **port** `80`.

### Not containerized

You can also host OxiTraffic directly with the binary that you can install with Cargo:

```fish
cargo install oxitraffic --locked
```

Make sure to provide the environment variable `OXITRAFFIC_DATA_DIR` when using the binary directly.

### Database

In both cases (container or binary), you need a PostgreSQL database.
There are many guides in the internet that explain how to host one either in a container or directly on the host.
You could use [my blog post about hosting PostgreSQL using Podman](https://mo8it.com/blog/containerized-postgresql-with-rootless-podman/).

### Configuration

| Parameter                 | Description                                                                                                                                                                                                                                                                                       | Default          |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------- |
| `socket_address`          | Use `127.0.0.1:8080` for local testing. `0.0.0.0` is important for usage in a container, but you can pick another port.                                                                                                                                                                           | `"0.0.0.0:80"`   |
| `base_url`                | The base URL of your OxiTraffic instance. Used to build the [`count.js`](templates/count.js) script.                                                                                                                                                                                              |                  |
| `tracked_origin`          | The [origin](https://developer.mozilla.org/en-US/docs/Glossary/Origin) of your tracked website that is used to allow [CORS-requests](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Access-Control-Allow-Origin) from the [`count.js`](templates/count.js) script to OxiTraffic.       |                  |
| `tracked_origin_callback` | The [origin](https://developer.mozilla.org/en-US/docs/Glossary/Origin) of your tracked website that is used to verify a newly requested path as explained above. This option exists to be able to make these requests inside a local network.                                                     | `tracked_origin` |
| `min_delay_secs`          | Minimum delay in seconds between visiting the website and being able to call `/post-sleep` to count the visit. It is recommended to call `/post-sleep` one second after this value. A low value not only counts meaningless visits, but also makes it easier for visits by web bots to be counts. | 19               |
| `db.host`                 | PostgreSQL host                                                                                                                                                                                                                                                                                   |                  |
| `db.port`                 | PostgreSQL port                                                                                                                                                                                                                                                                                   |                  |
| `db.username`             | PostgreSQL username                                                                                                                                                                                                                                                                               |                  |
| `db.password`             | PostgreSQL password                                                                                                                                                                                                                                                                               |                  |
| `db.database`             | PostgreSQL database                                                                                                                                                                                                                                                                               |                  |
| `utc_offset.hours`        | The hours of your UTC offset                                                                                                                                                                                                                                                                      | 0                |
| `utc_offset.minutes`      | The minutes of your UTC offset                                                                                                                                                                                                                                                                    | 0                |

#### Example configuration

This is an example of the configuration file `config.toml`:

```toml
# Can be omitted because this is the default value.
socket_address = "0.0.0.0:80"

base_url = "https://oxitraffic.your_domain.com"

tracked_origin = "https://your_domain.com"
# In case both OxiTraffic and your website are in a local network and `website` can be resolved to the local IP address of the your website.
# Omit this option to use the value of `tracked_origin` instead.
tracked_origin_callback = "http://website"

[db]
host = "127.0.0.1"
port = 5432
username = "postgres"
password = "CHANGE_ME"
database = "postgres"

[utc_offset]
hours = 2
# Can be omitted because 0 is the default.
minutes = 0
```

## Endpoints

### Dashboard

| Endpoint           | Description                                             | Return |
| ------------------ | ------------------------------------------------------- | ------ |
| `/`                | A list of registered paths to see their visits history. | HTML   |
| `/stats?path=PATH` | Statistics of the visits history of a specific path.    | HTML   |

### JSON API

| Endpoint                 | Description                                                                                                                                              | Return                                                                                                                               |
| ------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| `/api/counts`            | The visits count for each registered path                                                                                                                | `JSON([{"path": String, "count": i64}])`                                                                                             |
| `/api/count?path=PATH`   | The visits count for the specified path                                                                                                                  | `JSON(i64)`                                                                                                                          |
| `/api/history?path=PATH` | The visits datetimes for a specific path with the nullable referrer and global UTC offset. You can use this endpoint to make your own analysis and plots | `JSON({"utc_offset": String, "visits": [{"registered_at": String, "referrer": Option<String>, "spent_time_seconds": Option<i64>}]})` |

### Script

| Endpoint                 | Description                                                                                                         | Return                          |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------- | ------------------------------- |
| `/register?path=PATH`    | Register to receive a `VISITOR_ID` for the `PATH` (e.g. `/` or `/blog/rust-vs-julia`) of the page you are visiting. | `JSON(u16)`                     |
| `/post-sleep/VISITOR_ID` | Use the visitor ID after the minimum delay `min_delay_secs` for the visit to be counted.                            | Only status code 200 on success |
| `/page-left/VISITOR_ID`  | Use the visitor ID on leaving the page to record the total spent time.                                              | Only status code 200 on success |

## Questions?

Don't hesitate to open an issue ^^

## Contributing

You are welcome to contribute to the project!

You can always open an issue.
**Wait** for a response on the issue before starting with a pull request (Rejected pull request are very disappointing).

Use Clippy and rustfmt before submitting code :)
