//! The catalogue's records: parts and versioned builds, parsed from `parts/` and `builds/` and
//! checked against the contract every consumer relies on (the site, the warehouse).

use serde::Deserialize;
use std::collections::BTreeMap;
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
    /// The file this came from, relative to the repo root. Set on load, never written in a record.
    #[serde(skip)]
    pub record: String,
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
    /// The file this came from, relative to the repo root. Set on load, never written in a record.
    #[serde(skip)]
    pub record: String,
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
    #[serde(default = "one")]
    pub qty: u32,
}

fn one() -> u32 {
    1
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

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Chip => "chip",
            Kind::Module => "module",
            Kind::Product => "product",
        }
    }
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Draft => "draft",
            Status::Published => "published",
        }
    }
}

impl SimGrade {
    pub fn as_str(self) -> &'static str {
        match self {
            SimGrade::Tested => "tested",
            SimGrade::Checked => "checked",
            SimGrade::Proven => "proven",
        }
    }
}

impl Claim {
    pub fn sim_grade(&self) -> &'static str {
        self.sim.as_ref().map_or("unknown", |s| s.grade.as_str())
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
#[derive(Debug, Clone)]
pub struct Source {
    pub path: String,
    pub text: String,
}

pub struct Catalogue {
    pub parts: BTreeMap<String, Part>,
    /// Each build's versions, oldest first; `versions[i].version == i + 1`.
    pub builds: BTreeMap<String, Vec<Build>>,
    /// Every file exactly as read, records then drawings then site documents then the lab's
    /// limits: the warehouse's bronze layer.
    pub sources: Vec<Source>,
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
        let mut drawings = Vec::new();
        for (name, versions) in &cat.builds {
            for b in versions {
                if let Some(d) = &b.drawing {
                    let path = root.join("builds").join(name).join(d);
                    let text = fs::read_to_string(&path)
                        .map_err(|e| format!("{}: {e}", path.display()))?;
                    drawings.push(Source {
                        path: format!("builds/{name}/{d}"),
                        text,
                    });
                }
            }
        }
        cat.sources.extend(drawings);
        // Site documents go through the warehouse like everything else the site shows.
        let disclaimer = root.join("DISCLAIMER.md");
        let text = fs::read_to_string(&disclaimer)
            .map_err(|e| format!("{}: {e}", disclaimer.display()))?;
        cat.sources.push(Source {
            path: "DISCLAIMER.md".into(),
            text,
        });
        // The lab's limits, checked on the way in, so bronze holds their sha256 for lineage.
        let (_, text) = tentzhen_lab::Limits::load(root)?;
        cat.sources.push(Source {
            path: tentzhen_lab::LIMITS.into(),
            text,
        });
        Ok(cat)
    }

    pub fn from_sources(part_sources: &[Source], build_sources: &[Source]) -> Result<Self, String> {
        let mut parts = BTreeMap::new();
        for s in part_sources {
            let mut p: Part = toml::from_str(&s.text).map_err(|e| format!("{}: {e}", s.path))?;
            p.record = s.path.clone();
            if parts.contains_key(&p.part) {
                return Err(format!("{}: part {} is defined twice", s.path, p.part));
            }
            parts.insert(p.part.clone(), p);
        }

        let mut builds: BTreeMap<String, Vec<Build>> = BTreeMap::new();
        for s in build_sources {
            let mut b: Build = toml::from_str(&s.text).map_err(|e| format!("{}: {e}", s.path))?;
            b.record = s.path.clone();
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
            sources: part_sources.iter().chain(build_sources).cloned().collect(),
        };
        for versions in cat.builds.values() {
            for b in versions {
                cat.check_build(b)?;
            }
        }
        cat.check_no_cycles()?;
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
        if b.uses.iter().any(|u| u.qty == 0) {
            return Err(format!("{at}: uses a build with qty 0"));
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

    /// A build may not use itself through any chain of `uses`: a parts explosion must end.
    fn check_no_cycles(&self) -> Result<(), String> {
        fn visit<'a>(
            cat: &'a Catalogue,
            b: &'a Build,
            path: &mut Vec<(&'a str, u32)>,
        ) -> Result<(), String> {
            let key = (b.build.as_str(), b.version);
            if path.contains(&key) {
                let chain: Vec<String> = path.iter().map(|(n, v)| format!("{n} v{v}")).collect();
                return Err(format!(
                    "uses loop: {} -> {} v{}",
                    chain.join(" -> "),
                    key.0,
                    key.1
                ));
            }
            path.push(key);
            for u in &b.uses {
                if let Some(next) = cat.version(&u.build, u.version) {
                    visit(cat, next, path)?;
                }
            }
            path.pop();
            Ok(())
        }
        for b in self.builds.values().flatten() {
            visit(self, b, &mut Vec::new())?;
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
    fn a_uses_loop_is_refused() {
        let a = build("a", 1, "[[uses]]\nbuild = \"b\"\nversion = 1\n");
        let b = build("b", 1, "[[uses]]\nbuild = \"a\"\nversion = 1\n");
        let err = Catalogue::from_sources(&[], &[a, b]).err().unwrap();
        assert!(err.contains("uses loop"), "{err}");
    }

    #[test]
    fn uses_counts_one_unless_told() {
        let bench = build("bench", 1, "[[uses]]\nbuild = \"probe\"\nversion = 1\n");
        let cat = Catalogue::from_sources(&[], &[build("probe", 1, ""), bench]).unwrap();
        assert_eq!(cat.version("bench", 1).unwrap().uses[0].qty, 1);
    }

    #[test]
    fn using_a_missing_version_is_refused() {
        let bench = build("bench", 1, "[[uses]]\nbuild = \"probe\"\nversion = 2\n");
        assert!(Catalogue::from_sources(&[], &[build("probe", 1, ""), bench]).is_err());
    }
}
