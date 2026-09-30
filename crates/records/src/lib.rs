//! The catalogue's records: parts and versioned builds, parsed from `parts/` and `builds/` and
//! checked against the contract every consumer relies on (the site, the warehouse). Both can carry
//! claims, in one shape: [`Claim`]. A claim assumed rather than shown names an entry in the trusted
//! base, `trusted-base.toml`: [`Trusted`]. The lab log, every hardware action, is [`log`], and a
//! claim's `record` names a measurement record: [`measurement`]. The boards an agent may flash are
//! the lab-target list: [`target`].

pub mod log;
pub mod measurement;
pub mod target;

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
    #[serde(default)]
    pub claim: Vec<Claim>,
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
    /// Why no board of this build, or of a build that uses it, may be a lab target: the lab-target
    /// list refuses one ([`target`]).
    pub never_a_lab_target: Option<String>,
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

/// A claim on a part or a build version, and its referents. Its grades come from which referents
/// are present, and there is no field to type one: `proof`, `check` and `test` give proven, checked
/// and tested; `record` gives measured, and `trusted` gives trusted.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Claim {
    /// Unique in its record. The claim is cited as `<part>#<id>` or `<build>/v<n>#<id>`.
    pub id: String,
    pub says: String,
    /// A proof that shows it (SBY `prove`, Verus), with its tool.
    pub proof: Option<String>,
    /// A bounded check that shows it (SBY `bmc`, Kani), with its bound.
    pub check: Option<String>,
    /// A test that shows it, in simulation or on the host.
    pub test: Option<String>,
    /// The id of the measurement record that shows it on the bench: [`measurement::Measurement`].
    pub record: Option<String>,
    /// The id of the entry in the trusted base that assumes it: [`Trusted`].
    pub trusted: Option<String>,
    /// Numbers the claim states. Each key ends in its unit: [`VALUE_UNITS`].
    #[serde(default)]
    pub values: BTreeMap<String, f64>,
}

/// The unit a claim's value may name, as the last part of its key (`min_volts`, `input_ratio`).
/// `codes` is a raw ADC code, before it is converted. `kelvin` is a temperature or a span of one.
pub const VALUE_UNITS: [&str; 9] = [
    "volts", "amps", "ohms", "watts", "hertz", "seconds", "ratio", "codes", "kelvin",
];

impl Claim {
    /// Its referents by kind, as the warehouse stores them.
    pub fn referents(&self) -> impl Iterator<Item = (&'static str, &str)> {
        [
            ("proof", &self.proof),
            ("check", &self.check),
            ("test", &self.test),
            ("record", &self.record),
            ("trusted", &self.trusted),
        ]
        .into_iter()
        .filter_map(|(kind, r)| r.as_deref().map(|r| (kind, r)))
    }
}

/// Where the trusted base lives, relative to the repo root.
pub const TRUSTED_BASE: &str = "trusted-base.toml";

/// An entry in the trusted base: something assumed, not shown. A claim that rests on it names its
/// `id` as its `trusted` referent, and a `trusted` that names no entry is refused.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Trusted {
    pub id: String,
    pub assumes: String,
    /// The file this came from, relative to the repo root. Set on load, never written in a record.
    #[serde(skip)]
    pub record: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TrustedBase {
    #[serde(default)]
    entry: Vec<Trusted>,
}

/// A key in snake case: lowercase words joined by underscores, starting with a letter.
fn snake(key: &str) -> bool {
    key.starts_with(|ch: char| ch.is_ascii_lowercase())
        && key.split('_').all(|word| !word.is_empty())
        && key
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
}

/// The unit a key names as its last word (`min_volts`, `input_ratio`), if it is one of
/// [`VALUE_UNITS`].
fn unit_of(key: &str) -> Option<&'static str> {
    let (_, last) = key.rsplit_once('_')?;
    VALUE_UNITS
        .into_iter()
        .find(|unit| *unit == last)
        .filter(|_| snake(key))
}

/// An id or a claim's id: lowercase letters, digits and hyphens, and not empty.
fn plain(t: &str) -> bool {
    !t.is_empty()
        && t.chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
}

/// The record a device is, as a claim on it is cited before its `#`: a part (`DPS5005`), or a build
/// and its version (`supply/v1`). None unless it names exactly one of the two.
fn subject_of(part: Option<&str>, build: Option<&str>, version: Option<u32>) -> Option<String> {
    match (part, build, version) {
        (Some(p), None, None) => Some(p.into()),
        (None, Some(b), Some(v)) => Some(format!("{b}/v{v}")),
        _ => None,
    }
}

/// A sha256 as the records write it: 64 lowercase hex digits.
fn sha256_shaped(h: &str) -> bool {
    h.len() == 64
        && h.chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
}

