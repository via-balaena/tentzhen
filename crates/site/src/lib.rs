//! Generates tentzhen.com's build pages and parts page from the warehouse's gold views, and nothing
//! else.
//!
//! The records in `parts/`, `builds/` and `lab/` are loaded into DuckDB (crates/warehouse); every
//! value on a page is read back out of a `gold.page*` view. Every build version gets a permanent
//! page at `site/builds/<name>/v<N>/`; `site/builds/<name>/` sends you to the newest. `site/parts/`
//! lists everything the bench needs, and how far the lab has got buying it. `site/fab/` sets Hacker
//! Fab's stack beside the industry's, with the gap between them. Pages are output, never edited: the
//! Quality Gate reruns this and fails if `site/` differs.

use duckdb::{Connection, params};
use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use tentzhen_records::Class;

fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

/// A whole page. `up` is the path from the page back to `site/`.
fn page(up: &str, title: &str, body: &str) -> String {
    format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<link rel="icon" href="{up}img/favicon.svg" type="image/svg+xml">
<link rel="stylesheet" href="{up}style.css">
</head>
<body>

<header class="column site-header">
  <a class="brand" href="{up}index.html" aria-label="Tentzhen home">
    <img src="{up}img/mark-black.svg" alt="">
    <span class="wordmark">TENTZHEN</span>
  </a>
  <nav class="site-nav" aria-label="Main">
    <a href="{up}builds/index.html">BUILDS</a>
    <a href="{up}parts/index.html">PARTS</a>
    <a href="https://github.com/via-balaena/tentzhen/blob/main/docs/vision.md">VISION</a>
    <div class="nav-menu">
      <button type="button" class="nav-label">PROJECTS</button>
      <div class="nav-items">
        <a href="https://github.com/via-balaena/tentzhen/blob/main/docs/scope.md">DIGITIZER AND GENERATOR</a>
        <a href="https://github.com/via-balaena/tentzhen/blob/main/docs/mesh.md">THE MESH</a>
        <a href="https://github.com/via-balaena/tentzhen/blob/main/docs/littlebit.md">HARDWIRED NEURAL NET</a>
        <a href="{up}fab/index.html">THE OPEN FAB</a>
      </div>
    </div>
    <a href="https://github.com/via-balaena/tentzhen/blob/main/docs/verification.md">VERIFICATION</a>
    <a href="https://github.com/via-balaena/tentzhen/blob/main/docs/roadmap.md">ROADMAP</a>
    <a href="https://github.com/via-balaena/tentzhen">GITHUB</a>
  </nav>
</header>

<main class="column spec">
{body}</main>

<footer class="site-footer">
  <div class="column">
    <span>腾振 TENTZHEN · A VIA BALAENA PROJECT</span>
    <span>MIT OR APACHE-2.0 · <a href="{up}legal/index.html">LEGAL</a></span>
    <a href="https://github.com/via-balaena/tentzhen">GITHUB.COM/VIA-BALAENA/TENTZHEN</a>
  </div>
</footer>

</body>
</html>
"#
    )
}

/// A parts line: qty, part, commodity, form, datasheet, authorized_only (`gold.page_line`).
type LineRow = (
    i64,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    bool,
);
/// A walkthrough step: instruction, run, expect, agent (`gold.page_step`).
type StepRow = (String, Option<String>, Option<String>, Option<String>);
/// A claim: key, says, sim grade, sim evidence, bench grade, bench evidence (`gold.claim_grades`).
type ClaimRow = (
    String,
    String,
    String,
    Option<String>,
    String,
    Option<String>,
);

/// A claim's words with each `name` it states replaced by its number and unit, as
/// `gold.claim_values` gives them. A name the claim does not state stays as written.
fn fill(says: &str, values: &[(String, f64)]) -> String {
    let mut out = says.to_string();
    for (name, amount) in values {
        let unit = match name.rsplit('_').next().unwrap_or_default() {
            "volts" => " V",
            "amps" => " A",
            "ohms" => " Ω",
            "watts" => " W",
            "hertz" => " Hz",
            "seconds" => " s",
            "kelvin" => " K",
            "codes" => " codes",
            _ => "",
        };
        out = out.replace(&format!("`{name}`"), &format!("{amount}{unit}"));
    }
    out
}

/// One build version, as `gold.page` gives it.
pub struct PageRow {
    pub build: String,
    pub version: u32,
    pub status: String,
    pub does: String,
    pub changes: Option<String>,
    pub latest: u32,
    pub latest_changes: Option<String>,
    pub drawing_svg: Option<String>,
}

fn u32_at(row: &duckdb::Row, i: usize) -> duckdb::Result<u32> {
    let v: i64 = row.get(i)?;
    u32::try_from(v).map_err(|e| {
        duckdb::Error::FromSqlConversionFailure(i, duckdb::types::Type::BigInt, Box::new(e))
    })
}

pub fn pages(conn: &Connection) -> duckdb::Result<Vec<PageRow>> {
    let mut stmt = conn.prepare(
        "SELECT build, version, status, does, changes, latest, latest_changes, drawing_svg \
         FROM gold.page ORDER BY build, version",
    )?;
    stmt.query_map([], |r| {
        Ok(PageRow {
            build: r.get(0)?,
            version: u32_at(r, 1)?,
            status: r.get(2)?,
            does: r.get(3)?,
            changes: r.get(4)?,
            latest: u32_at(r, 5)?,
            latest_changes: r.get(6)?,
            drawing_svg: r.get(7)?,
        })
    })?
    .collect()
}

pub fn render_index(pages: &[PageRow]) -> String {
    let mut body = String::from(
        "<h1 class=\"spec-title\">Builds</h1>\n<table class=\"spec-table\">\n<thead><tr><th>BUILD</th><th>LATEST</th><th>DOES</th></tr></thead>\n<tbody>\n",
    );
    for b in pages.iter().filter(|p| p.version == p.latest) {
        let _ = writeln!(
            body,
            "<tr><td><a href=\"{n}/v{v}/index.html\">{n}</a></td><td>v{v} · {s}</td><td>{d}</td></tr>",
            n = esc(&b.build),
            v = b.version,
            s = b.status,
            d = esc(&b.does)
        );
    }
    body.push_str("</tbody>\n</table>\n");
    page("../", "Builds · Tentzhen", &body)
}

