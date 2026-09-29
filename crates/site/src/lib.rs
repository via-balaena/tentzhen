//! Generates tentzhen.com's build pages from the warehouse's gold views, and nothing else.
//!
//! The records in `parts/` and `builds/` are loaded into DuckDB (crates/warehouse); every value on
//! a page is read back out of a `gold.page*` view. Every build version gets a permanent page at
//! `site/builds/<name>/v<N>/`; `site/builds/<name>/` sends you to the newest. Pages are output,
//! never edited: the Quality Gate reruns this and fails if `site/` differs.

use duckdb::{Connection, params};
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

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
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Chakra+Petch:wght@500;600&amp;family=IBM+Plex+Mono:wght@400;500&amp;family=IBM+Plex+Sans:wght@300;400;500&amp;display=swap">
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
    <a href="https://github.com/via-balaena/tentzhen/blob/main/docs/vision.md">VISION</a>
    <a href="https://github.com/via-balaena/tentzhen/blob/main/docs/scope.md">INSTRUMENT</a>
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
    <span>MIT OR APACHE-2.0</span>
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
/// A claim: says, sim grade, sim evidence, irl grade, irl evidence (`gold.claim_grades`).
type ClaimRow = (String, String, Option<String>, String, Option<String>);

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
        h.push_str("<h2>Walkthrough</h2>\n<ol class=\"steps\">\n");
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
        "SELECT says, sim_grade, sim_evidence, irl_grade, irl_evidence FROM gold.claim_grades \
         WHERE build = ? AND version = ? ORDER BY claim_no",
    )?;
    let claims: Vec<ClaimRow> = stmt
        .query_map(key, |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })?
        .collect::<duckdb::Result<_>>()?;
    if !claims.is_empty() {
        h.push_str("<h2>Claims</h2>\n<table class=\"spec-table claims\">\n<thead><tr><th>CLAIM</th><th>SIMULATION</th><th>ON THE BENCH</th></tr></thead>\n<tbody>\n");
        for (says, sim_grade, sim_evidence, irl_grade, irl_evidence) in &claims {
            let evidence = |e: &Option<String>| {
                e.as_ref()
                    .map(|e| format!(" <span class=\"muted\">{}</span>", esc(e)))
                    .unwrap_or_default()
            };
            let _ = writeln!(
                h,
                "<tr><td>{}</td><td>{}{}</td><td>{}{}</td></tr>",
                esc(says),
                chip(sim_grade),
                evidence(sim_evidence),
                chip(irl_grade),
                evidence(irl_evidence)
            );
        }
        h.push_str("</tbody>\n</table>\n");
    }

    Ok(page(up, &format!("{name} v{} · Tentzhen", b.version), &h))
}

/// Writes every generated file under `root/site/builds/`. Returns how many it wrote.
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
    write(&out.join("index.html"), render_index(&pages).as_bytes())?;
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
    use tentzhen_records::{Catalogue, Source};

    fn src(path: &str, text: &str) -> Source {
        Source {
            path: path.into(),
            text: text.into(),
        }
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
        let cat = Catalogue::from_sources(&[], builds).unwrap();
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
}