/// Holds a record's claims to their shape: ids that are unique and plain, nothing empty, and
/// values that are finite and name their unit.
fn check_claims(at: &str, claims: &[Claim]) -> Result<(), String> {
    for (i, c) in claims.iter().enumerate() {
        if !plain(&c.id) {
            return Err(format!(
                "{at}: claim id {:?} must be lowercase letters, digits and hyphens",
                c.id
            ));
        }
        if claims[..i].iter().any(|d| d.id == c.id) {
            return Err(format!("{at}: claim id {} is used twice", c.id));
        }
        if c.says.is_empty() || c.referents().any(|(_, r)| r.is_empty()) {
            return Err(format!("{at}: claim {} has an empty string", c.id));
        }
        for (key, v) in &c.values {
            if unit_of(key).is_none() {
                return Err(format!(
                    "{at}: claim {} value {key} must end in its unit, one of {}",
                    c.id,
                    VALUE_UNITS.join(", ")
                ));
            }
            if !v.is_finite() {
                return Err(format!("{at}: claim {} value {key} is {v}", c.id));
            }
        }
    }
    Ok(())
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
    /// The trusted base's entries, in the file's order.
    pub trusted_base: Vec<Trusted>,
    /// Every file exactly as read, the trusted base then records then drawings then site documents
    /// then the lab's limits then the lab-target list then the lab log then the measurement
    /// records: the warehouse's bronze layer.
    pub sources: Vec<Source>,
    /// The lab's limits, checked. Set by [`Catalogue::with_limits`], which [`Catalogue::load`]
    /// calls; `from_sources` leaves it empty.
    pub limits: Option<tentzhen_lab::Limits>,
    /// The lab-target list, in the file's order, checked against the catalogue. Set by
    /// [`Catalogue::with_targets`], which [`Catalogue::load`] calls; `from_sources` leaves it
    /// empty, so nothing may be flashed.
    pub targets: Vec<target::Target>,
    /// The lab log's entries in the log's order, checked and chained. Set by [`Catalogue::with_log`],
    /// which [`Catalogue::load`] calls; `from_sources` leaves it empty.
    pub log: Vec<log::Entry>,
    /// The measurement records by id, checked against the catalogue and the log. Set by
    /// [`Catalogue::with_measurements`], which [`Catalogue::load`] calls, and which also holds
    /// every claim's `record` to them; `from_sources` leaves it empty and checks no `record`.
    pub measurements: BTreeMap<String, measurement::Measurement>,
}

impl Catalogue {
    pub fn load(root: &Path) -> Result<Self, String> {
        let base = read_source(root, &root.join(TRUSTED_BASE))?;
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
        let mut cat = Self::from_sources(&base, &parts, &builds)?;
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
        // The lab's limits, checked on the way in; bronze holds their sha256 for lineage.
        let (limits, text) = tentzhen_lab::Limits::load(root)?;
        cat.sources.push(Source {
            path: tentzhen_lab::LIMITS.into(),
            text,
        });
        cat.with_limits(limits)?
            .with_targets(target::read(root)?)?
            .with_log(log::read(root)?)?
            .with_measurements(measurement::read(root)?)
    }

    /// Adds the lab's limits, refusing what [`tentzhen_lab::Limits::check_with_claims`] refuses
    /// against this catalogue's claims.
    pub fn with_limits(mut self, limits: tentzhen_lab::Limits) -> Result<Self, String> {
        limits
            .check_with_claims(|c| self.claim(c).map(|c| &c.values))
            .map_err(|e| format!("{}: {e}", tentzhen_lab::LIMITS))?;
        self.limits = Some(limits);
        Ok(self)
    }

    /// Adds the lab-target list, before the log, whose flashes it holds. Refuses what
    /// [`target::parse`] refuses, and a target whose part or build version has no record.
    pub fn with_targets(mut self, list: Source) -> Result<Self, String> {
        let targets = target::parse(&list)?;
        for t in &targets {
            t.check_in(&self)?;
        }
        self.targets = targets;
        self.sources.push(list);
        Ok(self)
    }

    /// Adds the lab log's files, refusing what [`log::parse`] refuses and a flash the lab-target
    /// list does not allow ([`target`]), so it comes after [`Catalogue::with_targets`].
    pub fn with_log(mut self, files: Vec<Source>) -> Result<Self, String> {
        self.log = log::parse(&files)?;
        for e in self.log.iter().filter(|e| e.what == target::FLASH) {
            target::check_flash(&self, e)?;
        }
        self.sources.extend(files);
        Ok(self)
    }

    /// Adds the measurement records' files, after the log they cite. Refuses what
    /// [`measurement::parse`] refuses, a record that names what this catalogue or its log does not
    /// hold, and a claim whose `record` names no record that measured the claim's subject.
    pub fn with_measurements(mut self, files: Vec<Source>) -> Result<Self, String> {
        for s in &files {
            let m = measurement::parse(s)?;
            m.check_in(&self)?;
            if self.measurements.contains_key(&m.id) {
                return Err(format!("{}: record {} is given twice", s.path, m.id));
            }
            self.measurements.insert(m.id.clone(), m);
        }
        self.check_records()?;
        self.sources.extend(files);
        Ok(self)
    }

