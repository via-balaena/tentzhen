//! Generates tentzhen.com's build pages from the records in `parts/` and `builds/`.
//!
//! A build version is `builds/<name>/v<N>.toml`. Every version gets a permanent page at
//! `site/builds/<name>/v<N>/`; `site/builds/<name>/` sends you to the newest. Pages are output,
//! never edited: the Quality Gate reruns this and fails if `site/` differs.

use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use tentzhen_records::{Build, Catalogue};

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

pub fn render_index(cat: &Catalogue) -> String {
    let mut body = String::from(
        "<h1 class=\"spec-title\">Builds</h1>\n<table class=\"spec-table\">\n<thead><tr><th>BUILD</th><th>LATEST</th><th>DOES</th></tr></thead>\n<tbody>\n",
    );
    for (name, versions) in &cat.builds {
        if let Some(b) = versions.last() {
            let _ = writeln!(
                body,
                "<tr><td><a href=\"{n}/v{v}/index.html\">{n}</a></td><td>v{v} · {s}</td><td>{d}</td></tr>",
                n = esc(name),
                v = b.version,
                s = b.status.as_str(),
                d = esc(&b.does)
            );
        }
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

pub fn render_version(cat: &Catalogue, b: &Build) -> String {
    let up = "../../../";
    let versions = &cat.builds[&b.build];
    let latest = versions.len();
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
    for v in versions {
        if v.version == b.version {
            let _ = write!(links, " <span class=\"here\">v{}</span>", v.version);
        } else {
            let _ = write!(links, " <a href=\"../v{0}/index.html\">v{0}</a>", v.version);
        }
    }
    let _ = writeln!(
        h,
        "<p class=\"spec-meta\"><span class=\"state {s}\">v{} · {s}</span><span class=\"versions\">versions{links}</span></p>",
        b.version,
        s = b.status.as_str()
    );
    if usize::try_from(b.version).is_ok_and(|v| v < latest) {
        let newest = &versions[latest - 1];
        let _ = writeln!(
            h,
            "<p class=\"banner\">Superseded by <a href=\"../v{0}/index.html\">v{0}</a>. {1}</p>",
            newest.version,
            esc(newest.changes.as_deref().unwrap_or(""))
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
    if let Some(svg) = cat.drawings.get(&format!("{}/v{}", b.build, b.version)) {
        let _ = writeln!(
            h,
            "<figure class=\"drawing\" role=\"img\" aria-label=\"Drawing of {name} v{}\">\n{}\n</figure>",
            b.version,
            svg.trim_end()
        );
    }

    if !b.line.is_empty() {
        h.push_str("<h2>Parts</h2>\n<table class=\"spec-table\">\n<thead><tr><th>QTY</th><th>PART</th><th>FORM</th><th></th></tr></thead>\n<tbody>\n");
        for l in &b.line {
            let (what, note) = match (&l.part, &l.commodity) {
                (Some(p), _) => {
                    let part = &cat.parts[p];
                    let what = match &part.datasheet {
                        Some(url) => format!("<a href=\"{}\">{}</a>", esc(url), esc(p)),
                        None => esc(p),
                    };
                    (
                        what,
                        if part.authorized_only {
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
                "<tr><td>{}</td><td>{what}</td><td>{}</td><td class=\"muted\">{note}</td></tr>",
                l.qty,
                esc(l.form.as_deref().unwrap_or(""))
            );
        }
        h.push_str("</tbody>\n</table>\n");
    }

    if !b.uses.is_empty() {
        h.push_str("<h2>Uses</h2>\n<ul class=\"refs\">\n");
        for u in &b.uses {
            let _ = writeln!(
                h,
                "<li><a href=\"../../{0}/v{1}/index.html\">{0} v{1}</a></li>",
                esc(&u.build),
                u.version
            );
        }
        h.push_str("</ul>\n");
    }
    let used = cat.used_in(&b.build, b.version);
    if !used.is_empty() {
        h.push_str("<h2>Used in</h2>\n<ul class=\"refs\">\n");
        for u in used {
            let _ = writeln!(
                h,
                "<li><a href=\"../../{0}/v{1}/index.html\">{0} v{1}</a></li>",
                esc(&u.build),
                u.version
            );
        }
        h.push_str("</ul>\n");
    }

    if let Some(f) = &b.firmware {
        h.push_str("<h2>Firmware</h2>\n<dl class=\"facts\">\n");
        let _ = writeln!(
            h,
            "<div><dt>NAME</dt><dd><a href=\"{}\">{}</a> · {}</dd></div>",
            esc(&f.source),
            esc(&f.name),
            esc(&f.license)
        );
        let _ = writeln!(h, "<div><dt>RELEASE</dt><dd>{}</dd></div>", esc(&f.release));
        let _ = writeln!(h, "<div><dt>FILE</dt><dd>{}</dd></div>", esc(&f.file));
        let _ = writeln!(
            h,
            "<div><dt>SHA-256</dt><dd class=\"hash\">{}</dd></div>",
            esc(&f.sha256)
        );
        if let Some(p) = &f.pin_map {
            let _ = writeln!(h, "<div><dt>PIN MAP</dt><dd>{}</dd></div>", esc(p));
        }
        h.push_str("</dl>\n");
    }

    if !b.pin.is_empty() {
        h.push_str("<h2>Pins</h2>\n<table class=\"spec-table\">\n<thead><tr><th>PIN</th><th>BOARD PIN</th><th>NET</th><th></th></tr></thead>\n<tbody>\n");
        for p in &b.pin {
            let _ = writeln!(
                h,
                "<tr><td>{}</td><td>{}</td><td>{}</td><td class=\"muted\">{}</td></tr>",
                esc(&p.name),
                p.board_pin,
                esc(&p.net),
                if p.required { "required" } else { "optional" }
            );
        }
        h.push_str("</tbody>\n</table>\n");
    }

    if !b.step.is_empty() {
        h.push_str("<h2>Walkthrough</h2>\n<ol class=\"steps\">\n");
        for s in &b.step {
            let _ = write!(h, "<li><p>{}</p>", esc(&s.r#do));
            if let Some(r) = &s.run {
                let _ = write!(h, "<pre><code>{}</code></pre>", esc(r));
            }
            if let Some(e) = &s.expect {
                let _ = write!(h, "<p class=\"expect\"><span>EXPECT</span>{}</p>", esc(e));
            }
            if let Some(a) = &s.agent {
                let _ = write!(h, "<p class=\"agent\"><span>AGENTS</span>{}</p>", esc(a));
            }
            h.push_str("</li>\n");
        }
        h.push_str("</ol>\n");
    }

    if !b.claim.is_empty() {
        h.push_str("<h2>Claims</h2>\n<table class=\"spec-table claims\">\n<thead><tr><th>CLAIM</th><th>SIMULATION</th><th>ON THE BENCH</th></tr></thead>\n<tbody>\n");
        for c in &b.claim {
            let sim_ref = c
                .sim
                .as_ref()
                .map(|s| format!(" <span class=\"muted\">{}</span>", esc(&s.by)))
                .unwrap_or_default();
            let irl_ref = c
                .irl
                .as_ref()
                .map(|i| format!(" <span class=\"muted\">{}</span>", esc(&i.record)))
                .unwrap_or_default();
            let _ = writeln!(
                h,
                "<tr><td>{}</td><td>{}{sim_ref}</td><td>{}{irl_ref}</td></tr>",
                esc(&c.says),
                chip(c.sim_grade()),
                chip(c.irl_grade())
            );
        }
        h.push_str("</tbody>\n</table>\n");
    }

    page(up, &format!("{name} v{} · Tentzhen", b.version), &h)
}

/// Writes every generated file under `root/site/builds/`. Returns how many it wrote.
pub fn write_site(cat: &Catalogue, root: &Path) -> Result<usize, String> {
    let out = root.join("site/builds");
    let write = |path: &Path, bytes: &[u8]| -> Result<(), String> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        fs::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()))
    };
    let mut n = 0;
    write(&out.join("index.html"), render_index(cat).as_bytes())?;
    n += 1;
    for (name, versions) in &cat.builds {
        for b in versions {
            let dir = out.join(name).join(format!("v{}", b.version));
            write(&dir.join("index.html"), render_version(cat, b).as_bytes())?;
            n += 1;
        }
        let latest = u32::try_from(versions.len()).map_err(|e| e.to_string())?;
        write(
            &out.join(name).join("index.html"),
            render_latest(name, latest).as_bytes(),
        )?;
        n += 1;
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tentzhen_records::Source;

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

    #[test]
    fn an_old_version_points_to_the_newest() {
        let cat =
            Catalogue::from_sources(&[], &[build("probe", 1, ""), build("probe", 2, "")]).unwrap();
        let v1 = render_version(&cat, cat.version("probe", 1).unwrap());
        let v2 = render_version(&cat, cat.version("probe", 2).unwrap());
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
        let cat = Catalogue::from_sources(&[], &[build("probe", 1, ""), bench]).unwrap();
        let page = render_version(&cat, cat.version("probe", 1).unwrap());
        assert!(
            page.contains("<h2>Used in</h2>") && page.contains("bench v1"),
            "{page}"
        );
    }
}
