//! The `zoners` CLI: prints one moment as a psql-style table.

use anstyle::{AnsiColor, Style};
use clap::Parser;
use zoners::{convert, load_places, Band, Query, Result};

/// Print a moment in time across your configured zones.
#[derive(Parser)]
struct Args {
    /// `8.30 pm`, `8:30pm`, `8pm`, `20:30`. Default: now.
    #[arg(long, num_args = 1..=2)]
    time: Vec<String>,
    /// `YYYY-MM-DD`. Default: today in the source zone.
    #[arg(long)]
    date: Option<String>,
    /// Config label, `GMT+4` / `UTC+5:30` / `+04:00`, or IANA name. Default: system zone.
    #[arg(long, allow_hyphen_values = true)]
    zone: Option<String>,
}

const BOLD: Style = Style::new().bold();
const DIM: Style = Style::new().dimmed();
const WORK: Style = AnsiColor::Green.on_default();
const AWAKE: Style = AnsiColor::Yellow.on_default();
const OTHER_DAY: Style = AnsiColor::Magenta.on_default();
const MIN_PLACE_WIDTH: usize = 5;

const fn band_style(band: Band) -> Style {
    match band {
        Band::Work => WORK,
        Band::Awake => AWAKE,
        Band::Night => DIM,
    }
}

fn run() -> Result {
    let args = Args::parse();
    let time = (!args.time.is_empty()).then(|| args.time.join(""));
    let query = Query {
        time: time.as_deref(),
        date: args.date.as_deref(),
        zone: args.zone.as_deref(),
    };
    let rows = convert(query, &load_places()?)?;

    let w = rows
        .iter()
        .map(|row| row.label.chars().count())
        .max()
        .unwrap_or(0)
        .max(MIN_PLACE_WIDTH);
    // psql layout. Pad first, then style: ANSI codes would count towards the width.
    let cell = |st: Style, s: &str, w: usize| format!("{st}{s:<w$}{st:#}");
    let bar = format!(" {DIM}|{DIM:#} ");
    let head = [("PLACE", w), ("TIME", 5), ("DAY", 10), ("OFFSET", 6)];
    let names = head.map(|(s, w)| format!("{BOLD}{s:^w$}{BOLD:#}"));
    let rule = head.map(|(_, w)| "-".repeat(w + 2));
    anstream::println!(" {}", names.join(&bar));
    anstream::println!("{DIM}{}{DIM:#}", rule.join("+"));
    for row in &rows {
        let mut hs = band_style(row.band);
        if row.is_source {
            hs = hs.bold();
        }
        let ds = if row.day_shift == 0 {
            Style::new()
        } else {
            OTHER_DAY
        };
        let cells = [
            cell(hs, &row.label, w),
            cell(hs, &row.at.strftime("%H:%M").to_string(), 5),
            cell(ds, &row.at.strftime("%a %d %b").to_string(), 10),
            cell(DIM, &row.at.strftime("%:z").to_string(), 6),
        ];
        anstream::println!(" {}", cells.join(&bar));
    }
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("zoners: {e}");
        std::process::exit(1);
    }
}