/// `site/builds/<name>/`: always the newest version, by redirect, so a link to a build never
/// strands anyone on an old one.
pub fn render_latest(name: &str, latest: u32) -> String {
    format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<title>{n} · Tentzhen</title>\n\
         <meta http-equiv=\"refresh\" content=\"0; url=v{latest}/index.html\">\n</head>\n<body>\n\
         <p><a href=\"v{latest}/index.html\">{n} v{latest}</a></p>\n</body>\n</html>\n",
        n = esc(name)
    )
}

fn chip(grade: &str) -> String {
    format!("<span class=\"grade {grade}\">{grade}</span>")
}

pub fn render_version(conn: &Connection, b: &PageRow) -> duckdb::Result<String> {
    let up = "../../../";
    let key = params![b.build, b.version];
    let name = esc(&b.build);
    let mut h = String::new();

    let _ = writeln!(
        h,
        "<p class=\"crumbs\"><a href=\"../../index.html\">BUILDS</a> / {} / V{}</p>",
        name.to_uppercase(),
        b.version
    );
    let _ = writeln!(h, "<h1 class=\"spec-title\">{name}</h1>");
    let mut links = String::new();
    for v in 1..=b.latest {
        if v == b.version {
            let _ = write!(links, " <span class=\"here\">v{v}</span>");
        } else {
            let _ = write!(links, " <a href=\"../v{v}/index.html\">v{v}</a>");
        }
    }
    let _ = writeln!(
        h,
        "<p class=\"spec-meta\"><span class=\"state {s}\">v{} · {s}</span><span class=\"versions\">versions{links}</span></p>",
        b.version,
        s = b.status
    );
    if b.version < b.latest {
        let _ = writeln!(
            h,
            "<p class=\"banner\">Superseded by <a href=\"../v{0}/index.html\">v{0}</a>. {1}</p>",
            b.latest,
            esc(b.latest_changes.as_deref().unwrap_or(""))
        );
    }
    if let Some(c) = &b.changes {
        let _ = writeln!(
            h,
            "<p class=\"spec-changes\">Changed from v{}: {}</p>",
            b.version - 1,
            esc(c)
        );
    }
    let _ = writeln!(h, "<p class=\"spec-does\">{}</p>", esc(&b.does));
    if let Some(svg) = &b.drawing_svg {
        let _ = writeln!(
            h,
            "<figure class=\"drawing\" role=\"img\" aria-label=\"Drawing of {name} v{}\">\n{}\n</figure>",
            b.version,
            svg.trim_end()
        );
    }

    let mut stmt = conn.prepare(
        "SELECT qty, part, commodity, form, datasheet, authorized_only FROM gold.page_line \
         WHERE build = ? AND version = ? ORDER BY line_no",
    )?;
    let lines: Vec<LineRow> = stmt
        .query_map(key, |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
            ))
        })?
        .collect::<duckdb::Result<_>>()?;
    if !lines.is_empty() {
        h.push_str("<h2>Parts</h2>\n<table class=\"spec-table\">\n<thead><tr><th>QTY</th><th>PART</th><th>FORM</th><th></th></tr></thead>\n<tbody>\n");
        for (qty, part, commodity, form, datasheet, authorized_only) in &lines {
            let (what, note) = match (part, commodity) {
                (Some(p), _) => {
                    let what = match datasheet {
                        Some(url) => format!("<a href=\"{}\">{}</a>", esc(url), esc(p)),
                        None => esc(p),
                    };
                    (
                        what,
                        if *authorized_only {
                            "authorized sellers only"
                        } else {
                            ""
                        },
                    )
                }
                (None, Some(c)) => (esc(c), ""),
                (None, None) => (String::new(), ""),
            };
            let _ = writeln!(
                h,
                "<tr><td>{qty}</td><td>{what}</td><td>{}</td><td class=\"muted\">{note}</td></tr>",
                esc(form.as_deref().unwrap_or(""))
            );
        }
        h.push_str("</tbody>\n</table>\n");
    }

    let refs = |sql: &str| -> duckdb::Result<Vec<(String, u32)>> {
        let mut stmt = conn.prepare(sql)?;
        stmt.query_map(key, |r| Ok((r.get(0)?, u32_at(r, 1)?)))?
            .collect()
    };
    let uses = refs(
        "SELECT uses_build, uses_version FROM gold.page_uses \
         WHERE build = ? AND version = ? ORDER BY uses_no",
    )?;
    let used = refs(
        "SELECT in_build, in_version FROM gold.page_used_in \
         WHERE build = ? AND version = ? ORDER BY in_build, in_version",
    )?;
    for (title, list) in [("Uses", &uses), ("Used in", &used)] {
        if list.is_empty() {
            continue;
        }
        let _ = writeln!(h, "<h2>{title}</h2>\n<ul class=\"refs\">");
        for (n, v) in list {
            let _ = writeln!(
                h,
                "<li><a href=\"../../{0}/v{1}/index.html\">{0} v{1}</a></li>",
                esc(n),
                v
            );
        }
        h.push_str("</ul>\n");
    }

    let mut stmt = conn.prepare(
        "SELECT name, license, source, release, file, sha256, pin_map FROM gold.page_firmware \
         WHERE build = ? AND version = ?",
    )?;
    let firmware: Vec<[Option<String>; 7]> = stmt
        .query_map(key, |r| {
            Ok([
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
                r.get(6)?,
            ])
        })?
        .collect::<duckdb::Result<_>>()?;
    if let Some([name_, license, source, release, file, sha256, pin_map]) = firmware.first() {
        let t = |s: &Option<String>| esc(s.as_deref().unwrap_or(""));
        h.push_str("<h2>Firmware</h2>\n<dl class=\"facts\">\n");
        let _ = writeln!(
            h,
            "<div><dt>NAME</dt><dd><a href=\"{}\">{}</a> · {}</dd></div>",
            t(source),
            t(name_),
            t(license)
        );
        let _ = writeln!(h, "<div><dt>RELEASE</dt><dd>{}</dd></div>", t(release));
        let _ = writeln!(h, "<div><dt>FILE</dt><dd>{}</dd></div>", t(file));
        let _ = writeln!(
            h,
            "<div><dt>SHA-256</dt><dd class=\"hash\">{}</dd></div>",
            t(sha256)
        );
        if let Some(p) = pin_map {
            let _ = writeln!(h, "<div><dt>PIN MAP</dt><dd>{}</dd></div>", esc(p));
        }
        h.push_str("</dl>\n");
    }

    let mut stmt = conn.prepare(
        "SELECT name, board_pin, net, required FROM gold.page_pin \
         WHERE build = ? AND version = ? ORDER BY pin_no",
    )?;
    let pins: Vec<(String, i64, String, bool)> = stmt
        .query_map(key, |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
        .collect::<duckdb::Result<_>>()?;
    if !pins.is_empty() {
        h.push_str("<h2>Pins</h2>\n<table class=\"spec-table\">\n<thead><tr><th>PIN</th><th>BOARD PIN</th><th>NET</th><th></th></tr></thead>\n<tbody>\n");
        for (pin, board_pin, net, required) in &pins {
            let _ = writeln!(
                h,
                "<tr><td>{}</td><td>{board_pin}</td><td>{}</td><td class=\"muted\">{}</td></tr>",
                esc(pin),
                esc(net),
                if *required { "required" } else { "optional" }
            );
        }
        h.push_str("</tbody>\n</table>\n");
    }

    let mut stmt = conn.prepare(
        "SELECT instruction, run, expect, agent FROM gold.page_step \
         WHERE build = ? AND version = ? ORDER BY step_no",
    )?;
    let steps: Vec<StepRow> = stmt
        .query_map(key, |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
        .collect::<duckdb::Result<_>>()?;
    if !steps.is_empty() {
        h.push_str("<h2>Walkthrough</h2>\n<p class=\"notice\">At your own risk: not certified or calibrated test equipment. <a href=\"../../../legal/index.html\">Legal</a></p>\n<ol class=\"steps\">\n");
        for (instruction, run, expect, agent) in &steps {
            let _ = write!(h, "<li><p>{}</p>", esc(instruction));
            if let Some(r) = run {
                let _ = write!(h, "<pre><code>{}</code></pre>", esc(r));
            }
            if let Some(e) = expect {
                let _ = write!(h, "<p class=\"expect\"><span>EXPECT</span>{}</p>", esc(e));
            }
            if let Some(a) = agent {
                let _ = write!(h, "<p class=\"agent\"><span>AGENTS</span>{}</p>", esc(a));
            }
            h.push_str("</li>\n");
        }
        h.push_str("</ol>\n");
    }

    let mut stmt = conn.prepare(
        "SELECT claim, says, sim_grade, sim_evidence, bench_grade, bench_evidence \
         FROM gold.claim_grades WHERE build = ? AND version = ? ORDER BY claim_no",
    )?;
    let claims: Vec<ClaimRow> = stmt
        .query_map(key, |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
            ))
        })?
        .collect::<duckdb::Result<_>>()?;
    let mut values = conn.prepare("SELECT name, amount FROM gold.claim_values WHERE claim = ?")?;
    if !claims.is_empty() {
        h.push_str("<h2>Claims</h2>\n<table class=\"spec-table claims\">\n<thead><tr><th>CLAIM</th><th>SIMULATION</th><th>ON THE BENCH</th></tr></thead>\n<tbody>\n");
        for (claim, says, sim_grade, sim_evidence, bench_grade, bench_evidence) in &claims {
            let stated: Vec<(String, f64)> = values
                .query_map([claim], |r| Ok((r.get(0)?, r.get(1)?)))?
                .collect::<duckdb::Result<_>>()?;
            let evidence = |e: &Option<String>| {
                e.as_ref()
                    .map(|e| format!(" <span class=\"muted\">{}</span>", esc(e)))
                    .unwrap_or_default()
            };
            let _ = writeln!(
                h,
                "<tr><td>{}</td><td>{}{}</td><td>{}{}</td></tr>",
                esc(&fill(says, &stated)),
                chip(sim_grade),
                evidence(sim_evidence),
                chip(bench_grade),
                evidence(bench_evidence)
            );
        }
        h.push_str("</tbody>\n</table>\n");
    }

    Ok(page(up, &format!("{name} v{} · Tentzhen", b.version), &h))
}

