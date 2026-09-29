//! The catalogue's records: parts and versioned builds, parsed from `parts/` and `builds/` and
//! checked against the contract every consumer relies on (the site, the warehouse). Both can carry
//! claims, in one shape: [`Claim`]. A claim assumed rather than shown names an entry in the trusted
//! base, `trusted-base.toml`: [`Trusted`].

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
    /// A bench measurement record that shows it.
    pub record: Option<String>,
    /// The id of the entry in the trusted base that assumes it: [`Trusted`].
    pub trusted: Option<String>,
    /// Numbers the claim states. Each key ends in its unit: [`VALUE_UNITS`].
    #[serde(default)]
    pub values: BTreeMap<String, f64>,
}

/// The unit a claim's value may name, as the last part of its key (`min_volts`, `input_ratio`).
pub const VALUE_UNITS: [&str; 7] = [
    "volts", "amps", "ohms", "watts", "hertz", "seconds", "ratio",
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

/// An entry in the trusted base: something assumed, not shown. A claim or a lab fact that rests on
/// it names its `id` as its `trusted` referent, and a `trusted` that names no entry is refused.
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

/// An id or a claim's id: lowercase letters, digits and hyphens, and not empty.
fn plain(t: &str) -> bool {
    !t.is_empty()
        && t.chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
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
            let unit = key.rsplit('_').next().unwrap_or_default();
            let named = key.contains('_')
                && key.starts_with(|ch: char| ch.is_ascii_lowercase())
                && key.split('_').all(|word| !word.is_empty())
                && key
                    .chars()
                    .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_');
            if !named || !VALUE_UNITS.contains(&unit) {
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
    /// then the lab's limits: the warehouse's bronze layer.
    pub sources: Vec<Source>,
    /// The lab's limits, checked. Set by [`Catalogue::with_limits`], which [`Catalogue::load`]
    /// calls; `from_sources` leaves it empty.
    pub limits: Option<tentzhen_lab::Limits>,
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
        cat.with_limits(limits)
    }

    /// Adds the lab's limits, refusing a fact trusted to no entry in the trusted base.
    pub fn with_limits(mut self, limits: tentzhen_lab::Limits) -> Result<Self, String> {
        self.limits = Some(limits);
        self.check_trusted()?;
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

    /// The trusted base's entry with this id.
    pub fn trusted(&self, id: &str) -> Option<&Trusted> {
        self.trusted_base.iter().find(|e| e.id == id)
    }

    /// Every `trusted` referent, on a claim or a lab fact, names an entry in the trusted base.
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
        for f in self.limits.iter().flat_map(|l| l.facts()) {
            if let Some(t) = unknown(f.trusted) {
                return Err(format!(
                    "{}: [fact.{}] is trusted to {t:?}, which is no entry in {TRUSTED_BASE}",
                    tentzhen_lab::LIMITS,
                    f.id.as_str()
                ));
            }
        }
        Ok(())
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
        // A lab fact: the repo's limits, with the first fact's entry renamed.
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let text = fs::read_to_string(root.join(tentzhen_lab::LIMITS)).unwrap();
        let from = "trusted = \"dps5005\"";
        assert!(text.contains(from), "{from:?} is not in the limits");
        let limits = tentzhen_lab::Limits::parse(&text.replacen(from, "trusted = \"nope\"", 1));
        let err = Catalogue::load(&root)
            .unwrap()
            .with_limits(limits.unwrap())
            .err()
            .unwrap_or_default();
        assert!(
            err.contains("lab/limits.toml: [fact.dps_input] is trusted to \"nope\""),
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

    /// The trusted base states the DPS5005's numbers in words; lab/limits.toml must carry the same
    /// ones.
    #[test]
    fn the_dps_input_numbers_are_the_trusted_base_s() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let cat = Catalogue::load(&root).unwrap();
        let f = &cat.limits.as_ref().unwrap().fact.dps_input;
        let entry = &cat.trusted(f.trusted.as_deref().unwrap()).unwrap().assumes;
        for said in [
            format!("{}–{} V", f.min_volts.get(), f.max_volts.get()),
            format!("{} ×", f.ratio.get()),
        ] {
            assert!(
                entry.contains(&said),
                "the trusted base does not say {said:?}"
            );
        }
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