    pub fn from_sources(
        base: &Source,
        part_sources: &[Source],
        build_sources: &[Source],
    ) -> Result<Self, String> {
        let TrustedBase {
            entry: mut trusted_base,
        } = toml::from_str(&base.text).map_err(|e| format!("{}: {e}", base.path))?;
        for e in &mut trusted_base {
            e.record = base.path.clone();
        }
        for (i, e) in trusted_base.iter().enumerate() {
            if !plain(&e.id) {
                return Err(format!(
                    "{}: id {:?} must be lowercase letters, digits and hyphens",
                    base.path, e.id
                ));
            }
            if trusted_base[..i].iter().any(|d| d.id == e.id) {
                return Err(format!("{}: id {} is used twice", base.path, e.id));
            }
            if e.assumes.is_empty() {
                return Err(format!("{}: {} assumes nothing", base.path, e.id));
            }
        }

        let mut parts = BTreeMap::new();
        for s in part_sources {
            let mut p: Part = toml::from_str(&s.text).map_err(|e| format!("{}: {e}", s.path))?;
            p.record = s.path.clone();
            check_claims(&s.path, &p.claim)?;
            if parts.contains_key(&p.part) {
                return Err(format!("{}: part {} is defined twice", s.path, p.part));
            }
            parts.insert(p.part.clone(), p);
        }

        let mut builds: BTreeMap<String, Vec<Build>> = BTreeMap::new();
        for s in build_sources {
            let mut b: Build = toml::from_str(&s.text).map_err(|e| format!("{}: {e}", s.path))?;
            b.record = s.path.clone();
            check_claims(&s.path, &b.claim)?;
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
            trusted_base,
            sources: std::iter::once(base)
                .chain(part_sources)
                .chain(build_sources)
                .cloned()
                .collect(),
            limits: None,
            targets: Vec::new(),
            log: Vec::new(),
            measurements: BTreeMap::new(),
        };
        for versions in cat.builds.values() {
            for b in versions {
                cat.check_build(b)?;
            }
        }
        cat.check_no_cycles()?;
        cat.check_trusted()?;
        Ok(cat)
    }

    /// The claim a citation names: `<part>#<id>`, or `<build>/v<n>#<id>`.
    pub fn claim(&self, citation: &str) -> Option<&Claim> {
        let (subject, id) = citation.split_once('#')?;
        let claims = match self.parts.get(subject) {
            Some(p) => &p.claim,
            None => {
                let (build, v) = subject.rsplit_once("/v")?;
                &self.version(build, v.parse().ok()?)?.claim
            }
        };
        claims.iter().find(|c| c.id == id)
    }

    /// The trusted base's entry with this id.
    pub fn trusted(&self, id: &str) -> Option<&Trusted> {
        self.trusted_base.iter().find(|e| e.id == id)
    }

