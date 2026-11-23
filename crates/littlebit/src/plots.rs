//! Gate 4's two plots (docs/littlebit.md, "Experiments"), drawn from the experiments table there:
//! test accuracy against layer 1's total bits per weight, E1 to E8; and logic cells against the
//! sign bits hardwired, for Variant A's builds of the narrow network, E4 to E8 and E11.
//! `cargo run -p tentzhen-littlebit -- plots`, from the repo root, writes them as SVG to
//! [`ACCURACY`] and [`CELLS`], and a test fails if the committed files are not what it writes.
//!
//! Each is a still image for a page that shows it as one, so it has no hover; the experiments
//! table beside it carries every value. It paints its own surface, light, or dark when the
//! viewer's system prefers dark, so it reads on either page. Its colors are the first two slots
//! of the dataviz skill's reference palette, in both modes, checked with that skill's validator
//! (2026-11-23): every check passed, adjacent and all pairs.

/// Where the experiments table is, relative to the repo root.
pub const DOC: &str = "docs/littlebit.md";
/// Where the accuracy plot is written, relative to the repo root.
pub const ACCURACY: &str = "docs/img/littlebit-accuracy.svg";
/// Where the cells plot is written, relative to the repo root.
pub const CELLS: &str = "docs/img/littlebit-cells.svg";

/// The runs the accuracy plot draws: the dense baselines and the rank sweep.
pub const ACCURACY_RUNS: [&str; 8] = ["E1", "E2", "E3", "E4", "E5", "E6", "E7", "E8"];
/// The runs the cells plot draws: Variant A's builds of the 784 -> 256 -> 128 -> 10 network.
pub const CELLS_RUNS: [&str; 6] = ["E4", "E5", "E6", "E7", "E8", "E11"];

/// The sign bits of the narrow network's two hidden layers, per unit of rank: r (784 + 256) +
/// r (256 + 128).
pub const SIGN_BITS_PER_RANK: u32 = 784 + 256 + 256 + 128;

/// One row of the experiments table, as the plots read it.
#[derive(Debug, Clone, PartialEq)]
pub struct Experiment {
    pub run: String,
    pub model: String,
    /// Layer 1's total bits per weight.
    pub bpw: f64,
    /// The test accuracy in percent: the integer golden model's, or E1's in float.
    pub accuracy: f64,
    /// The accuracy's cell as the table writes it: "97.17 (float 97.13)", "98.31 (float)".
    pub accuracy_cell: String,
    /// nextpnr's logic cells, for a run that was built.
    pub cells: Option<u32>,
    /// nextpnr's maximum clock after routing, in MHz, for a run that was built.
    pub fmax: Option<f64>,
    /// The rank, for a LittleBit run.
    pub rank: Option<u32>,
}

/// The number `text` starts with, commas and all: 98.33 from "98.33 (float 98.35)", 6889 from
/// "6,889". None if it starts with none.
fn leading(text: &str) -> Option<f64> {
    let number: String = text
        .trim()
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == ',')
        .filter(|c| *c != ',')
        .collect();
    number.parse().ok()
}

/// The experiments table's rows, from docs/littlebit.md's text `doc`.
pub fn experiments(doc: &str) -> Result<Vec<Experiment>, String> {
    let header = "| run | model";
    let start = doc
        .find(header)
        .ok_or_else(|| format!("{DOC} has no table headed {header}"))?;
    let mut rows = Vec::new();
    for line in doc[start..]
        .lines()
        .skip(2)
        .take_while(|l| l.starts_with('|'))
    {
        let cells: Vec<&str> = line
            .trim()
            .trim_matches('|')
            .split('|')
            .map(str::trim)
            .collect();
        if cells.len() != 10 {
            return Err(format!("{}: {} cells, not 10", cells[0], cells.len()));
        }
        let number = |k: usize, what: &str| {
            leading(cells[k]).ok_or_else(|| format!("{}: no {what} in {:?}", cells[0], cells[k]))
        };
        let rank = cells[1]
            .split("r = ")
            .nth(1)
            .and_then(leading)
            .map(|r| r as u32);
        rows.push(Experiment {
            run: cells[0].to_string(),
            model: cells[1].to_string(),
            bpw: number(3, "bits per weight")?,
            accuracy: number(4, "accuracy")?,
            accuracy_cell: cells[4].to_string(),
            cells: leading(cells[5]).map(|c| c as u32),
            fmax: leading(cells[7]),
            rank,
        });
    }
    Ok(rows)
}