/// A shop as the page writes it.
fn shop_name(shop: &str) -> &str {
    match shop {
        "aliexpress" => "AliExpress",
        "amazon" => "Amazon",
        "lcsc" => "LCSC",
        "taobao" => "Taobao",
        other => other,
    }
}

/// A row of the parts page: parts_no, class, qty, part, form, commodity, datasheet,
/// authorized_only, shop, store, item, used, salvaged_from, authorized, status, since, passed_qa
/// (`gold.page_parts`).
struct PartsRow {
    parts_no: i64,
    class: String,
    qty: i64,
    part: Option<String>,
    form: Option<String>,
    commodity: Option<String>,
    datasheet: Option<String>,
    authorized_only: bool,
    shop: Option<String>,
    store: Option<String>,
    item: Option<String>,
    used: bool,
    salvaged_from: Option<String>,
    authorized: Option<String>,
    status: String,
    since: Option<String>,
    passed_qa: Option<String>,
}

/// `site/parts/`: everything the bench needs, each thing once, grouped by its class, with the
/// builds that need it, where the lab buys it and how far it has got.
pub fn render_parts(conn: &Connection) -> duckdb::Result<String> {
    let mut stmt = conn.prepare(
        "SELECT parts_no, class, qty, part, form, commodity, datasheet, authorized_only, shop, \
         store, item, used, salvaged_from, authorized, status, CAST(since AS TEXT), passed_qa \
         FROM gold.page_parts \
         ORDER BY class, parts_no",
    )?;
    let rows: Vec<PartsRow> = stmt
        .query_map([], |r| {
            Ok(PartsRow {
                parts_no: r.get(0)?,
                class: r.get(1)?,
                qty: r.get(2)?,
                part: r.get(3)?,
                form: r.get(4)?,
                commodity: r.get(5)?,
                datasheet: r.get(6)?,
                authorized_only: r.get(7)?,
                shop: r.get(8)?,
                store: r.get(9)?,
                item: r.get(10)?,
                used: r.get(11)?,
                salvaged_from: r.get(12)?,
                authorized: r.get(13)?,
                status: r.get(14)?,
                since: r.get(15)?,
                passed_qa: r.get(16)?,
            })
        })?
        .collect::<duckdb::Result<_>>()?;
    let mut stmt = conn.prepare(
        "SELECT parts_no, build, version FROM gold.page_parts_used_in \
         ORDER BY parts_no, build, version",
    )?;
    let mut used_in: std::collections::BTreeMap<i64, Vec<String>> = Default::default();
    for row in stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get(1)?, u32_at(r, 2)?)))? {
        let (n, build, version): (i64, String, u32) = row?;
        used_in.entry(n).or_default().push(format!(
            "<a href=\"../builds/{0}/v{1}/index.html\">{0} v{1}</a>",
            esc(&build),
            version
        ));
    }

    let mut h = String::from(
        "<h1 class=\"spec-title\">Parts</h1>\n<table class=\"spec-table parts-list\">\n<thead><tr><th>CLASS</th><th>QTY</th><th>PART</th><th>FORM</th><th>USED IN</th><th>SOURCE</th><th>STATUS</th></tr></thead>\n<tbody>\n",
    );
    for (i, p) in rows.iter().enumerate() {
        // A class's letter heads its group: the first row of each gets it, and the rest leave it
        // out.
        let first = i == 0 || rows[i - 1].class != p.class;
        let class = if first {
            let means = Class::ALL
                .into_iter()
                .find(|c| c.as_str() == p.class)
                .map_or("", Class::means);
            format!("<abbr title=\"{}\">{}</abbr>", esc(means), esc(&p.class))
        } else {
            String::new()
        };
        let what = match (&p.part, &p.commodity) {
            (Some(part), _) => match &p.datasheet {
                Some(url) => format!("<a href=\"{}\">{}</a>", esc(url), esc(part)),
                None => esc(part),
            },
            (None, Some(c)) => esc(c),
            (None, None) => String::new(),
        };
        // A shop's listing, a store with none, or the device a part came out of.
        let mut source = match (&p.salvaged_from, &p.shop) {
            (Some(donor), _) => format!("salvaged from {}", esc(donor)),
            (None, Some(shop)) => esc(shop_name(shop)),
            (None, None) => String::new(),
        };
        if let Some(store) = &p.store {
            if !source.is_empty() {
                source.push_str(" · ");
            }
            source.push_str(&esc(store));
        }
        if let Some(item) = &p.item {
            let _ = write!(source, " <span class=\"item\">{}</span>", esc(item));
        }
        if p.used {
            source.push_str(" · used");
        }
        // Where the maker lists the seller, or, for a part that needs one, that it does.
        let note = match &p.authorized {
            Some(at) => Some(format!("authorized seller: {}", esc(at))),
            None if p.authorized_only => Some("authorized sellers only".to_string()),
            None => None,
        };
        if let Some(note) = note {
            if !source.is_empty() {
                source.push_str("<br>");
            }
            let _ = write!(source, "<span class=\"muted\">{note}</span>");
        }
        let when = p.passed_qa.as_ref().or(p.since.as_ref());
        let status = match when {
            Some(w) => format!(
                "{} <span class=\"muted when\">{}</span>",
                esc(&p.status),
                esc(w)
            ),
            None => esc(&p.status),
        };
        // With no form, what it is spans both columns.
        let what = match &p.form {
            Some(form) => format!("<td>{what}</td><td>{}</td>", esc(form)),
            None => format!("<td colspan=\"2\">{what}</td>"),
        };
        let _ = writeln!(
            h,
            "<tr{}><td class=\"class\">{class}</td><td>{}</td>{what}<td>{}</td><td>{source}</td><td>{status}</td></tr>",
            if first { " class=\"group\"" } else { "" },
            p.qty,
            used_in
                .get(&p.parts_no)
                .map(|v| v.join(", "))
                .unwrap_or_default()
        );
    }
    h.push_str("</tbody>\n</table>\n");
    Ok(page("../", "Parts · Tentzhen", &h))
}