    /// Every claim's `trusted` referent names an entry in the trusted base.
    fn check_trusted<'a>(&'a self) -> Result<(), String> {
        let unknown = |t: Option<&'a str>| t.filter(|t| self.trusted(t).is_none());
        let claims = self.parts.values().map(|p| (&p.record, &p.claim)).chain(
            self.builds
                .values()
                .flatten()
                .map(|b| (&b.record, &b.claim)),
        );
        for (at, claims) in claims {
            for c in claims {
                if let Some(t) = unknown(c.trusted.as_deref()) {
                    return Err(format!(
                        "{at}: claim {} is trusted to {t:?}, which is no entry in {TRUSTED_BASE}",
                        c.id
                    ));
                }
            }
        }
        Ok(())
    }

    /// Every claim's `record` referent names a measurement record, which lists the claim's subject
    /// among the devices it measured.
    fn check_records(&self) -> Result<(), String> {
        let claims = self
            .parts
            .values()
            .map(|p| (&p.record, p.part.clone(), &p.claim))
            .chain(
                self.builds
                    .values()
                    .flatten()
                    .map(|b| (&b.record, format!("{}/v{}", b.build, b.version), &b.claim)),
            );
        for (at, subject, claims) in claims {
            for c in claims {
                let Some(r) = &c.record else { continue };
                let Some(m) = self.measurements.get(r) else {
                    return Err(format!(
                        "{at}: claim {} cites record {r:?}, which is no file in {}",
                        c.id,
                        measurement::RECORDS
                    ));
                };
                if !m
                    .device
                    .iter()
                    .any(|d| d.subject().as_deref() == Some(subject.as_str()))
                {
                    return Err(format!(
                        "{at}: claim {} cites record {r}, which does not list {subject} among the \
                         devices it measured",
                        c.id
                    ));
                }
            }
        }
        Ok(())
    }

    fn check_build(&self, b: &Build) -> Result<(), String> {
        let at = format!("{} v{}", b.build, b.version);
        if b.never_a_lab_target
            .as_ref()
            .is_some_and(|w| w.trim().is_empty())
        {
            return Err(format!("{at}: never_a_lab_target says why, and is empty"));
        }
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

    /// The build version that may never be a lab target, and why: `name` v`version` itself, or one
    /// it uses through any chain of `uses`.
    pub fn never_a_lab_target(&self, name: &str, version: u32) -> Option<(&Build, &str)> {
        let b = self.version(name, version)?;
        if let Some(why) = &b.never_a_lab_target {
            return Some((b, why));
        }
        b.uses
            .iter()
            .find_map(|u| self.never_a_lab_target(&u.build, u.version))
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

/// The files in a folder of the records under `root`, sorted, less its README: none if the folder
/// does not exist yet. The folder holds no folders.
fn read_folder(root: &Path, folder: &str) -> Result<Vec<Source>, String> {
    let dir = root.join(folder);
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    for path in read_dir_sorted(&dir)? {
        if path.ends_with("README.md") {
            continue;
        }
        if path.is_dir() {
            return Err(format!("{}: {folder} holds only files", path.display()));
        }
        files.push(read_source(root, &path)?);
    }
    Ok(files)
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

    /// A trusted base with one entry, `entry`.
    fn base() -> Source {
        src(
            "trusted-base.toml",
            "[[entry]]\nid = \"entry\"\nassumes = \"x\"\n",
        )
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

    /// The numbers builds/enforcer/v1.toml states are what its parts' claims and lab/limits.toml
    /// give. Each is worked out again here, so a changed part, resistor or limit fails until the
    /// claim follows it. A bound may be rounded, but only outward.
    #[test]
    fn the_enforcer_s_numbers_follow_from_its_parts() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let cat = Catalogue::load(&root).unwrap();
        let (limits, _) = tentzhen_lab::Limits::load(&root).unwrap();
        let v = |cite: &str, name: &str| -> f64 {
            let claim = cat.claim(cite).unwrap_or_else(|| panic!("no claim {cite}"));
            *claim
                .values
                .get(name)
                .unwrap_or_else(|| panic!("{cite} states no {name}"))
        };
        let e = |id: &str, name: &str| v(&format!("enforcer/v1#{id}"), name);
        // `stated` bounds `worked` from above (or below, with `sign` -1), and by less than `slack`.
        let bounds = |stated: f64, worked: f64, sign: f64, slack: f64, what: &str| {
            let over = sign * (stated - worked);
            assert!(
                (0.0..slack).contains(&over),
                "{what}: states {stated}, works out to {worked}"
            );
        };

        // The hardware cut: the divider's worst ratios, the reference's worst values over the
        // room, and the comparator's offset and hysteresis, counted in full against the cut, with
        // the offset's shift at the inputs' common mode, which sits at the reference.
        let (top, bottom) = (
            e("hardware-cut", "divider_top_ohms"),
            e("hardware-cut", "divider_bottom_ohms"),
        );
        let tol = e("hardware-cut", "resistor_tolerance_ratio");
        let drift = v("TLV3011B#reference", "reference_drift_per_kelvin_ratio")
            * e("hardware-cut", "room_half_span_kelvin");
        let slop = v("TLV3011B#offset", "max_offset_volts")
            + v("TLV3011B#hysteresis", "max_hysteresis_volts");
        let least = bottom * (1.0 - tol) / (top * (1.0 + tol) + bottom * (1.0 - tol));
        let most = bottom * (1.0 + tol) / (top * (1.0 - tol) + bottom * (1.0 + tol));
        let top_ref = v("TLV3011B#reference", "max_reference_volts") * (1.0 + drift);
        let slop = slop + top_ref * v("TLV3011B#common-mode", "common_mode_gain_ratio");
        let highest = (top_ref + slop) / least;
        let lowest = (v("TLV3011B#reference", "min_reference_volts") * (1.0 - drift) - slop) / most;
        let max_cut = e("hardware-cut", "max_cut_volts");
        bounds(max_cut, highest, 1.0, 0.001, "max_cut_volts");
        bounds(
            e("hardware-cut", "min_cut_volts"),
            lowest,
            -1.0,
            0.001,
            "min_cut_volts",
        );
        assert!(max_cut < v("RP2350#io-supply", "absolute_max_io_supply_volts"));

        // The measuring error, at the supply's ceilings, and a full scale past the fuse.
        let s = &limits.supply;
        assert_eq!(e("measuring-error", "at_volts"), s.max_volts.get());
        assert_eq!(e("measuring-error", "at_amps"), s.max_amps.get());
        let at = e("measuring-error", "at_volts");
        let volts = v("INA239#bus-voltage", "max_bus_offset_volts")
            + v("INA239#bus-voltage", "max_bus_gain_error_ratio") * at
            + v("INA239#bus-voltage", "bus_step_volts") / 2.0;
        bounds(
            e("measuring-error", "max_error_volts"),
            volts,
            1.0,
            1e-5,
            "max_error_volts",
        );
        let shunt = e("measuring-error", "shunt_ohms");
        let amps = e("measuring-error", "at_amps")
            * (v("INA239#shunt-voltage", "max_shunt_gain_error_ratio")
                + e("measuring-error", "shunt_tolerance_ratio"))
            + v("INA239#shunt-voltage", "max_shunt_offset_volts") / shunt
            + v("INA239#shunt-voltage", "shunt_step_volts") / (2.0 * shunt);
        bounds(
            e("measuring-error", "max_error_amps"),
            amps,
            1.0,
            1e-5,
            "max_error_amps",
        );
        let full = v("INA239#shunt-voltage", "shunt_full_scale_volts") / shunt;
        bounds(
            full,
            e("measuring-error", "full_scale_amps"),
            1.0,
            1e-9,
            "full_scale_amps",
        );
        assert!(
            full > s.fuse.rating_amps.get(),
            "reads any current the fuse passes"
        );

        // The switch blocks, and its gate takes, what feeds the DPS5005 if the DPS5005 fails.
        let block = e("switch-blocks-upstream", "max_block_volts");
        assert_eq!(block, v("AO3401A#ratings", "max_drain_source_volts"));
        assert!(block > s.upstream.volts.get());
        assert!(v("AO3401A#ratings", "max_gate_source_volts") > s.upstream.volts.get());

        // The gate drive, and its pull-down against erratum E9, which is at GP20's pad: the pad
        // sees ground through the series resistor and the pull-down together.
        let pull_down = e("open-when-undriven", "pull_down_ohms");
        let (drive, series) = (
            e("gate-drive", "drive_volts"),
            e("gate-drive", "series_ohms"),
        );
        assert!(series + pull_down <= v("RP2350#e9-pull-down", "max_pull_down_ohms"));
        let high = drive * pull_down / (series + pull_down);
        bounds(
            e("gate-drive", "en_high_volts"),
            high,
            -1.0,
            0.01,
            "en_high_volts",
        );
        assert!(e("gate-drive", "en_high_volts") >= v("AO3400A#on-resistance", "gate_drive_volts"));
        assert!(drive / series <= v("TLV3011B#output", "output_sink_amps"));
        assert_eq!(
            e("gate-drive", "en_low_volts"),
            v("TLV3011B#output", "max_output_low_volts")
        );
        assert!(e("gate-drive", "en_low_volts") < v("AO3400A#threshold", "min_threshold_volts"));
        for part in ["INA239#supply", "TLV3011B#supply"] {
            assert!((v(part, "min_supply_volts")..=v(part, "max_supply_volts")).contains(&drive));
        }
    }

    /// Copies a file, or a folder and everything in it.
    fn copy(from: &Path, to: &Path) {
        if from.is_dir() {
            for entry in fs::read_dir(from).unwrap() {
                let entry = entry.unwrap();
                copy(&entry.path(), &to.join(entry.file_name()));
            }
        } else {
            fs::create_dir_all(to.parent().unwrap()).unwrap();
            fs::copy(from, to).unwrap();
        }
    }

    /// `load` reads the lab log and the measurement records from disk, and holds a claim's `record`
    /// to them, and a flash in the log to the lab-target list: a copy of the repo's records, in a
    /// folder of its own, with a claim that cites one and a flash.
    #[test]
    fn load_holds_claims_to_the_records_on_disk() {
        let root =
            std::env::temp_dir().join(format!("tentzhen-records-{}-load", std::process::id()));
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        for path in [
            TRUSTED_BASE,
            "DISCLAIMER.md",
            "parts",
            "builds",
            tentzhen_lab::LIMITS,
            target::TARGETS,
        ] {
            copy(&repo.join(path), &root.join(path));
        }
        let rp2350 = root.join("parts/rp2350.toml");
        let text = fs::read_to_string(&rp2350).unwrap();
        let claim = "\n[[claim]]\nid = \"draw\"\nsays = \"x\"\nrecord = \"2026-09-21-draw\"\n";
        fs::write(&rp2350, format!("{text}{claim}")).unwrap();
        let err = Catalogue::load(&root).err().unwrap_or_default();
        assert!(
            err.contains(
                "claim draw cites record \"2026-09-21-draw\", which is no file in lab/records"
            ),
            "{err}"
        );
        let seen = log::append(
            &root,
            std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_790_000_000),
            "person:jon",
            "board.look",
            BTreeMap::new(),
            BTreeMap::from([("seen".into(), log::Datum::Flag(true))]),
        )
        .unwrap();
        let path = format!("{}/2026-09-21-draw.toml", measurement::RECORDS);
        fs::create_dir_all(root.join(measurement::RECORDS)).unwrap();
        fs::write(
            root.join(&path),
            format!(
                "measures = \"x\"\nsetup = \"x\"\nlog = {{ first = \"{0}\", last = \"{0}\" }}\n\n\
                 [[device]]\npart = \"RP2350\"\n\n[results]\nseen = true\n",
                seen.sha256
            ),
        )
        .unwrap();
        let cat = Catalogue::load(&root).unwrap();
        assert_eq!(cat.log.len(), 1);
        assert_eq!(cat.measurements["2026-09-21-draw"].log.first, seen.sha256);
        for file in [path.as_str(), "lab/log/2026-09-21.jsonl", target::TARGETS] {
            assert!(
                cat.sources.iter().any(|s| s.path == file),
                "{file} is in bronze"
            );
        }
        // The log takes a flash the list refuses, so it keeps what happened, and the records stop
        // loading until the list allows it.
        let flash = |key: &str, value: &str| (key.to_string(), log::Datum::Text(value.into()));
        log::append(
            &root,
            std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_790_000_060),
            "agent:claude",
            target::FLASH,
            BTreeMap::from([
                flash("target", "pico-1"),
                flash("serial", "E661"),
                flash("image_sha256", &"c".repeat(64)),
            ]),
            BTreeMap::from([("done".into(), log::Datum::Flag(true))]),
        )
        .unwrap();
        let err = Catalogue::load(&root).err().unwrap_or_default();
        assert!(
            err.contains("flashes pico-1, which is no target in lab/targets.toml"),
            "{err}"
        );
        let list = root.join(target::TARGETS);
        let text = fs::read_to_string(&list).unwrap();
        let pico = "[[target]]\nid = \"pico-1\"\npart = \"RP2350\"\nserial = \"E661\"\n\
                    listed = \"2026-09-21\"\napproved_by = \"person:jon\"\n";
        fs::write(&list, format!("{text}\n{pico}")).unwrap();
        let cat = Catalogue::load(&root).unwrap();
        assert_eq!((cat.log.len(), cat.targets.len()), (2, 1));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn no_record_can_type_a_grade() {
        let typed = "[[claim]]\nid = \"a\"\nsays = \"x\"\ngrade = \"proven\"\n";
        let err = Catalogue::from_sources(&base(), &[], &[build("probe", 1, typed)])
            .err()
            .unwrap_or_default();
        assert!(err.contains("unknown field `grade`"), "{err}");
    }

    #[test]
    fn a_part_can_carry_claims() {
        let part = format!("{PART}[[claim]]\nid = \"io\"\nsays = \"x\"\ntrusted = \"entry\"\n");
        let cat =
            Catalogue::from_sources(&base(), &[src("parts/rp2350.toml", &part)], &[]).unwrap();
        let c = &cat.parts["RP2350"].claim[0];
        assert_eq!(c.referents().collect::<Vec<_>>(), [("trusted", "entry")]);
    }

    #[test]
    fn a_trusted_referent_names_an_entry_in_the_trusted_base() {
        let claim = "[[claim]]\nid = \"a\"\nsays = \"x\"\ntrusted = \"nope\"\n";
        let err = Catalogue::from_sources(&base(), &[], &[build("probe", 1, claim)])
            .err()
            .unwrap_or_default();
        assert!(
            err.contains("builds/probe/v1.toml: claim a is trusted to \"nope\", which is no entry"),
            "{err}"
        );
        let part = format!("{PART}{claim}");
        let err = Catalogue::from_sources(&base(), &[src("parts/rp2350.toml", &part)], &[])
            .err()
            .unwrap_or_default();
        assert!(
            err.contains("parts/rp2350.toml: claim a is trusted to \"nope\""),
            "{err}"
        );
    }

    #[test]
    fn the_trusted_base_keeps_its_shape() {
        let refused = |text: &str, because: &str| {
            let err = Catalogue::from_sources(&src(TRUSTED_BASE, text), &[], &[])
                .err()
                .unwrap_or_default();
            assert!(err.contains(because), "{because}: {err}");
        };
        let entry = |id: &str, assumes: &str| {
            format!("[[entry]]\nid = \"{id}\"\nassumes = \"{assumes}\"\n")
        };
        refused(&entry("Silicon", "x"), "lowercase letters");
        refused(&entry("", "x"), "lowercase letters");
        refused(
            &format!("{}{}", entry("a", "x"), entry("a", "y")),
            "id a is used twice",
        );
        refused(&entry("a", ""), "a assumes nothing");
        refused("[[entry]]\nid = \"a\"\n", "missing field `assumes`");
        let two = format!("{}{}", entry("b", "y"), entry("a", "x"));
        let cat = Catalogue::from_sources(&src(TRUSTED_BASE, &two), &[], &[]).unwrap();
        let ids: Vec<&str> = cat.trusted_base.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, ["b", "a"], "entries keep the file's order");
    }

    fn repo() -> Catalogue {
        Catalogue::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap()
    }

    /// The trusted base states the DPS5005's numbers in words; its claim must carry the same ones.
    #[test]
    fn the_dps_input_numbers_are_the_trusted_base_s() {
        let cat = repo();
        let c = cat.claim("DPS5005#input-range").unwrap();
        let entry = &cat.trusted(c.trusted.as_deref().unwrap()).unwrap().assumes;
        let v = |name: &str| c.values[name];
        for said in [
            format!("{}–{} V", v("min_input_volts"), v("max_input_volts")),
            format!("{} ×", v("input_ratio")),
        ] {
            assert!(
                entry.contains(&said),
                "the trusted base does not say {said:?}"
            );
        }
    }

    #[test]
    fn a_citation_names_a_claim_on_a_part_or_a_build_version() {
        let cat = repo();
        assert_eq!(cat.claim("DPS5005#input-range").unwrap().id, "input-range");
        assert_eq!(
            cat.claim("debug-probe/v1#enumerates").unwrap().id,
            "enumerates"
        );
        for none in [
            "DPS5005#input-rnage",
            "DPS5005",
            "dps5005#input-range",
            "debug-probe/v2#enumerates",
            "debug-probe/vx#enumerates",
            "debug-probe#enumerates",
        ] {
            assert!(cat.claim(none).is_none(), "{none}");
        }
    }

    /// The limits are checked against the catalogue's own claims: change one and they are refused.
    #[test]
    fn the_limits_are_held_to_the_claims_they_cite() {
        let refused = |cat: Catalogue, because: &str| {
            let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
            let (limits, _) = tentzhen_lab::Limits::load(&root).unwrap();
            let err = cat.with_limits(limits).err().unwrap_or_default();
            assert!(err.contains(because), "{because}: {err}");
        };
        let mut raised = repo();
        let dps = raised.parts.get_mut("DPS5005").unwrap();
        dps.claim[0].values.insert("min_input_volts".into(), 7.0);
        refused(
            raised,
            "lab/limits.toml: [supply.upstream] volts is 6.5 V, below the 7.5 V",
        );
        let mut gone = repo();
        gone.parts.get_mut("DPS5005").unwrap().claim.remove(1);
        refused(gone, "rests on DPS5005#current-limit, which is no claim");
        let mut lower = repo();
        let rp2350 = lower.parts.get_mut("RP2350").unwrap();
        rp2350.claim[0]
            .values
            .insert("max_io_supply_volts".into(), 3.3);
        refused(
            lower,
            "pico_3v3.max_volts: is 3.4, above RP2350#io-supply's max_io_supply_volts of 3.3",
        );
    }

    // What lab/limits.toml's comments say about its values, with the claims they rest on.

    fn dps_floor(cat: &Catalogue) -> f64 {
        let v = &cat.claim("DPS5005#input-range").unwrap().values;
        let limits = cat.limits.as_ref().unwrap();
        limits
            .supply
            .dps_input_floor(v["min_input_volts"], v["input_ratio"])
    }

    #[test]
    fn the_dps_needs_6_5_volts_in_for_3v4_out() {
        let cat = repo();
        let err = cat.claim("bench-supply#setting-error").unwrap().values["setting_error_volts"];
        assert_eq!(dps_floor(&cat), 6.0);
        assert_eq!(dps_floor(&cat) + err, 6.5);
    }

    #[test]
    fn the_pico_ceiling_is_230_mv_under_the_rp2350_s_maximum() {
        let cat = repo();
        let v = &cat.claim("RP2350#io-supply").unwrap().values;
        let ceiling = cat.limits.as_ref().unwrap().pico_3v3.max_volts.get();
        assert_eq!(v["max_io_supply_volts"], 3.63);
        assert_eq!(v["absolute_max_io_supply_volts"], 3.63);
        assert!((v["max_io_supply_volts"] - ceiling - 0.230).abs() < 1e-9);
    }

    #[test]
    fn the_dps_input_is_above_the_ceiling_it_would_have_to_hold() {
        let cat = repo();
        assert!(dps_floor(&cat) > cat.limits.as_ref().unwrap().supply.max_volts.get());
    }

    #[test]
    fn a_claim_keeps_its_shape() {
        let refused = |claims: &str, because: &str| {
            let err = Catalogue::from_sources(&base(), &[], &[build("probe", 1, claims)])
                .err()
                .unwrap_or_default();
            assert!(err.contains(because), "{because}: {err}");
        };
        let claim = |body: &str| format!("[[claim]]\n{body}\n");
        refused(&claim("id = \"A b\"\nsays = \"x\""), "lowercase letters");
        refused(&claim("id = \"\"\nsays = \"x\""), "lowercase letters");
        refused(
            &format!(
                "{}{}",
                claim("id = \"a\"\nsays = \"x\""),
                claim("id = \"a\"\nsays = \"y\"")
            ),
            "claim id a is used twice",
        );
        refused(&claim("id = \"a\"\nsays = \"\""), "empty string");
        refused(
            &claim("id = \"a\"\nsays = \"x\"\ntest = \"\""),
            "empty string",
        );
        refused(
            &claim("id = \"a\"\nsays = \"x\"\nvalues = { min = 6.0 }"),
            "must end in its unit",
        );
        refused(
            &claim("id = \"a\"\nsays = \"x\"\nvalues = { min_furlongs = 6.0 }"),
            "must end in its unit",
        );
        refused(
            &claim("id = \"a\"\nsays = \"x\"\nvalues = { min_volts = inf }"),
            "is inf",
        );
        for key in ["_volts", "min__volts", "1_volts", "Min_volts"] {
            refused(
                &claim(&format!(
                    "id = \"a\"\nsays = \"x\"\nvalues = {{ {key} = 1 }}"
                )),
                "must end in its unit",
            );
        }
        let part = format!("{PART}{}", claim("id = \"A\"\nsays = \"x\""));
        let err = Catalogue::from_sources(&base(), &[src("parts/rp2350.toml", &part)], &[])
            .err()
            .unwrap_or_default();
        assert!(
            err.contains("lowercase letters"),
            "a part's claims are checked too: {err}"
        );
        let ok = claim(
            "id = \"input-range\"\nsays = \"x\"\nvalues = { min_volts = 6, input_ratio = 1.1 }",
        );
        assert!(Catalogue::from_sources(&base(), &[], &[build("probe", 1, &ok)]).is_ok());
    }

    #[test]
    fn every_table_refuses_a_key_it_does_not_know() {
        let refused = |parts: &[Source], builds: &[Source]| {
            let err = Catalogue::from_sources(&base(), parts, builds)
                .err()
                .unwrap_or_default();
            assert!(err.contains("unknown field `extra`"), "{err}");
        };
        refused(
            &[src("parts/rp2350.toml", &format!("extra = 1\n{PART}"))],
            &[],
        );
        for base in [
            "extra = 1\n",
            "[[entry]]\nextra = 1\nid = \"a\"\nassumes = \"x\"\n",
        ] {
            let err = Catalogue::from_sources(&src(TRUSTED_BASE, base), &[], &[])
                .err()
                .unwrap_or_default();
            assert!(err.contains("unknown field `extra`"), "{err}");
        }
        let part_claim = format!("{PART}[[claim]]\nextra = 1\nid = \"a\"\nsays = \"s\"\n");
        refused(&[src("parts/rp2350.toml", &part_claim)], &[]);
        let firmware = "name = \"f\"\nlicense = \"MIT\"\nsource = \"s\"\nrelease = \"r\"\nfile = \"f\"\nsha256 = \"0\"\n";
        for table in [
            "extra = 1\n".to_string(),
            "[[line]]\nextra = 1\ncommodity = \"wire\"\nqty = 1\n".into(),
            "[[uses]]\nextra = 1\nbuild = \"b\"\nversion = 1\n".into(),
            format!("[firmware]\nextra = 1\n{firmware}"),
            "[[pin]]\nextra = 1\nname = \"p\"\nboard_pin = 1\nnet = \"n\"\nrequired = true\n"
                .into(),
            "[[step]]\nextra = 1\ndo = \"d\"\n".into(),
            "[[claim]]\nextra = 1\nid = \"a\"\nsays = \"s\"\n".into(),
        ] {
            refused(&[], &[build("probe", 1, &table)]);
        }
    }

    #[test]
    fn an_unrecorded_part_is_refused() {
        let b = build("probe", 1, "[[line]]\npart = \"RP9999\"\nqty = 1\n");
        let err = Catalogue::from_sources(&base(), &[src("parts/rp2350.toml", PART)], &[b])
            .err()
            .unwrap();
        assert!(err.contains("RP9999"), "{err}");
    }

    #[test]
    fn a_file_must_sit_at_its_version() {
        let mut b = build("probe", 1, "");
        b.path = "builds/probe/v2.toml".into();
        assert!(Catalogue::from_sources(&base(), &[], &[b]).is_err());
    }

    #[test]
    fn versions_have_no_gaps() {
        let err = Catalogue::from_sources(
            &base(),
            &[],
            &[build("probe", 1, ""), build("probe", 3, "")],
        )
        .err()
        .unwrap();
        assert!(err.contains("v2 is missing"), "{err}");
    }

    #[test]
    fn a_uses_loop_is_refused() {
        let a = build("a", 1, "[[uses]]\nbuild = \"b\"\nversion = 1\n");
        let b = build("b", 1, "[[uses]]\nbuild = \"a\"\nversion = 1\n");
        let err = Catalogue::from_sources(&base(), &[], &[a, b])
            .err()
            .unwrap();
        assert!(err.contains("uses loop"), "{err}");
    }

    /// A build that may never be a lab target says why, and so does every build that uses it.
    #[test]
    fn a_build_that_is_never_a_lab_target_says_why() {
        let enforcer = build("e", 1, "never_a_lab_target = \"it holds the limits\"\n");
        let bench = build("b", 1, "[[uses]]\nbuild = \"e\"\nversion = 1\n");
        let other = build("o", 1, "");
        let cat = Catalogue::from_sources(&base(), &[], &[enforcer, bench, other]).unwrap();
        for name in ["e", "b"] {
            let (b, why) = cat.never_a_lab_target(name, 1).unwrap();
            assert_eq!((b.build.as_str(), why), ("e", "it holds the limits"));
        }
        assert!(cat.never_a_lab_target("o", 1).is_none());
        for empty in ["\"\"", "\"  \""] {
            let e = build("e", 1, &format!("never_a_lab_target = {empty}\n"));
            let err = Catalogue::from_sources(&base(), &[], &[e])
                .err()
                .unwrap_or_default();
            assert!(err.contains("never_a_lab_target says why"), "{err}");
        }
    }

    #[test]
    fn uses_counts_one_unless_told() {
        let bench = build("bench", 1, "[[uses]]\nbuild = \"probe\"\nversion = 1\n");
        let cat = Catalogue::from_sources(&base(), &[], &[build("probe", 1, ""), bench]).unwrap();
        assert_eq!(cat.version("bench", 1).unwrap().uses[0].qty, 1);
    }

    #[test]
    fn using_a_missing_version_is_refused() {
        let bench = build("bench", 1, "[[uses]]\nbuild = \"probe\"\nversion = 2\n");
        assert!(Catalogue::from_sources(&base(), &[], &[build("probe", 1, ""), bench]).is_err());
    }
}