/// The rows of `runs`, in that order, or an error naming one the table lacks.
fn pick<'a>(rows: &'a [Experiment], runs: &[&str]) -> Result<Vec<&'a Experiment>, String> {
    runs.iter()
        .map(|run| {
            rows.iter()
                .find(|e| e.run == *run)
                .ok_or_else(|| format!("the experiments table has no {run}"))
        })
        .collect()
}

const WIDTH: f64 = 680.0;
const HEIGHT: f64 = 430.0;
const TOP: f64 = 110.0;
const BOTTOM: f64 = 370.0;
const RIGHT: f64 = 648.0;

/// The page's surface, ink and series colors, light and then dark: the reference palette's
/// chart chrome and its first two categorical slots.
const STYLE: &str = "
  svg { font-family: system-ui, -apple-system, \"Segoe UI\", sans-serif; }
  .surface { fill: #fcfcfb; }
  .title { fill: #0b0b0b; font-size: 15px; font-weight: 600; }
  .subtitle, .legend, .label, .axis-title { fill: #52514e; font-size: 12px; }
  .label { font-size: 11px; }
  .tick { fill: #898781; font-size: 11px; font-variant-numeric: tabular-nums; }
  .grid { stroke: #e1e0d9; stroke-width: 1; }
  .axis { stroke: #c3c2b7; stroke-width: 1; }
  .reference { stroke: #898781; stroke-width: 1; }
  .line-1 { fill: none; stroke: #2a78d6; stroke-width: 2; stroke-linejoin: round; stroke-linecap: round; }
  .dot-1 { fill: #2a78d6; stroke: #fcfcfb; stroke-width: 2; }
  .dot-2 { fill: #eb6834; stroke: #fcfcfb; stroke-width: 2; }
  @media (prefers-color-scheme: dark) {
    .surface { fill: #1a1a19; }
    .title { fill: #f0efec; }
    .subtitle, .legend, .label, .axis-title { fill: #c3c2b7; }
    .grid { stroke: #2c2c2a; }
    .axis { stroke: #383835; }
    .line-1 { stroke: #3987e5; }
    .dot-1 { fill: #3987e5; stroke: #1a1a19; }
    .dot-2 { fill: #d95926; stroke: #1a1a19; }
  }
";

/// Text escaped for SVG.
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// A plot under way: its SVG so far.
struct Svg(String);

impl Svg {
    fn new(title: &str, description: &str) -> Svg {
        Svg(format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{WIDTH}\" height=\"{HEIGHT}\" \
             viewBox=\"0 0 {WIDTH} {HEIGHT}\" role=\"img\" aria-labelledby=\"title desc\">\n\
             <title id=\"title\">{}</title>\n<desc id=\"desc\">{}</desc>\n<style>{STYLE}</style>\n\
             <rect class=\"surface\" width=\"{WIDTH}\" height=\"{HEIGHT}\" rx=\"8\"/>\n",
            escape(title),
            escape(description)
        ))
    }

    fn text(&mut self, class: &str, x: f64, y: f64, anchor: &str, text: &str) {
        self.0 += &format!(
            "<text class=\"{class}\" x=\"{x:.1}\" y=\"{y:.1}\" text-anchor=\"{anchor}\">{}</text>\n",
            escape(text)
        );
    }

    fn line(&mut self, class: &str, (x1, y1): (f64, f64), (x2, y2): (f64, f64)) {
        self.0 += &format!(
            "<line class=\"{class}\" x1=\"{x1:.1}\" y1=\"{y1:.1}\" x2=\"{x2:.1}\" y2=\"{y2:.1}\"/>\n"
        );
    }

    fn polyline(&mut self, class: &str, points: &[(f64, f64)]) {
        let points: Vec<String> = points
            .iter()
            .map(|(x, y)| format!("{x:.1},{y:.1}"))
            .collect();
        self.0 += &format!(
            "<polyline class=\"{class}\" points=\"{}\"/>\n",
            points.join(" ")
        );
    }

    fn dot(&mut self, class: &str, (x, y): (f64, f64)) {
        self.0 += &format!("<circle class=\"{class}\" cx=\"{x:.1}\" cy=\"{y:.1}\" r=\"5\"/>\n");
    }

    /// The title and subtitle, at the top left.
    fn heading(&mut self, title: &str, subtitle: &str) {
        self.text("title", 24.0, 28.0, "start", title);
        self.text("subtitle", 24.0, 47.0, "start", subtitle);
    }

    /// Hairline gridlines and their labels at each of `ticks`, (value, label), across the plot
    /// from `left`; then the baseline.
    fn y_axis(&mut self, left: f64, ticks: &[(f64, String)], y: impl Fn(f64) -> f64, title: &str) {
        for (value, label) in ticks {
            let at = y(*value);
            self.line("grid", (left, at), (RIGHT, at));
            self.text("tick", left - 8.0, at + 4.0, "end", label);
        }
        self.line("axis", (left, BOTTOM), (RIGHT, BOTTOM));
        self.text("axis-title", left, TOP - 14.0, "start", title);
    }

    /// Tick marks and labels along the baseline at each of `ticks`, and the axis's title.
    fn x_axis(&mut self, left: f64, ticks: &[(f64, String)], x: impl Fn(f64) -> f64, title: &str) {
        for (value, label) in ticks {
            let at = x(*value);
            self.line("axis", (at, BOTTOM), (at, BOTTOM + 5.0));
            self.text("tick", at, BOTTOM + 19.0, "middle", label);
        }
        self.text(
            "axis-title",
            (left + RIGHT) / 2.0,
            HEIGHT - 16.0,
            "middle",
            title,
        );
    }

    fn end(self) -> String {
        self.0 + "</svg>\n"
    }
}

/// A number with a comma between each three digits: 30,000.
fn thousands(n: f64) -> String {
    let digits = format!("{}", n.round() as u64);
    let mut out = String::new();
    for (k, c) in digits.chars().enumerate() {
        if k > 0 && (digits.len() - k) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// The accuracy plot: each of [`ACCURACY_RUNS`]'s test accuracy against its layer-1 total bits
/// per weight, on a log scale. The LittleBit runs are one series, joined in order of bits; the
/// dense baselines are the other, unjoined.
pub fn accuracy(rows: &[Experiment]) -> Result<String, String> {
    let runs = pick(rows, &ACCURACY_RUNS)?;
    let (low, high) = (86.0, 100.0);
    if let Some(e) = runs.iter().find(|e| !(low..=high).contains(&e.accuracy)) {
        return Err(format!(
            "{}'s {} % is off the plot's {low}-{high} %",
            e.run, e.accuracy
        ));
    }
    let left = 64.0;
    // Layer 1's bits per weight, 1/16 to 32, with room at each end.
    let (from, to) = (-4.4, 5.4);
    let x = |bpw: f64| left + (bpw.log2() - from) / (to - from) * (RIGHT - left);
    let y = |pct: f64| BOTTOM - (pct - low) / (high - low) * (BOTTOM - TOP);

    let values: Vec<String> = runs
        .iter()
        .map(|e| format!("{}, {} bits, {} %", e.run, e.bpw, e.accuracy))
        .collect();
    let mut svg = Svg::new(
        "Test accuracy against bits per weight, E1 to E8",
        &format!(
            "MNIST test images right, in percent, against layer 1's total bits per weight on a \
             log scale: {}. E4 to E8 are LittleBit at r = 4 to 64; E1 to E3 are dense, FP32, \
             int8 and binary. The experiments table gives each.",
            values.join("; ")
        ),
    );
    svg.heading(
        "Test accuracy against bits per weight, E1 to E8",
        "MNIST's 10,000 test images, as the integer golden model scores them; E1 in float",
    );
    // The legend: a line and dot for the LittleBit series, a dot for the baselines.
    svg.line("line-1", (24.0, 66.0), (44.0, 66.0));
    svg.dot("dot-1", (34.0, 66.0));
    svg.text(
        "legend",
        52.0,
        70.0,
        "start",
        "LittleBit, r = 4 to 64 (E4 to E8)",
    );
    svg.dot("dot-2", (290.0, 66.0));
    svg.text(
        "legend",
        300.0,
        70.0,
        "start",
        "dense: binary, int8 and FP32 (E3, E2, E1)",
    );

    let y_ticks: Vec<(f64, String)> = (0..=7)
        .map(|k| (low + 2.0 * f64::from(k), format!("{}", 86 + 2 * k)))
        .collect();
    svg.y_axis(left, &y_ticks, y, "test accuracy, %");
    let x_ticks: Vec<(f64, String)> = (-4..=5)
        .map(|k: i32| {
            let label = if k < 0 {
                format!("1/{}", 1 << -k)
            } else {
                format!("{}", 1 << k)
            };
            (2f64.powi(k), label)
        })
        .collect();
    svg.x_axis(
        left,
        &x_ticks,
        x,
        "layer 1's total bits per weight, signs and 8-bit scales (log scale)",
    );

    let mut littlebit: Vec<&&Experiment> = runs.iter().filter(|e| e.rank.is_some()).collect();
    littlebit.sort_by(|a, b| a.bpw.total_cmp(&b.bpw));
    let points: Vec<(f64, f64)> = littlebit
        .iter()
        .map(|e| (x(e.bpw), y(e.accuracy)))
        .collect();
    svg.polyline("line-1", &points);
    for (e, &(px, py)) in littlebit.iter().zip(&points) {
        svg.dot("dot-1", (px, py));
        svg.text("label", px + 7.0, py + 17.0, "start", &e.run);
    }
    for e in runs.iter().filter(|e| e.rank.is_none()) {
        let (px, py) = (x(e.bpw), y(e.accuracy));
        svg.dot("dot-2", (px, py));
        let kind = if e.model.starts_with("binary") {
            "binary"
        } else if e.model.starts_with("int8") {
            "int8"
        } else {
            "FP32"
        };
        svg.text(
            "label",
            px,
            py - 12.0,
            "middle",
            &format!("{}, {kind}", e.run),
        );
    }
    Ok(svg.end())
}

/// The cells plot: each of [`CELLS_RUNS`]'s logic cells against the sign bits its two hidden
/// layers hardwire, joined in order of rank, with Gate 2's budget of 30,000 cells.
pub fn cells(rows: &[Experiment]) -> Result<String, String> {
    let runs = pick(rows, &CELLS_RUNS)?;
    let mut points = Vec::new();
    for e in &runs {
        let rank = e.rank.ok_or_else(|| format!("{} names no rank", e.run))?;
        let cells = e
            .cells
            .ok_or_else(|| format!("{} has no logic cells", e.run))?;
        let fmax = e.fmax.ok_or_else(|| format!("{} has no Fmax", e.run))?;
        points.push((rank, rank * SIGN_BITS_PER_RANK, cells, e.run.as_str(), fmax));
    }
    points.sort_by_key(|p| p.0);
    let (x_max, y_max) = (100_000.0, 40_000.0);
    let left = 72.0;
    let x = |bits: f64| left + bits / x_max * (RIGHT - left);
    let y = |cells: f64| BOTTOM - cells / y_max * (BOTTOM - TOP);
    if let Some(p) = points
        .iter()
        .find(|p| f64::from(p.1) > x_max || f64::from(p.2) > y_max)
    {
        return Err(format!("{} is off the plot", p.3));
    }

    let values: Vec<String> = points
        .iter()
        .map(|p| {
            format!(
                "{} at r = {}, {} sign bits, {} cells, {} MHz",
                p.3,
                p.0,
                thousands(f64::from(p.1)),
                thousands(f64::from(p.2)),
                p.4
            )
        })
        .collect();
    let mut svg = Svg::new(
        "Logic cells against sign bits, Variant A",
        &format!(
            "nextpnr's logic cells for the whole board design against the sign bits its two \
             hidden layers hardwire, 1,424 a unit of rank, with Gate 2's budget of 30,000 cells: \
             {}. The experiments table gives each.",
            values.join("; ")
        ),
    );
    svg.heading(
        "Logic cells against sign bits, Variant A",
        "The board's whole design, weights hardwired, as nextpnr counts its cells (TRELLIS_COMB)",
    );
    let y_ticks: Vec<(f64, String)> = (0..=4)
        .map(|k| {
            (
                10_000.0 * f64::from(k),
                if k == 0 {
                    "0".into()
                } else {
                    format!("{}0K", k)
                },
            )
        })
        .collect();
    svg.y_axis(left, &y_ticks, y, "logic cells");
    let x_ticks: Vec<(f64, String)> = (0..=5)
        .map(|k| (20_000.0 * f64::from(k), thousands(20_000.0 * f64::from(k))))
        .collect();
    svg.x_axis(
        left,
        &x_ticks,
        x,
        "sign bits hardwired, both hidden layers: 1,424 × r",
    );

    // Gate 2's budget, a hairline across the plot, named at its left end.
    let budget = y(30_000.0);
    svg.line("reference", (left, budget), (RIGHT, budget));
    svg.text(
        "label",
        left + 6.0,
        budget - 6.0,
        "start",
        "Gate 2's budget, 30,000 cells",
    );

    let xy: Vec<(f64, f64)> = points
        .iter()
        .map(|p| (x(f64::from(p.1)), y(f64::from(p.2))))
        .collect();
    svg.polyline("line-1", &xy);
    let last = points.len() - 1;
    for (k, (p, &(px, py))) in points.iter().zip(&xy).enumerate() {
        svg.dot("dot-1", (px, py));
        let mut label = format!("{}, r = {}", p.3, p.0);
        if p.4 < 50.0 {
            label += ", misses 50 MHz";
        }
        if k == last {
            // The last is near the right edge: its label goes above and to its left.
            svg.text("label", px - 8.0, py - 10.0, "end", &label);
        } else {
            svg.text("label", px + 7.0, py + 17.0, "start", &label);
        }
    }
    Ok(svg.end())
}