/// A value in its unit, with an SI prefix and up to three significant figures: 1e-6 m is "1 µm".
/// A count or ratio (unit `1`) is written whole above 1,000, grouped by thousands.
fn si(value: f64, unit: &str) -> String {
    fn sig(m: f64, figures: i32) -> String {
        let places = (figures - 1 - m.abs().log10().floor() as i32).max(0) as usize;
        let s = format!("{m:.places$}");
        if s.contains('.') {
            s.trim_end_matches('0').trim_end_matches('.').to_string()
        } else {
            s
        }
    }
    fn grouped(n: f64) -> String {
        let digits = format!("{:.0}", n.round());
        let mut out = String::new();
        for (i, c) in digits.chars().enumerate() {
            if i > 0 && (digits.len() - i) % 3 == 0 {
                out.push(',');
            }
            out.push(c);
        }
        out
    }
    match unit {
        "1" if value >= 1000.0 => grouped(value),
        "1" => sig(value, 3),
        "%" => format!("{} %", sig(value, 3)),
        _ => {
            const PREFIX: [&str; 10] = ["f", "p", "n", "µ", "m", "", "k", "M", "G", "T"];
            let mut e = ((value.log10() / 3.0).floor() as i32).clamp(-5, 4);
            let mut m = value / 10f64.powi(3 * e);
            // 999.96 rounds to 1000: move up a prefix.
            if sig(m, 3) == "1000" && e < 4 {
                e += 1;
                m /= 1000.0;
            }
            format!("{} {}{unit}", sig(m, 3), PREFIX[(e + 5) as usize])
        }
    }
}

/// The gap as a factor, in two significant figures: how many times better the industry's figure is.
fn gap(factor: f64) -> String {
    let two = |f: f64| {
        let e = f.log10().floor() as i32;
        si((f / 10f64.powi(e - 1)).round() * 10f64.powi(e - 1), "1")
    };
    if (0.9..=1.1).contains(&factor) {
        "none".into()
    } else if factor > 1.0 {
        format!("{}×", two(factor))
    } else {
        format!("Hacker Fab ahead, {}×", two(1.0 / factor))
    }
}

