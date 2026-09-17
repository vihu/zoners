# zoners

One moment in time, read in every zone you care about. A CLI that prints a
psql-style table, and a small self-hosted web app that shows the same answer on
a 24 hour dial.

## Places

Both the CLI and the web app read the same list. One entry per line, or
separated by `;`: `<IANA zone> <label>`. The label is the rest of the entry.
Lines starting with `#` are comments.

```text
Indian/Mauritius Mauritius
Asia/Kolkata Delhi
America/New_York New York
```

Sources, first match wins:

1. The `ZONERS_ZONES` environment variable.
2. `$XDG_CONFIG_HOME/zoners/zones`, falling back to `~/.config/zoners/zones`.
3. Neither: UTC only, with a hint on stderr.

## CLI

```sh
cargo install --path . --locked
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
- `--zone`: a place label, an offset (`GMT+4`, `UTC+5:30`, `-04:00`) or an IANA
  name. Default: the system zone.

Green is work hours (09 to 17), yellow is awake (07 to 22), dim is night. Bold
is the source. Magenta marks a different date.

## Web app

```sh
cargo run --features web --bin zoners-web
```

Open <http://127.0.0.1:5051>. The URL carries the state, so any view is a
bookmark: `/?time=20:30&date=2026-09-22&zone=GMT%2B4&view=table`.

| Variable       | Default | Meaning                                         |
| -------------- | ------- | ----------------------------------------------- |
| `ZONERS_PORT`  | `5051`  | Port to listen on.                              |
| `ZONERS_ZONES` | unset   | The places, separated by `;`.                   |
| `TZ`           | `UTC`   | Source zone when the browser cannot supply one. |

There is no auth and no state. Put it behind your reverse proxy and auth of
choice.

### Deploy

```sh
cp .env.example .env   # then edit the places
docker compose up -d
```

The image is a single static binary in `scratch`. The time zone database is
compiled in, so new tz rules arrive with a new image (`cargo update -p
jiff-tzdb`).
