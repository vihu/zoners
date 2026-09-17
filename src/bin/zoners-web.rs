//! The `zoners-web` server: the same conversion, shown on a 24 hour dial.

use askama::Template;
use axum::{
    extract::{Path, Query as UrlQuery, State},
    http::{header, HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use jiff::tz::TimeZone;
use serde::Deserialize;
use std::{
    env,
    hash::{DefaultHasher, Hash, Hasher},
    sync::Arc,
};
use tokio::{
    net::TcpListener,
    signal::{
        ctrl_c,
        unix::{signal, SignalKind},
    },
};
use zoners::{convert, load_places, Band, Error, Place, Query, Row};

const DEFAULT_PORT: u16 = 5051;
const PORT_VAR: &str = "ZONERS_PORT";
const FALLBACK_ZONE: &str = "UTC";
const UNKNOWN_ZONE: &str = "Etc/Unknown";
const IMMUTABLE: &str = "public, max-age=31536000, immutable";
const DAY_FORMAT: &str = "%a %d %b";
const MINUTES_PER_DAY: f64 = 1440.0;
/// The dial turns a quarter degree per minute, with noon at the top.
const MINUTES_PER_DEGREE: f64 = 4.0;
const NOON_DEGREES: f64 = 180.0;

const CSS: &str = "text/css; charset=utf-8";
const JS: &str = "text/javascript; charset=utf-8";
const WOFF2: &str = "font/woff2";

macro_rules! asset {
    ($path:literal, $mime:expr) => {
        (
            $path,
            $mime,
            include_bytes!(concat!("../../assets/", $path)).as_slice(),
        )
    };
}

/// Every file the page loads, compiled into the binary.
const ASSETS: &[(&str, &str, &[u8])] = &[
    asset!("daisyui.css", CSS),
    asset!("app.css", CSS),
    asset!("app.js", JS),
    asset!("htmx.min.js", JS),
    asset!("favicon.svg", "image/svg+xml"),
    asset!("fonts/AtkinsonHyperlegibleNext-Regular.woff2", WOFF2),
    asset!("fonts/AtkinsonHyperlegibleNext-SemiBold.woff2", WOFF2),
    asset!("fonts/AtkinsonHyperlegibleNext-Bold.woff2", WOFF2),
    asset!("fonts/AtkinsonHyperlegibleMono-Regular.woff2", WOFF2),
    asset!("fonts/AtkinsonHyperlegibleMono-Medium.woff2", WOFF2),
];

/// State shared by every request. Nothing in it changes after startup.
struct App {
    places: Vec<Place>,
    zone_names: Vec<String>,
    asset_version: String,
}

/// The query string. Empty values count as absent, as HTML forms send them.
#[derive(Default, Deserialize)]
struct Params {
    time: Option<String>,
    date: Option<String>,
    zone: Option<String>,
    view: Option<String>,
}

#[derive(Template)]
#[template(path = "index.html")]
struct Page<'a> {
    asset_version: &'a str,
    zone_names: &'a [String],
    time: String,
    date: String,
    zone: String,
    /// Name of the field the error belongs to, empty when there is none.
    invalid: &'static str,
    error: Option<String>,
    dial: Option<Dial>,
    table_view: bool,
    /// No time or date was asked for, so the page means "now" and follows the clock.
    live: bool,
}

/// Everything that depends on a successful conversion.
struct Dial {
    /// The source time and date as the form fields carry them.
    time: String,
    date: String,
    date_label: String,
    /// Source minutes since midnight: the slider position.
    minutes: i32,
    hand_angle: f64,
    source_offset: i32,
    /// Labels for the day before, the source day and the day after, `|` separated.
    days: String,
    marks: Vec<Mark>,
    rows: Vec<TableRow>,
}

/// One marker on the ring. Places that share an offset share a marker.
struct Mark {
    offset: i32,
    angle: f64,
    band: Band,
    is_source: bool,
    places: Vec<String>,
    time: String,
    date: String,
    is_shifted: bool,
    /// Label lines beyond the usual two, so the label clears the ring.
    extra_lines: usize,
}

struct TableRow {
    label: String,
    offset: i32,
    offset_label: String,
    band: Band,
    is_source: bool,
    time: String,
    date: String,
    is_shifted: bool,
}

impl App {
    fn new(places: Vec<Place>) -> Self {
        let mut zone_names: Vec<String> = places.iter().map(|p| p.label.clone()).collect();
        zone_names.extend(jiff::tz::db().available().map(|name| name.to_string()));

        let mut hasher = DefaultHasher::new();
        ASSETS
            .iter()
            .for_each(|(_, _, bytes)| bytes.hash(&mut hasher));
        Self {
            places,
            zone_names,
            asset_version: format!("{:x}", hasher.finish()),
        }
    }
}

impl Dial {
    fn new(rows: &[Row]) -> Option<Self> {
        let source = &rows.iter().find(|row| row.is_source)?.at;
        let day = |date: jiff::civil::Date| date.strftime(DAY_FORMAT).to_string();
        let days = [
            source.date().yesterday().ok()?,
            source.date(),
            source.date().tomorrow().ok()?,
        ]
        .map(day)
        .join("|");

        let mut marks: Vec<Mark> = Vec::new();
        for row in rows {
            let offset = row.at.offset().seconds();
            match marks.iter_mut().find(|mark| mark.offset == offset) {
                Some(mark) => {
                    mark.places.push(row.label.clone());
                    mark.is_source |= row.is_source;
                    mark.extra_lines += 1;
                }
                None => marks.push(Mark {
                    offset,
                    angle: angle(minutes(&row.at), row.day_shift),
                    band: row.band,
                    is_source: row.is_source,
                    places: vec![row.label.clone()],
                    time: row.at.strftime("%H:%M").to_string(),
                    date: day(row.at.date()),
                    is_shifted: row.day_shift != 0,
                    extra_lines: usize::from(row.day_shift != 0),
                }),
            }
        }
        let rows = rows
            .iter()
            .map(|row| TableRow {
                label: row.label.clone(),
                offset: row.at.offset().seconds(),
                offset_label: row.at.strftime("%:z").to_string(),
                band: row.band,
                is_source: row.is_source,
                time: row.at.strftime("%H:%M").to_string(),
                date: day(row.at.date()),
                is_shifted: row.day_shift != 0,
            })
            .collect();
        Some(Self {
            time: source.strftime("%H:%M").to_string(),
            date: source.date().to_string(),
            date_label: day(source.date()),
            minutes: minutes(source),
            hand_angle: angle(minutes(source), 0),
            source_offset: source.offset().seconds(),
            days,
            marks,
            rows,
        })
    }
}

fn minutes(at: &jiff::Zoned) -> i32 {
    i32::from(at.hour()) * 60 + i32::from(at.minute())
}

/// Degrees clockwise from noon. Not wrapped to a turn, so that a place on
/// another date keeps a continuous path when the dial is turned in the browser.
fn angle(minutes: i32, day_shift: i32) -> f64 {
    (f64::from(minutes) + MINUTES_PER_DAY * f64::from(day_shift)) / MINUTES_PER_DEGREE
        - NOON_DEGREES
}

fn default_zone() -> String {
    TimeZone::system()
        .iana_name()
        .filter(|name| *name != UNKNOWN_ZONE)
        .unwrap_or(FALLBACK_ZONE)
        .to_string()
}

fn given(value: Option<String>) -> Option<String> {
    value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

async fn index(
    State(app): State<Arc<App>>,
    headers: HeaderMap,
    UrlQuery(params): UrlQuery<Params>,
) -> Response {
    let (time, date) = (given(params.time), given(params.date));
    let live = time.is_none() && date.is_none();
    let zone = given(params.zone).unwrap_or_else(default_zone);
    let query = Query {
        time: time.as_deref(),
        date: date.as_deref(),
        zone: Some(&zone),
    };
    let (dial, error) = match convert(query, &app.places) {
        Ok(rows) => (Dial::new(&rows), None),
        Err(e) => (None, Some(e)),
    };
    let invalid = match error {
        Some(Error::Time(_)) => "time",
        Some(Error::Date(_) | Error::Moment(_)) => "date",
        Some(Error::Zone(..)) => "zone",
        _ => "",
    };
    // Show the normalised moment, or echo what was typed so it can be corrected in place.
    let (time, date) = match &dial {
        Some(d) => (d.time.clone(), d.date.clone()),
        None => (time.unwrap_or_default(), date.unwrap_or_default()),
    };
    let page = Page {
        asset_version: &app.asset_version,
        zone_names: &app.zone_names,
        time,
        date,
        zone,
        invalid,
        error: error.as_ref().map(Error::to_string),
        dial,
        table_view: params.view.as_deref() == Some("table"),
        live,
    };
    // htmx ignores 4xx bodies, and the alert is in the body.
    let status = match (&page.error, headers.contains_key("hx-request")) {
        (Some(_), false) => StatusCode::BAD_REQUEST,
        _ => StatusCode::OK,
    };
    match page.render() {
        Ok(html) => (status, Html(html)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn asset(Path(path): Path<String>) -> Response {
    match ASSETS.iter().find(|(name, ..)| *name == path) {
        Some((_, mime, bytes)) => (
            [
                (header::CONTENT_TYPE, *mime),
                (header::CACHE_CONTROL, IMMUTABLE),
            ],
            *bytes,
        )
            .into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

fn router(app: App) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/assets/{*path}", get(asset))
        .route("/healthz", get(|| async { "ok" }))
        .with_state(Arc::new(app))
}

/// Resolves on Ctrl-C or SIGTERM.
///
/// As PID 1 in a container the process gets no default SIGTERM action, so
/// without this `docker stop` waits out its full timeout.
async fn shutdown() {
    let mut terminate = signal(SignalKind::terminate()).expect("SIGTERM handler installs");
    tokio::select! {
        _ = ctrl_c() => {}
        _ = terminate.recv() => {}
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = match env::var(PORT_VAR) {
        Ok(value) => value
            .parse()
            .map_err(|_| format!("{PORT_VAR} must be a port number, got {value:?}"))?,
        Err(_) => DEFAULT_PORT,
    };
    let app = App::new(load_places()?);
    // Inside a container the host side of the port mapping decides who can reach this.
    let listener = TcpListener::bind(("0.0.0.0", port)).await?;
    eprintln!(
        "zoners-web: {} places, listening on http://0.0.0.0:{port}",
        app.places.len()
    );
    axum::serve(listener, router(app))
        .with_graceful_shutdown(shutdown())
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{router, App, IMMUTABLE};
    use axum::{
        body::{to_bytes, Body},
        http::{header, Request, StatusCode},
    };
    use tower::ServiceExt;
    use zoners::parse_places;

    const EVENING: &str = "/?time=8.30pm&date=2026-09-22&zone=GMT%2B4";

    async fn get(uri: &str, htmx: bool) -> (StatusCode, String, String) {
        let places = parse_places("Asia/Tokyo Tokyo; America/New_York New York").unwrap();
        let mut request = Request::get(uri);
        if htmx {
            request = request.header("hx-request", "true");
        }
        let response = router(App::new(places))
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap();
        let cache = response
            .headers()
            .get(header::CACHE_CONTROL)
            .map(|v| v.to_str().unwrap().to_string())
            .unwrap_or_default();
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (status, String::from_utf8_lossy(&body).into_owned(), cache)
    }

    #[tokio::test]
    async fn page() {
        let (status, body, _) = get(EVENING, false).await;
        assert_eq!(status, StatusCode::OK);
        // Tokyo is past midnight: 22 Sep 20:30 at +04:00 is 23 Sep 01:30 at +09:00.
        // The colon sits in its own element, so that CSS can blink it.
        assert!(body.contains(r#">01<i class="c">:</i>30<"#) && body.contains("Wed 23 Sep"));
        assert!(body.contains(r#"value="20:30""#), "time is normalised");
        // 01:30 on the next day: (90 + 1440) / 4 - 180.
        assert!(body.contains("--a:202.5deg"));
        assert!(!body.contains("checked") && !body.contains("data-live"));
        assert!(get("/?zone=UTC", false).await.1.contains("data-live"));
        assert!(get(&format!("{EVENING}&view=table"), false)
            .await
            .1
            .contains("checked"));
    }

    #[tokio::test]
    async fn bad_zone() {
        let uri = "/?zone=%3Cb%3EMars";
        let (status, body, _) = get(uri, false).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(body.contains("zone must be"));
        assert!(!body.contains("<b>Mars"), "input is escaped");
        assert_eq!(get(uri, true).await.0, StatusCode::OK);
    }

    #[tokio::test]
    async fn assets_and_health() {
        let (status, body, cache) = get("/assets/app.css", false).await;
        assert_eq!((status, cache.as_str()), (StatusCode::OK, IMMUTABLE));
        assert!(body.contains("@font-face"));
        assert_eq!(
            get("/assets/../Cargo.toml", false).await.0,
            StatusCode::NOT_FOUND
        );
        assert_eq!(get("/healthz", false).await.1, "ok");
    }
}