/// A side's source as a link: a URL as it is, and owner/repo@sha:path as the file on GitHub.
fn source_link(source: &str) -> (String, String) {
    if source.starts_with("https://") {
        let shown = source
            .trim_start_matches("https://")
            .split('/')
            .next()
            .unwrap_or(source)
            .trim_start_matches("www.")
            .to_string();
        return (source.to_string(), shown);
    }
    let (repo, rest) = source.split_once('@').unwrap_or((source, ""));
    let (sha, path) = rest.split_once(':').unwrap_or((rest, ""));
    (
        format!("https://github.com/{repo}/blob/{sha}/{path}"),
        format!("{repo} at {}", &sha[..sha.len().min(8)]),
    )
}

/// A side of the fab page, as `gold.page_fab` gives it.
struct FabSide {
    value: Option<f64>,
    is: Option<String>,
    tool: String,
    note: Option<String>,
    quote: String,
    source: String,
    read: String,
}

/// `site/fab/`: Hacker Fab's stack beside the industry's, row by row, from `gold.page_fab`.
pub fn render_fab(conn: &Connection) -> duckdb::Result<String> {
    let mut stmt = conn.prepare(
        "SELECT layer, measure, unit, gap, route, why_not, built_with, factor, \
         hf_value, hf_is, hf_tool, hf_note, hf_quote, hf_source, CAST(hf_read AS TEXT), \
         ind_value, ind_is, ind_tool, ind_note, ind_quote, ind_source, CAST(ind_read AS TEXT) \
         FROM gold.page_fab ORDER BY row_no",
    )?;
    type Row = (
        String,
        String,
        Option<String>,
        Option<String>,
        String,
        Option<String>,
        String,
        Option<f64>,
        FabSide,
        FabSide,
    );
    let side = |r: &duckdb::Row, at: usize| -> duckdb::Result<FabSide> {
        Ok(FabSide {
            value: r.get(at)?,
            is: r.get(at + 1)?,
            tool: r.get(at + 2)?,
            note: r.get(at + 3)?,
            quote: r.get(at + 4)?,
            source: r.get(at + 5)?,
            read: r.get(at + 6)?,
        })
    };
    let rows: Vec<Row> = stmt
        .query_map([], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
                r.get(6)?,
                r.get(7)?,
                side(r, 8)?,
                side(r, 15)?,
            ))
        })?
        .collect::<duckdb::Result<_>>()?;

    let mut h = String::new();
    h.push_str(
        "<p class=\"crumbs\">PROJECTS / THE OPEN FAB</p>\n\
         <p class=\"crumbs eyebrow\">HACKER FAB ADJACENT</p>\n\
         <h1 class=\"spec-title\">To TSMC and ASML, on a Hacker Fab budget</h1>\n\
         <p class=\"spec-does\">Hacker Fab's open stack beside the industry's, row by row, and the gap \
         between them. Every tool is engineered and built here, in Rust and CortenForge where they \
         can be.</p>\n\
         <p class=\"muted\">We build on Hacker Fab's open designs and send back what we add. We are \
         not part of Hacker Fab, and it has not reviewed this page. Safety comes first: tools on \
         mains are bought, listed and legal, and every step runs under its own safety plan \
         (<a href=\"https://github.com/via-balaena/tentzhen/blob/main/docs/fab.md\">fab.md</a>, \
         \"Safety first\"). Where a bought tool is what stops the next stage, making our own, at \
         a fraction of the cost and meeting every safety mark, is weighed.</p>\n\
         <table class=\"spec-table fab-gap\">\n\
         <thead><tr><th>MEASURE</th><th>HACKER FAB</th><th>INDUSTRY</th><th>GAP</th>\
         <th class=\"ours\">OUR ROUTE</th></tr></thead>\n<tbody>\n",
    );
    // Each source once: a quote cited by two rows keeps its first number.
    let mut sources: Vec<((String, String, String), String)> = Vec::new();
    let mut cell = |s: &FabSide, unit: Option<&str>| -> String {
        let shown = match (s.value, &s.is) {
            (Some(v), _) => si(v, unit.unwrap_or("1")),
            (None, Some(is)) => esc(is),
            (None, None) => String::new(),
        };
        let key = (s.quote.clone(), s.source.clone(), s.read.clone());
        let n = match sources.iter().position(|(k, _)| *k == key) {
            Some(i) => i + 1,
            None => {
                let (href, site) = source_link(&s.source);
                let n = sources.len() + 1;
                let li = format!(
                    "<li id=\"s{n}\">“{q}” <span class=\"muted\"><a href=\"{href}\">{site}</a>, read {r}</span></li>",
                    q = esc(&s.quote),
                    href = esc(&href),
                    site = esc(&site),
                    r = esc(&s.read)
                );
                sources.push((key, li));
                n
            }
        };
        let note = s
            .note
            .as_ref()
            .map(|n| format!(" · {}", esc(n)))
            .unwrap_or_default();
        format!(
            "{shown} <sup><a href=\"#s{n}\">{n}</a></sup><br><span class=\"muted\">{tool}{note}</span>",
            tool = esc(&s.tool)
        )
    };
    let mut layer = String::new();
    for (l, measure, unit, words, route, why_not, with, factor, hf, ind) in &rows {
        if *l != layer {
            let _ = writeln!(
                h,
                "<tr class=\"layer\"><td colspan=\"5\">{}</td></tr>",
                esc(&l.to_uppercase())
            );
            layer = l.clone();
        }
        let hf = cell(hf, unit.as_deref());
        let ind = cell(ind, unit.as_deref());
        let gap = match (factor, words) {
            (Some(f), _) => gap(*f),
            (None, Some(w)) => esc(w),
            (None, None) => String::new(),
        };
        let why = why_not
            .as_ref()
            .map(|w| format!(": {}", esc(w)))
            .unwrap_or_default();
        let _ = writeln!(
            h,
            "<tr><td>{}</td><td>{hf}</td><td>{ind}</td><td class=\"gap\">{gap}</td>\
             <td class=\"ours\">{}<br><span class=\"muted\">{}{why}</span></td></tr>",
            esc(measure),
            esc(route),
            esc(with)
        );
    }
    h.push_str("</tbody>\n</table>\n");
    h.push_str(
        "<p class=\"graded\"><span class=\"grade\">unknown</span>Every route is a proposal: \
         nothing on it is built or measured yet. The two sides are their sources' figures, \
         quoted below.</p>\n<h2>Sources</h2>\n<ol class=\"refs fab-sources\">\n",
    );
    for (_, li) in &sources {
        h.push_str(li);
        h.push('\n');
    }
    h.push_str("</ol>\n");
    Ok(page("../", "The open fab · Tentzhen", &h))
}

