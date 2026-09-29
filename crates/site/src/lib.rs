//! Generates tentzhen.com's build pages from the records in `parts/` and `builds/`.
//!
//! A build version is `builds/<name>/v<N>.toml`. Every version gets a permanent page at
//! `site/builds/<name>/v<N>/`; `site/builds/<name>/` sends you to the newest. Pages are output,
//! never edited: the Quality Gate reruns this and fails if `site/` differs.

use serde::Deserialize;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Part {
    pub part: String,
    pub kind: Kind,
    pub is: String,
    #[serde(default)]
    pub authorized_only: bool,
    pub datasheet: Option<String>,
}

/// What a part is at its lowest level. Commodities (wire, solder) are not parts: a build names
/// them by their properties on the line itself.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Chip,
    Module,
    Product,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Draft,
    Published,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Build {
    pub build: String,
    pub version: u32,
    pub status: Status,
    pub does: String,
    /// What changed from the previous version. Required from v2 on.
    pub changes: Option<String>,
    pub drawing: Option<String>,
    #[serde(default)]
    pub line: Vec<Line>,
    #[serde(default)]
    pub uses: Vec<Uses>,
    pub firmware: Option<Firmware>,
    #[serde(default)]
    pub pin: Vec<Pin>,
    #[serde(default)]
    pub step: Vec<Step>,
    #[serde(default)]
    pub claim: Vec<Claim>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Line {
    pub part: Option<String>,
    pub commodity: Option<String>,
    pub form: Option<String>,
    pub qty: u32,
}

/// Another build this one is made from, pinned to a version.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Uses {
    pub build: String,
    pub version: u32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Firmware {
    pub name: String,
    pub license: String,
    pub source: String,
    pub release: String,
    pub file: String,
    pub sha256: String,
    pub pin_map: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pin {
    pub name: String,
    pub board_pin: u32,
    pub net: String,
    pub required: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Step {
    pub r#do: String,
    pub run: Option<String>,
    pub expect: Option<String>,
    pub agent: Option<String>,
}

/// A claim and its referents. The grade shown is derived from these, never typed.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Claim {
    pub says: String,
    pub sim: Option<Sim>,
    pub irl: Option<Irl>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sim {
    pub grade: SimGrade,
    /// The test, bounded check or proof that shows it.
    pub by: String,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SimGrade {
    Tested,
    Checked,
    Proven,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Irl {
    /// The lab measurement record that shows it.
    pub record: String,
}

impl Claim {
    pub fn sim_grade(&self) -> &'static str {
        match self.sim.as_ref().map(|s| s.grade) {
            Some(SimGrade::Tested) => "tested",
            Some(SimGrade::Checked) => "checked",
            Some(SimGrade::Proven) => "proven",
            None => "unknown",
        }
    }

    pub fn irl_grade(&self) -> &'static str {
        if self.irl.is_some() {
            "measured"
        } else {
            "unknown"
        }
    }
}

/// A record's path relative to the repo root, and its text.
pub struct Source {
    pub path: String,
    pub text: String,
}

pub struct Catalogue {
    pub parts: BTreeMap<String, Part>,
    /// Each build's versions, oldest first; `versions[i].version == i + 1`.
    pub builds: BTreeMap<String, Vec<Build>>,
    /// Drawing SVGs by `<build>/v<N>`, inlined into pages so they draw in the site's fonts.
    pub drawings: BTreeMap<String, String>,
}

impl Catalogue {
    pub fn load(root: &Path) -> Result<Self, String> {
        let mut parts = Vec::new();
        for entry in read_dir_sorted(&root.join("parts"))? {
            if entry.extension().is_some_and(|e| e == "toml") {
                parts.push(read_source(root, &entry)?);
            }
        }
        let mut builds = Vec::new();
        for dir in read_dir_sorted(&root.join("builds"))? {
            if !dir.is_dir() {
                continue;
            }
            for entry in read_dir_sorted(&dir)? {
                if entry.extension().is_some_and(|e| e == "toml") {
                    builds.push(read_source(root, &entry)?);
                }
            }
        }
        let mut cat = Self::from_sources(&parts, &builds)?;
        for (name, versions) in &cat.builds {
            for b in versions {
                if let Some(d) = &b.drawing {
                    let path = root.join("builds").join(name).join(d);
                    let svg = fs::read_to_string(&path)
                        .map_err(|e| format!("{}: {e}", path.display()))?;
                    cat.drawings.insert(format!("{name}/v{}", b.version), svg);
                }
            }
        }
        Ok(cat)
    }

    pub fn from_sources(part_sources: &[Source], build_sources: &[Source]) -> Result<Self, String> {
        let mut parts = BTreeMap::new();
        for s in part_sources {
            let p: Part = toml::from_str(&s.text).map_err(|e| format!("{}: {e}", s.path))?;
            if parts.contains_key(&p.part) {
                return Err(format!("{}: part {} is defined twice", s.path, p.part));
            }
            parts.insert(p.part.clone(), p);
        }

        let mut builds: BTreeMap<String, Vec<Build>> = BTreeMap::new();
        for s in build_sources {
            let b: Build = toml::from_str(&s.text).map_err(|e| format!("{}: {e}", s.path))?;
            let expected = format!("builds/{}/v{}.toml", b.build, b.version);
            if s.path != expected {
                return Err(format!(
                    "{}: says build {} v{}, so it belongs at {expected}",
                    s.path, b.build, b.version
                ));
            }
            builds.entry(b.build.clone()).or_default().push(b);
        }

        for (name, versions) in &mut builds {
            versions.sort_by_key(|b| b.version);
            for (i, b) in versions.iter().enumerate() {
                let want = u32::try_from(i + 1).map_err(|e| e.to_string())?;
                if b.version != want {
                    return Err(format!(
                        "{name}: versions must run v1, v2, … without gaps; v{want} is missing"
                    ));
                }
                if b.version > 1 && b.changes.is_none() {
                    return Err(format!(
                        "{name} v{}: says nothing under `changes`",
                        b.version
                    ));
                }
            }
        }

        let cat = Catalogue {
            parts,
            builds,
            drawings: BTreeMap::new(),
        };
        for versions in cat.builds.values() {
            for b in versions {
                cat.check_build(b)?;
            }
        }
        Ok(cat)
    }

    fn check_build(&self, b: &Build) -> Result<(), String> {
        let at = format!("{} v{}", b.build, b.version);
        for l in &b.line {
            match (&l.part, &l.commodity) {
                (Some(p), None) if !self.parts.contains_key(p) => {
                    return Err(format!("{at}: part {p} has no record in parts/"));
                }
                (Some(_), None) | (None, Some(_)) => {}
                _ => {
                    return Err(format!(
                        "{at}: a line names exactly one of `part` or `commodity`"
                    ));
                }
            }
            if l.qty == 0 {
                return Err(format!("{at}: a line has qty 0"));
            }
        }
        for u in &b.uses {
            if u.build == b.build {
                return Err(format!("{at}: a build cannot use itself"));
            }
            if self.version(&u.build, u.version).is_none() {
                return Err(format!(
                    "{at}: uses {} v{}, which does not exist",
                    u.build, u.version
                ));
            }
        }
        Ok(())
    }

    pub fn version(&self, name: &str, version: u32) -> Option<&Build> {
        let i = usize::try_from(version.checked_sub(1)?).ok()?;
        self.builds.get(name)?.get(i)
    }

    /// Every build version that uses `name` v`version`.
    pub fn used_in(&self, name: &str, version: u32) -> Vec<&Build> {
        self.builds
            .values()
            .flatten()
            .filter(|b| {
                b.uses
                    .iter()
                    .any(|u| u.build == name && u.version == version)
            })
            .collect()
    }
}

fn read_dir_sorted(dir: &Path) -> Result<Vec<std::path::PathBuf>, String> {
    let mut out: Vec<_> = fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .map(|e| e.map(|e| e.path()))
        .collect::<Result<_, _>>()
        .map_err(|e| format!("{}: {e}", dir.display()))?;
    out.sort();
    Ok(out)
}

fn read_source(root: &Path, path: &Path) -> Result<Source, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let rel = path.strip_prefix(root).map_err(|e| e.to_string())?;
    let rel = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/");
    Ok(Source { path: rel, text })
}

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
                s = status_word(b.status),
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

fn status_word(s: Status) -> &'static str {
    match s {
        Status::Draft => "draft",
        Status::Published => "published",
    }
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
        s = status_word(b.status)
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

    fn src(path: &str, text: &str) -> Source {
        Source {
            path: path.into(),
            text: text.into(),
        }
    }

    const PART: &str = "part = \"RP2350\"\nkind = \"chip\"\nis = \"microcontroller\"\n";

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
    fn the_repo_records_load() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let cat = Catalogue::load(&root).expect("records load");
        let probe = cat
            .version("debug-probe", 1)
            .expect("debug-probe v1 exists");
        assert!(
            probe
                .line
                .iter()
                .any(|l| l.part.as_deref() == Some("RP2350"))
        );
    }

    #[test]
    fn grades_come_from_referents() {
        let none: Claim = toml::from_str("says = \"x\"").unwrap();
        assert_eq!((none.sim_grade(), none.irl_grade()), ("unknown", "unknown"));
        let both: Claim =
            toml::from_str("says = \"x\"\nsim = { grade = \"checked\", by = \"sby bmc 20\" }\nirl = { record = \"lab-0001\" }").unwrap();
        assert_eq!(
            (both.sim_grade(), both.irl_grade()),
            ("checked", "measured")
        );
    }

    #[test]
    fn an_unrecorded_part_is_refused() {
        let b = build("probe", 1, "[[line]]\npart = \"RP9999\"\nqty = 1\n");
        let err = Catalogue::from_sources(&[src("parts/rp2350.toml", PART)], &[b])
            .err()
            .unwrap();
        assert!(err.contains("RP9999"), "{err}");
    }

    #[test]
    fn a_file_must_sit_at_its_version() {
        let mut b = build("probe", 1, "");
        b.path = "builds/probe/v2.toml".into();
        assert!(Catalogue::from_sources(&[], &[b]).is_err());
    }

    #[test]
    fn versions_have_no_gaps() {
        let err = Catalogue::from_sources(&[], &[build("probe", 1, ""), build("probe", 3, "")])
            .err()
            .unwrap();
        assert!(err.contains("v2 is missing"), "{err}");
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

    #[test]
    fn using_a_missing_version_is_refused() {
        let bench = build("bench", 1, "[[uses]]\nbuild = \"probe\"\nversion = 2\n");
        assert!(Catalogue::from_sources(&[], &[build("probe", 1, ""), bench]).is_err());
    }
}
