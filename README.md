# OxiTraffic

Self-hosted, simple and privacy respecting website traffic tracker 🌐

## Features

- Short visits are not counted ❌
  - Only meaningful visits are counted ✅
  - Makes it less likely to count visits by web bots 🤖
- Respects privacy (no personal data or IP is logged) 🥷🏼
- Self-hosted 🕊️
- Visualization of call history 📈
- API for visits history and count 💻️
- Low memory usage (about 12 MB) 🏅
- First class container support 📦️
- Asynchronous and multithreaded 🔀
- Informative tracing (logging) to stdout and to a log file 📜
- YAML configuration 🛠️
- Free & open source (AGPLv3) 🆓
- Written in Rust (**oxi**dized) 🦀

## Demonstration

My website [mo8it.com](https://mo8it.com) is an example website that uses OxiTraffic.
You can visit the OxiTraffic [dashboard](https://oxitraffic.mo8it.com) to see the call history of each page on the website.
Here is an [example](https://oxitraffic.mo8it.com/stats?path=/blog/rust-vs-julia) for a specific blog post.

Try out the following API endpoints (with `curl` for example):

- `https://oxitraffic.mo8it.com/api/counts`
- `https://oxitraffic.mo8it.com/api/history?path=/blog/rust-vs-julia`

## How it works

You add the following script tag to your website after replacing `OXITRAFFIC_BASE_URL` with the base URL of your OxiTraffic instance:

```html
<script type="module" src="https://OXITRAFFIC_BASE_URL/count.js"></script>
```

It runs the tiny script [count.js](templates/count.js).

The script calls `/register?path=PATH` to receive a registration ID.
`PATH` is the path of the page you are on.

This ID is used after the minimum delay (configuration option `min_delay_secs`) to call `/post-sleep/REGISTRATION_ID` which leads to counting that visit.

How does OxiTraffic know if a newly requested path is a valid one for your tracked website?

Only for the first request to a new path, OxiTraffic sends a request to that path prefixed by the configuration option `tracked_origin_callback`.
If the status code is in the range 200-299 (success), the path is added to the database.
Otherwise, the request is rejected.

## Setup

### Data directory

The binary expects the environment variable `OXITRAFFIC_DATA_DIR` to point to a directory that stores the YAML configuration file `config.yaml`.

The log file `oxitraffic.log` will be also placed in that directory.

## Hosting

If you want to host OxiTraffic in a container, check [`Containerfile`](Containerfile) and [`compose.yaml`](compose.yaml) as a starting point.
The container expects the data directory to be mounted as a **volume** at `/volumes/data`.

You can also host OxiTraffic directly with the binary that you can install with Cargo:

```fish
cargo install oxitraffic --locked
```

Make sure to provide the environment variable `OXITRAFFIC_DATA_DIR` when using the binary directly.

In both cases (container or binary), you need a PostgreSQL database.
There are many guides in the internet that explain how to host one either in a container or directly on the host.
You could use [my blog post about hosting PostgreSQL using Podman](https://mo8it.com/blog/containerized-postgresql-with-rootless-podman/).

### Configuration

| Parameter                 | Description                                                                                                                                                                                                                                                                                       | Default          |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------- |
| `socket_address`          | Use `127.0.0.1:8080` for local testing. `0.0.0.0` is important for usage in a container, but you can pick another port.                                                                                                                                                                           | `"0.0.0.0:80"`   |
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

```yaml
# Can be omitted because this is the default.
socket_address: 0.0.0.0:80

tracked_origin: https://mo8it.com

# In case both OxiTraffic and your website are in a local network and `website` can be resolved to the local IP address of the your website.
# Omit this option to use `tracked_origin` instead.
tracked_origin_callback: http://website

db:
  host: 127.0.0.1
  port: 5432
  username: postgres
  password: CHANGE_ME
  database: postgres

utc_offset:
  hours: 2
  # Can be omitted because 0 is the default.
  minutes: 0
```

## Endpoints

OxiTraffic has the following endpoints:

- `/register?path=PATH`: Register to receive a `REGISTRATION_ID` for the `PATH` (e.g. `/` or `/blog/rust-vs-julia`) of the page you are visiting.
- `/post-sleep/REGISTRATION_ID`: Use the registration ID after the minimum delay `min_delay_secs` for the visit to be counted.
- `/`: A list of registered paths to see their call history.
- `/stats?path=PATH`: Statistics of the call history of a specific path.
- `/api/counts`: JSON with the call count for each registered path.
- `/api/history?path=PATH`: JSON with the call datetimes for a specific path. You can use it to make your own analysis and plots.

## Questions?

Don't hesitate to open an issue ^^

## Contributing

You are welcome to contribute to the project!

You can always open an issue.
**Wait** for a response on the issue before starting with a pull request (Rejected pull request are very disappointing).

Use Clippy and rustfmt before submitting code :)