/// `site/legal/`: DISCLAIMER.md, as the warehouse holds it.
pub fn render_legal(markdown: &str) -> String {
    let mut body = String::from("<div class=\"prose\">\n");
    pulldown_cmark::html::push_html(&mut body, pulldown_cmark::Parser::new(markdown));
    body.push_str("</div>\n");
    page("../", "Legal · Tentzhen", &body)
}

/// Writes every generated file under `root/site/builds/`, `root/site/parts/`, `root/site/fab/` and
/// `root/site/legal/`. Returns how many it wrote.
pub fn write_site(conn: &Connection, root: &Path) -> Result<usize, String> {
    let out = root.join("site/builds");
    let write = |path: &Path, bytes: &[u8]| -> Result<(), String> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        fs::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()))
    };
    let pages = pages(conn).map_err(|e| e.to_string())?;
    let mut n = 0;
    let legal: String = conn
        .query_row("SELECT body FROM gold.legal", [], |r| r.get(0))
        .map_err(|e| format!("gold.legal: {e}"))?;
    write(
        &root.join("site/legal/index.html"),
        render_legal(&legal).as_bytes(),
    )?;
    n += 1;
    write(&out.join("index.html"), render_index(&pages).as_bytes())?;
    n += 1;
    let parts = render_parts(conn).map_err(|e| e.to_string())?;
    write(&root.join("site/parts/index.html"), parts.as_bytes())?;
    n += 1;
    let fab = render_fab(conn).map_err(|e| e.to_string())?;
    write(&root.join("site/fab/index.html"), fab.as_bytes())?;
    n += 1;
    for b in &pages {
        let dir = out.join(&b.build).join(format!("v{}", b.version));
        let html = render_version(conn, b).map_err(|e| e.to_string())?;
        write(&dir.join("index.html"), html.as_bytes())?;
        n += 1;
        if b.version == b.latest {
            write(
                &out.join(&b.build).join("index.html"),
                render_latest(&b.build, b.latest).as_bytes(),
            )?;
            n += 1;
        }
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_claim_s_numbers_fill_its_words() {
        let values = [
            ("min_cut_volts".to_string(), 3.393),
            ("max_cut_volts".to_string(), 3.61),
            ("divider_top_ohms".to_string(), 18200.0),
            ("resistor_tolerance_ratio".to_string(), 0.001),
        ];
        assert_eq!(
            fill(
                "between `min_cut_volts` and `max_cut_volts`, `divider_top_ohms` of `resistor_tolerance_ratio`; `unstated_volts`",
                &values
            ),
            "between 3.393 V and 3.61 V, 18200 Ω of 0.001; `unstated_volts`"
        );
    }
    use tentzhen_records::{Catalogue, Source};

    fn src(path: &str, text: &str) -> Source {
        Source {
            path: path.into(),
            text: text.into(),
        }
    }

    /// A trusted base with one entry, `entry`.
    fn base() -> Source {
        src(
            "trusted-base.toml",
            "[[entry]]\nid = \"entry\"\nassumes = \"x\"\n",
        )
    }

    fn build(name: &str, v: u32, extra: &str) -> Source {
        let changes = if v > 1 {
            "changes = \"swapped the wire\"\n"
        } else {
            ""
        };
        src(
            &format!("builds/{name}/v{v}.toml"),
            &format!(
                "build = \"{name}\"\nversion = {v}\nstatus = \"draft\"\ndoes = \"a thing\"\n{changes}{extra}"
            ),
        )
    }

    /// The records, through the warehouse, as the site sees them.
    fn site(builds: &[Source]) -> Connection {
        let cat = Catalogue::from_sources(&base(), &[], builds).unwrap();
        let mut conn = Connection::open_in_memory().unwrap();
        tentzhen_warehouse::load(&mut conn, &cat).unwrap();
        conn
    }

    fn render(conn: &Connection, name: &str, v: u32) -> String {
        let all = pages(conn).unwrap();
        let p = all
            .iter()
            .find(|p| p.build == name && p.version == v)
            .unwrap();
        render_version(conn, p).unwrap()
    }

    #[test]
    fn an_old_version_points_to_the_newest() {
        let conn = site(&[build("probe", 1, ""), build("probe", 2, "")]);
        let v1 = render(&conn, "probe", 1);
        let v2 = render(&conn, "probe", 2);
        assert!(
            v1.contains("Superseded by <a href=\"../v2/index.html\">v2</a>. swapped the wire"),
            "{v1}"
        );
        assert!(!v2.contains("Superseded"));
        assert!(render_latest("probe", 2).contains("url=v2/index.html"));
    }

    #[test]
    fn a_build_knows_where_it_is_used() {
        let bench = build("bench", 1, "[[uses]]\nbuild = \"probe\"\nversion = 1\n");
        let conn = site(&[build("probe", 1, ""), bench]);
        let page = render(&conn, "probe", 1);
        assert!(
            page.contains("<h2>Used in</h2>") && page.contains("bench v1"),
            "{page}"
        );
        assert!(render(&conn, "bench", 1).contains("<h2>Uses</h2>"));
    }

    /// The parts page lists each thing once, summed over the builds that write it, with where the
    /// lab buys it and how far it has got. A part sold by authorized sellers only says so until a
    /// source is chosen, and then where the maker lists that seller.
    #[test]
    fn the_parts_page_lists_each_thing_once() {
        let chip = |name: &str| {
            src(
                &format!("parts/{}.toml", name.to_lowercase()),
                &format!(
                    "part = \"{name}\"\nkind = \"chip\"\nis = \"x\"\nauthorized_only = true\n"
                ),
            )
        };
        let probe = build(
            "probe",
            1,
            "[[line]]\npart = \"RP2350\"\nclass = \"U\"\nqty = 1\n\n\
             [[line]]\ncommodity = \"wire\"\nclass = \"W\"\nqty = 6\n",
        );
        let bench = build(
            "bench",
            1,
            "[[uses]]\nbuild = \"probe\"\nversion = 1\n\n\
             [[line]]\npart = \"RP2350\"\nclass = \"U\"\nqty = 1\n\n\
             [[line]]\npart = \"INA239\"\nform = \"VSSOP-10\"\nclass = \"U\"\nqty = 1\n\n\
             [[line]]\ncommodity = \"glue\"\nclass = \"MP\"\nqty = 1\n\n\
             [[line]]\ncommodity = \"tape\"\nclass = \"MP\"\nqty = 1\n\n\
             [[line]]\ncommodity = \"cable\"\nclass = \"W\"\nqty = 1\n",
        );
        // One lab-log entry, as `tentzhen log append` writes it, and the INA239's incoming QA.
        let entry = format!(
            "{{\"seq\":1,\"at\":\"2026-10-14T10:00:00Z\",\"by\":\"person:jon\",\"what\":\"part.look\",\
             \"params\":{{}},\"result\":{{\"seen\":true}},\"limits_sha256\":\"{}\",\"prev\":null}}\n",
            "a".repeat(64)
        );
        let cat =
            Catalogue::from_sources(&base(), &[chip("RP2350"), chip("INA239")], &[probe, bench])
                .unwrap()
                .with_log(vec![src("lab/log/2026-10-14.jsonl", &entry)])
                .unwrap();
        let first = cat.log[0].sha256.clone();
        let qa = format!(
            "measures = \"x\"\nsetup = \"x\"\nlog = {{ first = \"{first}\", last = \"{first}\" }}\n\n\
             [[device]]\npart = \"INA239\"\n\n[results]\nseen = true\n"
        );
        let sourcing = "[[line]]\ncommodity = \"wire\"\nshop = \"amazon\"\nstore = \"Maker\"\n\
                        item = \"B0ABCDEFGH\"\nordered = \"2026-10-02\"\n\n\
                        [[line]]\npart = \"INA239\"\nform = \"VSSOP-10\"\nshop = \"lcsc\"\nitem = \"C2040\"\n\
                        authorized = \"ti.com distributors, read 2026-10-02\"\n\
                        arrived = \"2026-10-14\"\npassed_qa = \"2026-10-14-ina-incoming\"\n\n\
                        [[line]]\ncommodity = \"glue\"\nstore = \"Hardware Store\"\narrived = true\n\n\
                        [[line]]\ncommodity = \"tape\"\nshop = \"amazon\"\nstore = \"Maker\"\n\
                        item = \"B0ABCDEFGH\"\nused = true\nordered = true\n\n\
                        [[line]]\ncommodity = \"cable\"\nsalvaged_from = \"a dead printer\"\n\
                        store = \"Goodwill\"\narrived = true\n";
        let cat = cat
            .with_measurements(vec![src("lab/records/2026-10-14-ina-incoming.toml", &qa)])
            .unwrap()
            .with_sourcing(src("lab/sourcing.toml", sourcing))
            .unwrap();
        let mut conn = Connection::open_in_memory().unwrap();
        tentzhen_warehouse::load(&mut conn, &cat).unwrap();
        let page = render_parts(&conn).unwrap();
        assert!(page.contains("<thead><tr><th>CLASS</th><th>QTY</th><th>PART</th><th>FORM</th>"));
        // Grouped by class, in the letters' order, and in the bench's order within one; the letter
        // heads its group, and what has no form spans the form's column.
        let rows: Vec<&str> = page.lines().filter(|l| l.starts_with("<tr")).collect();
        assert_eq!(
            rows,
            [
                "<tr class=\"group\"><td class=\"class\"><abbr title=\"mechanical part\">MP</abbr></td>\
                 <td>1</td><td colspan=\"2\">glue</td><td>\
                 <a href=\"../builds/bench/v1/index.html\">bench v1</a></td>\
                 <td>Hardware Store</td><td>arrived</td></tr>",
                "<tr><td class=\"class\"></td><td>1</td><td colspan=\"2\">tape</td><td>\
                 <a href=\"../builds/bench/v1/index.html\">bench v1</a></td>\
                 <td>Amazon · Maker <span class=\"item\">B0ABCDEFGH</span> · used</td><td>ordered</td></tr>",
                "<tr class=\"group\"><td class=\"class\"><abbr title=\"integrated circuit\">U</abbr></td>\
                 <td>2</td><td colspan=\"2\">RP2350</td><td>\
                 <a href=\"../builds/bench/v1/index.html\">bench v1</a>, \
                 <a href=\"../builds/probe/v1/index.html\">probe v1</a></td>\
                 <td><span class=\"muted\">authorized sellers only</span></td><td>specced</td></tr>",
                "<tr><td class=\"class\"></td><td>1</td><td>INA239</td><td>VSSOP-10</td><td>\
                 <a href=\"../builds/bench/v1/index.html\">bench v1</a></td>\
                 <td>LCSC <span class=\"item\">C2040</span><br>\
                 <span class=\"muted\">authorized seller: ti.com distributors, read 2026-10-02</span></td>\
                 <td>passed incoming QA <span class=\"muted when\">2026-10-14-ina-incoming</span></td></tr>",
                "<tr class=\"group\"><td class=\"class\"><abbr title=\"wire or cable\">W</abbr></td>\
                 <td>1</td><td colspan=\"2\">cable</td><td>\
                 <a href=\"../builds/bench/v1/index.html\">bench v1</a></td>\
                 <td>salvaged from a dead printer · Goodwill</td><td>arrived</td></tr>",
                "<tr><td class=\"class\"></td><td>6</td><td colspan=\"2\">wire</td><td>\
                 <a href=\"../builds/probe/v1/index.html\">probe v1</a></td>\
                 <td>Amazon · Maker <span class=\"item\">B0ABCDEFGH</span></td>\
                 <td>ordered <span class=\"muted when\">2026-10-02</span></td></tr>",
            ]
        );
        assert!(page.contains("<a href=\"../parts/index.html\">PARTS</a>"));
    }

    #[test]
    fn a_value_is_written_with_its_si_prefix() {
        assert_eq!(si(1e-6, "m"), "1 µm");
        assert_eq!(si(13e-9, "m"), "13 nm");
        assert_eq!(si(2.5e-3, "s"), "2.5 ms");
        assert_eq!(si(0.99996, "s"), "1 s");
        assert_eq!(si(100.0, "W"), "100 W");
        assert_eq!(si(13.56e6, "Hz"), "13.6 MHz");
        assert_eq!(si(952.0, "1"), "952");
        assert_eq!(si(1.2, "1"), "1.2");
        assert_eq!(si(208e9, "1"), "208,000,000,000");
        assert_eq!(si(94.0, "%"), "94 %");
        assert_eq!(gap(76.923), "77×");
        assert_eq!(gap(1234.0), "1,200×");
        assert_eq!(gap(4.24), "4.2×");
        assert_eq!(gap(1.05), "none");
        assert_eq!(gap(0.25), "Hacker Fab ahead, 4×");
    }

    /// The fab page shows each side with its source, works the gap out from two values, keeps a
    /// row's own words when it has no values, and heads each layer once.
    #[test]
    fn the_fab_page_sets_each_side_beside_its_source() {
        let side = |v: &str, source: &str| {
            format!(
                "{v}\ntool = \"t\"\nquote = \"q <b>\"\nsource = \"{source}\"\nread = \"2026-11-25\"\n"
            )
        };
        let text = format!(
            "[[row]]\nlayer = \"Lithography\"\nmeasure = \"smallest line\"\nunit = \"m\"\n\
             better = \"lower\"\nroute = \"r\"\nwith = [\"Rust\", \"CortenForge\"]\n\
             [row.hacker_fab]\n{}\n[row.industry]\n{}\n\
             [[row]]\nlayer = \"Lithography\"\nmeasure = \"cleanroom\"\nroute = \"r2\"\n\
             with = [\"bought\"]\nwhy_not = \"rule 3\"\ngap = \"none needed\"\n\
             [row.hacker_fab]\n{}\n[row.industry]\n{}\n",
            side("value = 1e-6", "hacker-fab/gitbook@bd1eef57:a/b.md"),
            side("value = 13e-9", "https://www.asml.com/en/products"),
            side("is = \"none\"", "https://e.org/a"),
            side("is = \"ISO 1\"", "https://e.org/b"),
        );
        let cat = Catalogue::from_sources(&base(), &[], &[])
            .unwrap()
            .with_fab(src("fab/stack.toml", &text))
            .unwrap();
        let mut conn = Connection::open_in_memory().unwrap();
        tentzhen_warehouse::load(&mut conn, &cat).unwrap();
        let page = render_fab(&conn).unwrap();
        for want in [
            "1 µm <sup><a href=\"#s1\">1</a></sup>",
            "13 nm <sup><a href=\"#s2\">2</a></sup>",
            "<td class=\"gap\">77×</td>",
            "<td class=\"gap\">none needed</td>",
            "Rust · CortenForge</span>",
            "bought: rule 3</span>",
            "href=\"https://github.com/hacker-fab/gitbook/blob/bd1eef57/a/b.md\">hacker-fab/gitbook at bd1eef57</a>",
            ">asml.com</a>, read 2026-11-25",
            "<li id=\"s4\">“q &lt;b&gt;”",
            "HACKER FAB ADJACENT",
            "it has not reviewed this page",
            "meeting every safety mark",
        ] {
            assert!(page.contains(want), "the page lacks {want:?}:\n{page}");
        }
        assert_eq!(page.matches("<tr class=\"layer\">").count(), 1);
        // The same quote from the same source, on two rows, is listed once.
        let twice = text.replace(
            "is = \"ISO 1\"\ntool = \"t\"\nquote = \"q <b>\"\nsource = \"https://e.org/b\"",
            "is = \"ISO 1\"\ntool = \"t\"\nquote = \"q <b>\"\nsource = \"https://e.org/a\"",
        );
        assert_ne!(twice, text);
        let cat = Catalogue::from_sources(&base(), &[], &[])
            .unwrap()
            .with_fab(src("fab/stack.toml", &twice))
            .unwrap();
        let mut conn = Connection::open_in_memory().unwrap();
        tentzhen_warehouse::load(&mut conn, &cat).unwrap();
        let page = render_fab(&conn).unwrap();
        assert_eq!(page.matches("<li id=").count(), 3, "{page}");
        assert!(
            page.contains("ISO 1 <sup><a href=\"#s3\">3</a></sup>"),
            "{page}"
        );
        assert!(page.contains("href=\"../fab/index.html\">THE OPEN FAB</a>"));
    }

    /// The home page is written by hand, and carries the nav every generated page does.
    #[test]
    fn the_home_page_s_nav_is_the_generated_one() {
        let nav = |html: &str| {
            let start = html.find("<nav class=\"site-nav\"").expect("a nav");
            let end = start + html[start..].find("</nav>").expect("the nav's end");
            html[start..end].to_string()
        };
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let home = std::fs::read_to_string(root.join("site/index.html")).unwrap();
        assert_eq!(nav(&home), nav(&page("", "", "")));
    }

    #[test]
    fn a_walkthrough_carries_the_risk_notice() {
        let steps = "[[step]]\ndo = \"plug it in\"\n";
        let conn = site(&[build("probe", 1, steps)]);
        let page = render(&conn, "probe", 1);
        assert!(
            page.contains("<p class=\"notice\">At your own risk"),
            "{page}"
        );
        assert!(page.contains("href=\"../../../legal/index.html\""));
        assert!(render_legal("# Disclaimer\n\nAS IS.").contains("<h1>Disclaimer</h1>"));
    }
}
