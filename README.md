# zoners

[![CI](https://github.com/vihu/zoners/actions/workflows/ci.yml/badge.svg)](https://github.com/vihu/zoners/actions/workflows/ci.yml)
[![Release](https://github.com/vihu/zoners/actions/workflows/release.yml/badge.svg)](https://github.com/vihu/zoners/actions/workflows/release.yml)

One moment in time, read in every zone you care about. A CLI that prints a psql-style table, and a small self-hosted web app that shows the same answer on a live 24 hour dial.

## Run

```sh
curl -fsSLO "https://raw.githubusercontent.com/vihu/zoners/main/{docker-compose.yml,env.example}"
cp env.example .env   # set ZONERS_ZONES to your places
docker compose up -d
open http://localhost:5051
```

The image is `ghcr.io/vihu/zoners`. A release `v1.2.3` publishes the tags `1.2.3`, `1.2`, `1` and `latest`.
It is a single static binary in `scratch`: templates, assets, fonts and the time zone database are compiled in, so new tz rules arrive with a new image.

There is no auth, no state and no volume. The URL carries everything, so any view is a bookmark:
`/?time=20:30&date=2026-09-22&zone=GMT%2B4&view=table`. With no `time` and no `date` the page follows the clock.

Native: `cargo run --features web --bin zoners-web`. It does not read `.env`, so export the variables yourself.

## Configuration

| Variable       | Required | Default | Notes                                                           |
| -------------- | -------- | ------- | --------------------------------------------------------------- |
| `ZONERS_ZONES` | no       | unset   | The places, separated by `;`. See [Places](#places)             |
| `ZONERS_PORT`  | no       | `5051`  | Listen port. Compose publishes it on 127.0.0.1                  |
| `TZ`           | no       | `UTC`   | Source zone when the browser cannot supply one (no JavaScript)  |

## Places

The CLI and the web app read the same list. One entry per line, or separated by `;`: `<IANA zone> <label>`.
The label is the rest of the entry. Lines starting with `#` are comments.

```text
Indian/Mauritius Mauritius
Asia/Kolkata Delhi
America/New_York New York
```

Sources, first match wins:

1. The `ZONERS_ZONES` environment variable.
2. `$XDG_CONFIG_HOME/zoners/zones`, falling back to `~/.config/zoners/zones`.
3. Neither: UTC only, with a hint on stderr.

## Reverse proxy

Terminate TLS and do auth in front. Compose only publishes on 127.0.0.1.

```caddyfile
zones.example.com {
    reverse_proxy 127.0.0.1:5051
}
```

Every URL in the page is relative, so serving it under a sub-path works too.

## CLI

```sh
cargo install --locked --git https://github.com/vihu/zoners
zoners --time 8.30 pm --date 2026-09-22 --zone GMT+4
```

```text
    PLACE    | TIME  |    DAY     | OFFSET
-------------+-------+------------+--------
 GMT+4       | 20:30 | Tue 22 Sep | +04:00
 New York    | 12:30 | Tue 22 Sep | -04:00
 Tokyo       | 01:30 | Wed 23 Sep | +09:00
```

- `--time`: `20:30`, `8.30pm`, `8 pm`. Default: now.
- `--date`: `YYYY-MM-DD`. Default: today in the source zone.
- `--zone`: a place label, an offset (`GMT+4`, `UTC+5:30`, `-04:00`) or an IANA name. Default: the system zone.

Green is work hours (09 to 17), yellow is awake (07 to 22), dim is night. Bold is the source. Magenta marks a different date.
The install builds the CLI only. The web app sits behind the `web` feature.

## Non-goals

- Auth, users, sessions. Put it behind your own.
- A database or a zone editor. The places are configuration.
- A JSON API.
- A Tailwind or Node.js build. The CSS is vendored, see `assets/README.md`.
- arm64 images, until someone needs one.

## Check

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-features
```

## Release

Bump `version` in `Cargo.toml`, commit, then tag and push. The Release workflow builds and publishes the image.

```sh
git tag v0.1.0 && git push origin v0.1.0
```

## License

AGPL-3.0-only. See `LICENSE`. The bundled fonts are under the OFL, see `assets/fonts/OFL.txt`.
