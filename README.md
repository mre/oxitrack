# THE README HAS CONFLICTS WITH THE MAIN BRANCH AND WILL BE UPDATED SOON

# OxiTraffic

Self-hosted, simple and privacy respecting website traffic tracker 🌐

## Demonstration

My website [mo8it.com](https://mo8it.com) is an example website that uses OxiTraffic.
Visit the website and look for the image of Ferris (happy crab 🦀) on the bottom.
Click on it to see a plot of the call history for that page.
Each page on the website has its own call history.

You can visit the dashboard: [oxitraffic.mo8it.com/dashboard](https://oxitraffic.mo8it.com/dashboard)

Try out the following API endpoints (with `curl` for example):

- `https://oxitraffic.mo8it.com/api/counts`
- `https://oxitraffic.mo8it.com/api/history?path=blog`

## Features

- Protection against spam ❌
- Visualization of call history 📈
- API for call history and count 🤖
- Respects privacy (no personal data or IP is logged) 🥷🏼
- Self-hosted 🕊️
- Low memory usage (about 8 MB) 🏅
- First class container support 📦️
- Asynchronous and multithreaded 🔀
- Informative tracing (logging) to stdout and to a log file 📜
- YAML configuration 🛠️
- Free & open source (AGPLv3) 🆓
- Written in Rust (**oxi**dized) 🦀

## How it works

TODO: Update README

How does OxiTraffic know if a newly requested path is a valid one for your tracked website?

Only for the first request to a new path, OxiTraffic sends a request to that path prefixed by the configuration option `tracked_base_url` (TRACKED_BASE_URL/PATH).
If the status code is 200 (OK), the path is added to the database.
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
cargo install oxitraffic
```

Make sure to provide the environment variable `OXITRAFFIC_DATA_DIR` when using the binary directly.

In both cases (container or binary), you need a PostgreSQL database.
There are many guides in the internet that explain how to host one either in a container or directly on the host.
You could use [my blog post about hosting PostgreSQL using Podman](https://mo8it.com/blog/containerized-postgresql-with-rootless-podman/).

### Configuration

| Parameter          | Description                                                                                                                                  | Default      |
| ------------------ | -------------------------------------------------------------------------------------------------------------------------------------------- | ------------ |
| socket_address     | Use `127.0.0.1:8080` for testing on `http://localhost:8080`. `0.0.0.0` is important for usage in a container, but you can pick another port. | "0.0.0.0:80" |
| tracked_base_url   | The base URL of your tracked website                                                                                                         |              |
| db.host            | PostgreSQL host                                                                                                                              |              |
| db.port            | PostgreSQL port                                                                                                                              |              |
| db.username        | PostgreSQL username                                                                                                                          |              |
| db.password        | PostgreSQL password                                                                                                                          |              |
| db.database        | PostgreSQL database                                                                                                                          |              |
| utc_offset.hours   | The hours of your UTC offset                                                                                                                 | 0            |
| utc_offset.minutes | The minutes of your UTC offset                                                                                                               | 0            |

#### Example configuration

```yaml
socket_address: 0.0.0.0:80
tracked_base_url: https://mo8it.com

db:
  host: oxitraffic-db
  port: 5432
  username: postgres
  password: CHANGE_ME
  database: postgres

utc_offset:
  hours: 2
  minutes: 0
```

## Usage

OxiTraffic has the following endpoints:

- `/register?path=PATH`: TODO
- `/post-sleep/REGISTRATION_ID`: TODO
- `/dashboard`: A list of registered paths to plot their call history.
- `/dashboard/plot?path=PATH`: Plot of the call history of a specific path.
- `/api/counts`: JSON with the call count for each registered path.
- `/api/history?path=PATH`: JSON with the call datetimes for a specific path. You can use it to make your own analysis and plots.

## Questions?

Don't hesitate to open an issue ^^

## Contributing

You are welcome to contribute to the project!

You can always open an issue.
**Wait** for a response on the issue before starting with a pull request (Rejected pull request are very disappointing).

Use Clippy and rustfmt before submitting code :)
