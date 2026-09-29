//! The lab's safety records. For now that is `lab/limits.toml`: what the hardware below any agent
//! will allow, each value's basis, and the facts the values rest on. [`Limits::parse`] refuses a
//! key it does not know, a value that is not a finite number above zero, a value without exactly
//! one basis, a copy that differs from its original, a DPS5005 setpoint or current limit above the
//! supply's ceiling, an upstream voltage outside the DPS5005's input rule, and a fuse that is not
//! the next rating above `max_amps`.

use serde::Deserialize;
use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::Path;

/// Where the limits live, relative to the repo root.
pub const LIMITS: &str = "lab/limits.toml";

/// The rated currents, in amps, of a quick-acting 5 x 20 mm fuse to IEC 60127-2, as one maker
/// offers them (Schurter FSF 5x20 datasheet, "Variants", read 2026-09-29).
const FUSE_5X20_AMPS: [f64; 26] = [
    0.032, 0.04, 0.05, 0.063, 0.08, 0.1, 0.125, 0.16, 0.2, 0.25, 0.315, 0.4, 0.5, 0.63, 0.8, 1.0,
    1.25, 1.6, 2.0, 2.5, 3.15, 4.0, 5.0, 6.3, 8.0, 10.0,
];

macro_rules! quantity {
    ($name:ident, $unit:literal, $symbol:literal) => {
        #[doc = concat!("A value in ", $unit, ": always finite and above zero.")]
        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Deserialize)]
        #[serde(try_from = "f64")]
        pub struct $name(f64);

        impl $name {
            pub fn get(self) -> f64 {
                self.0
            }
        }

        impl TryFrom<f64> for $name {
            type Error = String;

            fn try_from(v: f64) -> Result<Self, String> {
                if v.is_finite() && v > 0.0 {
                    Ok(Self(v))
                } else {
                    Err(format!(
                        "{} must be a finite number above 0, not {v}",
                        $unit
                    ))
                }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "{} {}", self.0, $symbol)
            }
        }
    };
}

quantity!(Volts, "volts", "V");
quantity!(Amps, "amps", "A");
quantity!(Ratio, "a ratio", "x");

/// Each value's basis, keyed by the value's name in its table.
pub type Bases = BTreeMap<String, Basis>;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    pub pico_3v3: Ceiling,
    pub supply: Supply,
    pub fact: Facts,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ceiling {
    pub max_volts: Volts,
    pub max_amps: Amps,
    pub basis: Bases,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Supply {
    pub max_volts: Volts,
    pub max_amps: Amps,
    pub reenable: Reenable,
    pub upstream: Upstream,
    pub dps: Dps,
    pub fuse: Fuse,
    pub basis: Bases,
}

/// After a trip, who may turn the output back on.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Reenable {
    Person,
    Agent,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Upstream {
    pub volts: Volts,
    pub basis: Bases,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dps {
    pub max_setpoint_volts: Volts,
    pub current_limit_amps: Amps,
    pub basis: Bases,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fuse {
    pub size: FuseSize,
    pub rating_amps: Amps,
    pub speed: FuseSpeed,
    pub basis: Bases,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum FuseSize {
    #[serde(rename = "5x20 mm")]
    Mm5x20,
}

/// Quick-acting only: the file asks for it, and the ratings above are that series'.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FuseSpeed {
    Fast,
}

/// Where a value comes from: exactly one of `policy`, `same_as` and `rule`, and the facts it
/// relies on to do its job.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Basis {
    /// A person's decision, and where it is recorded.
    pub policy: Option<String>,
    /// The path of another value that this one copies.
    pub same_as: Option<String>,
    /// The rule in this crate that this value is held to.
    pub rule: Option<Rule>,
    #[serde(default)]
    pub rests_on: Vec<FactId>,
}

/// A rule a value is held to. Its check is in `check_values`, marked with its name;
/// [`Rule::inputs`] and [`Rule::facts`] name what that check reads, kept in step by hand, and give
/// the warehouse its lineage.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Rule {
    /// The upstream voltage: at least the DPS5005's input floor for its setpoint plus the bench
    /// supply's setting error, and at most its maximum input less that error.
    DpsInput,
    /// The fuse: the next 5 x 20 mm rating above the supply's `max_amps`.
    NextFuseRating,
}

/// The facts at the end of `lab/limits.toml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Facts {
    pub dps_input: DpsInput,
    pub opendps_current_limit: Behaviour,
    pub bench_setting_error: SettingError,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FactId {
    DpsInput,
    OpendpsCurrentLimit,
    BenchSettingError,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DpsInput {
    pub says: String,
    pub min_volts: Volts,
    pub max_volts: Volts,
    pub ratio: Ratio,
    pub trusted: Option<String>,
    pub record: Option<String>,
}

/// A fact about how something behaves, with no number of its own.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Behaviour {
    pub says: String,
    pub trusted: Option<String>,
    pub record: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettingError {
    pub says: String,
    pub volts: Volts,
    pub trusted: Option<String>,
    pub record: Option<String>,
}

/// A value as the warehouse stores it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Value {
    Volts(f64),
    Amps(f64),
    Choice(&'static str),
}

/// A table of values in the file: its path, its values by name, and their bases.
struct Table<'a> {
    path: &'static str,
    bases: &'a Bases,
    values: Vec<(&'static str, Value)>,
}

/// One value, where it sits, and its basis.
pub struct Entry<'a> {
    pub path: String,
    pub value: Value,
    pub basis: &'a Basis,
}

/// One fact and its referents: `record` (a bench measurement) or `trusted` (an entry in
/// docs/verification.md's trusted base). The warehouse grades it from them.
pub struct Fact<'a> {
    pub id: FactId,
    pub says: &'a str,
    pub trusted: Option<&'a str>,
    pub record: Option<&'a str>,
}

impl Reenable {
    pub fn as_str(self) -> &'static str {
        match self {
            Reenable::Person => "person",
            Reenable::Agent => "agent",
        }
    }
}

impl FuseSize {
    pub fn as_str(self) -> &'static str {
        match self {
            FuseSize::Mm5x20 => "5x20 mm",
        }
    }
}

impl FuseSpeed {
    pub fn as_str(self) -> &'static str {
        match self {
            FuseSpeed::Fast => "fast",
        }
    }
}

impl Rule {
    pub const ALL: [Rule; 2] = [Rule::DpsInput, Rule::NextFuseRating];

    pub fn as_str(self) -> &'static str {
        match self {
            Rule::DpsInput => "dps-input",
            Rule::NextFuseRating => "next-fuse-rating",
        }
    }

    /// The one value this rule is for.
    pub fn target(self) -> &'static str {
        match self {
            Rule::DpsInput => "supply.upstream.volts",
            Rule::NextFuseRating => "supply.fuse.rating_amps",
        }
    }

    pub fn inputs(self) -> &'static [&'static str] {
        match self {
            Rule::DpsInput => &["supply.dps.max_setpoint_volts"],
            Rule::NextFuseRating => &["supply.max_amps"],
        }
    }

    pub fn facts(self) -> &'static [FactId] {
        match self {
            Rule::DpsInput => &[FactId::DpsInput, FactId::BenchSettingError],
            Rule::NextFuseRating => &[],
        }
    }
}

impl FactId {
    pub fn as_str(self) -> &'static str {
        match self {
            FactId::DpsInput => "dps_input",
            FactId::OpendpsCurrentLimit => "opendps_current_limit",
            FactId::BenchSettingError => "bench_setting_error",
        }
    }
}

impl Basis {
    /// Which of `policy`, `same_as` and `rule` this is. After [`Limits::parse`], exactly one.
    pub fn kind(&self) -> &'static str {
        match (&self.policy, &self.same_as) {
            (Some(_), _) => "policy",
            (None, Some(_)) => "same_as",
            (None, None) => "rule",
        }
    }

    /// The values this one comes from.
    pub fn inputs(&self) -> Vec<&str> {
        match (&self.same_as, self.rule) {
            (Some(p), _) => vec![p.as_str()],
            (None, Some(r)) => r.inputs().to_vec(),
            (None, None) => Vec::new(),
        }
    }
}

impl Supply {
    /// The least the DPS5005's input may be for its highest setpoint, before any setting error.
    pub fn dps_input_floor(&self, f: &DpsInput) -> f64 {
        f.min_volts
            .get()
            .max(f.ratio.get() * self.dps.max_setpoint_volts.get())
    }

    /// The fuse rating the file asks for: the next one above `max_amps`.
    pub fn fuse_wanted(&self) -> Option<f64> {
        FUSE_5X20_AMPS
            .into_iter()
            .find(|&r| r > self.max_amps.get())
    }
}

impl Limits {
    /// Parses the limits and checks them. The error names the rule a value breaks.
    pub fn parse(text: &str) -> Result<Self, String> {
        let limits: Limits = toml::from_str(text).map_err(|e| e.to_string())?;
        limits.check()?;
        Ok(limits)
    }

    /// Reads and checks [`LIMITS`] under `root`, returning the limits and the text they came from.
    pub fn load(root: &Path) -> Result<(Self, String), String> {
        let text = fs::read_to_string(root.join(LIMITS)).map_err(|e| format!("{LIMITS}: {e}"))?;
        let limits = Self::parse(&text).map_err(|e| format!("{LIMITS}: {e}"))?;
        Ok((limits, text))
    }

    /// Every table of values in the file.
    fn tables(&self) -> [Table<'_>; 5] {
        let (p, s) = (&self.pico_3v3, &self.supply);
        [
            Table {
                path: "pico_3v3",
                bases: &p.basis,
                values: vec![
                    ("max_volts", Value::Volts(p.max_volts.get())),
                    ("max_amps", Value::Amps(p.max_amps.get())),
                ],
            },
            Table {
                path: "supply",
                bases: &s.basis,
                values: vec![
                    ("max_volts", Value::Volts(s.max_volts.get())),
                    ("max_amps", Value::Amps(s.max_amps.get())),
                    ("reenable", Value::Choice(s.reenable.as_str())),
                ],
            },
            Table {
                path: "supply.upstream",
                bases: &s.upstream.basis,
                values: vec![("volts", Value::Volts(s.upstream.volts.get()))],
            },
            Table {
                path: "supply.dps",
                bases: &s.dps.basis,
                values: vec![
                    (
                        "max_setpoint_volts",
                        Value::Volts(s.dps.max_setpoint_volts.get()),
                    ),
                    (
                        "current_limit_amps",
                        Value::Amps(s.dps.current_limit_amps.get()),
                    ),
                ],
            },
            Table {
                path: "supply.fuse",
                bases: &s.fuse.basis,
                values: vec![
                    ("size", Value::Choice(s.fuse.size.as_str())),
                    ("rating_amps", Value::Amps(s.fuse.rating_amps.get())),
                    ("speed", Value::Choice(s.fuse.speed.as_str())),
                ],
            },
        ]
    }

    /// Every value with its basis. After [`Limits::parse`], every value has one.
    pub fn entries(&self) -> Vec<Entry<'_>> {
        let mut out = Vec::new();
        for t in self.tables() {
            for (name, value) in t.values {
                if let Some(basis) = t.bases.get(name) {
                    out.push(Entry {
                        path: format!("{}.{name}", t.path),
                        value,
                        basis,
                    });
                }
            }
        }
        out
    }

    pub fn facts(&self) -> [Fact<'_>; 3] {
        let f = &self.fact;
        [
            Fact {
                id: FactId::DpsInput,
                says: &f.dps_input.says,
                trusted: f.dps_input.trusted.as_deref(),
                record: f.dps_input.record.as_deref(),
            },
            Fact {
                id: FactId::OpendpsCurrentLimit,
                says: &f.opendps_current_limit.says,
                trusted: f.opendps_current_limit.trusted.as_deref(),
                record: f.opendps_current_limit.record.as_deref(),
            },
            Fact {
                id: FactId::BenchSettingError,
                says: &f.bench_setting_error.says,
                trusted: f.bench_setting_error.trusted.as_deref(),
                record: f.bench_setting_error.record.as_deref(),
            },
        ]
    }

    fn check(&self) -> Result<(), String> {
        self.check_values()?;
        self.check_bases()?;
        for f in self.facts() {
            if [Some(f.says), f.trusted, f.record].contains(&Some("")) {
                return Err(format!("[fact.{}] has an empty string", f.id.as_str()));
            }
        }
        Ok(())
    }

    fn check_values(&self) -> Result<(), String> {
        let s = &self.supply;
        if s.dps.max_setpoint_volts > s.max_volts {
            return Err(format!(
                "[supply.dps] max_setpoint_volts is {}, above [supply] max_volts {}",
                s.dps.max_setpoint_volts, s.max_volts
            ));
        }
        if s.dps.current_limit_amps > s.max_amps {
            return Err(format!(
                "[supply.dps] current_limit_amps is {}, above [supply] max_amps {}",
                s.dps.current_limit_amps, s.max_amps
            ));
        }
        let f = &self.fact.dps_input;
        if f.ratio.get() <= 1.0 {
            return Err(format!(
                "[fact.dps_input] ratio is {}, not above 1",
                f.ratio
            ));
        }
        if f.min_volts >= f.max_volts {
            return Err(format!(
                "[fact.dps_input] min_volts {} is not below max_volts {}",
                f.min_volts, f.max_volts
            ));
        }
        // Rule::DpsInput
        let (f, err) = (
            &self.fact.dps_input,
            self.fact.bench_setting_error.volts.get(),
        );
        let (volts, least) = (s.upstream.volts.get(), s.dps_input_floor(f) + err);
        if volts < least {
            return Err(format!(
                "[supply.upstream] volts is {volts} V, below the {least} V the DPS5005 needs to \
                 put out {}: at least {}, and {} its output, plus the bench supply's setting \
                 error of {err} V",
                s.dps.max_setpoint_volts, f.min_volts, f.ratio
            ));
        }
        if volts + err > f.max_volts.get() {
            return Err(format!(
                "[supply.upstream] volts is {volts} V, which with the bench supply's setting \
                 error of {err} V can pass the DPS5005's maximum input of {}",
                f.max_volts
            ));
        }
        // Rule::NextFuseRating
        match s.fuse_wanted() {
            Some(want) if s.fuse.rating_amps.get() == want => Ok(()),
            Some(want) => Err(format!(
                "[supply.fuse] rating_amps is {}; the next 5 x 20 mm rating above max_amps {} is \
                 {want} A",
                s.fuse.rating_amps, s.max_amps
            )),
            None => Err(format!(
                "[supply] max_amps {} is above every 5 x 20 mm fuse rating listed in crates/lab",
                s.max_amps
            )),
        }
    }

    fn check_bases(&self) -> Result<(), String> {
        let mut values = BTreeMap::new();
        for t in self.tables() {
            for (name, value) in &t.values {
                if !t.bases.contains_key(*name) {
                    return Err(format!("[{}] {name} has no basis", t.path));
                }
                values.insert(format!("{}.{name}", t.path), *value);
            }
            for name in t.bases.keys() {
                if !t.values.iter().any(|(n, _)| n == name) {
                    return Err(format!(
                        "[{}] basis.{name} names no value in this table",
                        t.path
                    ));
                }
            }
        }
        let entries = self.entries();
        for e in &entries {
            let (b, at) = (e.basis, &e.path);
            let kinds = [b.policy.is_some(), b.same_as.is_some(), b.rule.is_some()];
            if kinds.iter().filter(|&&k| k).count() != 1 {
                return Err(format!(
                    "{at}: its basis must name exactly one of policy, same_as and rule"
                ));
            }
            if b.policy.as_deref() == Some("") {
                return Err(format!("{at}: its policy is empty"));
            }
            for (i, f) in b.rests_on.iter().enumerate() {
                if b.rests_on[..i].contains(f) {
                    return Err(format!("{at}: rests on {} twice", f.as_str()));
                }
            }
            if let Some(p) = &b.same_as {
                let Some(v) = values.get(p) else {
                    return Err(format!("{at}: same_as {p}, which is not a value"));
                };
                if std::mem::discriminant(v) != std::mem::discriminant(&e.value) {
                    return Err(format!("{at}: same_as {p}, a different kind of value"));
                }
                if *v != e.value {
                    return Err(format!("{at}: same_as {p}, but {:?} is not {v:?}", e.value));
                }
            }
            if let Some(r) = b.rule {
                if r.target() != at {
                    return Err(format!(
                        "{at}: rule {} is for {}, not this",
                        r.as_str(),
                        r.target()
                    ));
                }
                if let Some(f) = r.facts().iter().find(|f| !b.rests_on.contains(f)) {
                    return Err(format!(
                        "{at}: rule {} uses fact {}, so it must rest on it",
                        r.as_str(),
                        f.as_str()
                    ));
                }
            }
        }
        // check_values holds every rule's value to it, so its basis must say so, or its grade
        // would leave out the facts the rule reads.
        for r in Rule::ALL {
            if !entries
                .iter()
                .any(|e| e.path == r.target() && e.basis.rule == Some(r))
            {
                return Err(format!(
                    "{}: is held to rule {}, so its basis must name it",
                    r.target(),
                    r.as_str()
                ));
            }
        }
        // No value may come from itself through any chain of same_as and rule inputs.
        let inputs: BTreeMap<&str, Vec<&str>> = entries
            .iter()
            .map(|e| (e.path.as_str(), e.basis.inputs()))
            .collect();
        for (&start, first) in &inputs {
            let (mut stack, mut seen) = (first.clone(), Vec::new());
            while let Some(p) = stack.pop() {
                if p == start {
                    return Err(format!("{start} comes from itself"));
                }
                if !seen.contains(&p) {
                    seen.push(p);
                    stack.extend(inputs.get(p).into_iter().flatten().copied());
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn repo() -> String {
        fs::read_to_string(root().join(LIMITS)).unwrap()
    }

    /// The repo's file with `from` changed to `to`, where `from` occurs exactly once.
    fn with(from: &str, to: &str) -> String {
        let text = repo();
        assert_eq!(text.matches(from).count(), 1, "{from:?} is not unique");
        text.replace(from, to)
    }

    fn refused(text: &str, because: &str) {
        let err = Limits::parse(text)
            .err()
            .unwrap_or_else(|| panic!("accepted, but {because}"));
        assert!(err.contains(because), "refused for another reason: {err}");
    }

    #[test]
    fn the_repo_limits_load() {
        Limits::load(&root()).expect("lab/limits.toml loads");
    }

    /// `tables` lists values by hand; this holds it to the file, so no value goes without a basis
    /// check or a row in the warehouse.
    #[test]
    fn every_value_in_the_file_is_an_entry() {
        fn leaves(t: &toml::Table, at: &str, out: &mut Vec<String>) {
            for (k, v) in t {
                let path = if at.is_empty() {
                    k.clone()
                } else {
                    format!("{at}.{k}")
                };
                match v {
                    toml::Value::Table(_) if k == "basis" || path == "fact" => {}
                    toml::Value::Table(sub) => leaves(sub, &path, out),
                    _ => out.push(path),
                }
            }
        }
        let mut in_file = Vec::new();
        leaves(&repo().parse().unwrap(), "", &mut in_file);
        let l = Limits::parse(&repo()).unwrap();
        let mut entries: Vec<String> = l.entries().into_iter().map(|e| e.path).collect();
        in_file.sort();
        entries.sort();
        assert_eq!(in_file, entries);
    }

    /// The entries of docs/verification.md's trusted base, each bullet joined onto one line.
    fn trusted_base() -> Vec<String> {
        let doc = fs::read_to_string(root().join("docs/verification.md")).unwrap();
        let base = doc
            .split("\n## Trusted base\n")
            .nth(1)
            .expect("docs/verification.md has a Trusted base section");
        let base = base.split("\n## ").next().unwrap_or(base);
        let mut entries: Vec<String> = Vec::new();
        for line in base.lines() {
            if let Some(entry) = line.strip_prefix("- ") {
                entries.push(entry.into());
            } else if let (Some(last), Some(more)) = (entries.last_mut(), line.strip_prefix("  ")) {
                last.push(' ');
                last.push_str(more);
            }
        }
        entries
    }

    fn trusted_entry(t: &str) -> Option<String> {
        trusted_base().into_iter().find(|e| e.starts_with(t))
    }

    #[test]
    fn every_trusted_fact_names_an_entry_in_the_trusted_base() {
        let l = Limits::parse(&repo()).unwrap();
        for f in l.facts() {
            if let Some(t) = f.trusted {
                assert!(
                    trusted_entry(t).is_some(),
                    "fact {} is trusted as {t:?}, which begins no entry in the trusted base",
                    f.id.as_str()
                );
            }
        }
    }

    /// The trusted base states the DPS5005's numbers in words; the file must carry the same ones.
    #[test]
    fn the_dps_input_numbers_are_the_trusted_base_s() {
        let f = Limits::parse(&repo()).unwrap().fact.dps_input;
        let entry = trusted_entry(f.trusted.as_deref().unwrap()).unwrap();
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

    /// `facts` lists the facts by hand; this holds it to the file's [fact.*] tables.
    #[test]
    fn every_fact_in_the_file_is_listed() {
        let table: toml::Table = repo().parse().unwrap();
        let mut in_file: Vec<&str> = table["fact"]
            .as_table()
            .unwrap()
            .keys()
            .map(|k| k.as_str())
            .collect();
        let l = Limits::parse(&repo()).unwrap();
        let mut listed: Vec<&str> = l.facts().iter().map(|f| f.id.as_str()).collect();
        in_file.sort();
        listed.sort();
        assert_eq!(in_file, listed);
    }

    // What the file's comments say about its own values.

    #[test]
    fn the_dps_needs_6_5_volts_in_for_3v6_out() {
        let l = Limits::parse(&repo()).unwrap();
        let floor = l.supply.dps_input_floor(&l.fact.dps_input);
        assert_eq!(floor, 6.0);
        assert_eq!(floor + l.fact.bench_setting_error.volts.get(), 6.5);
    }

    #[test]
    fn the_dps_input_is_above_the_ceiling_it_would_have_to_hold() {
        let l = Limits::parse(&repo()).unwrap();
        assert!(l.supply.dps_input_floor(&l.fact.dps_input) > l.supply.max_volts.get());
    }

    // The rules: each one refuses a file that breaks it.

    #[test]
    fn a_misspelt_key_is_refused() {
        refused(
            &with("current_limit_amps = 0.200", "current_limit_amp = 0.200"),
            "unknown field `current_limit_amp`",
        );
    }

    #[test]
    fn every_table_refuses_a_key_it_does_not_know() {
        refused(&format!("extra = 1\n{}", repo()), "unknown field `extra`");
        for table in [
            "[pico_3v3]\n",
            "[supply]\n",
            "[supply.upstream]\n",
            "[supply.dps]\n",
            "[supply.fuse]\n",
            "[fact.dps_input]\n",
            "[fact.opendps_current_limit]\n",
            "[fact.bench_setting_error]\n",
        ] {
            refused(
                &with(table, &format!("{table}extra = 1\n")),
                "unknown field `extra`",
            );
        }
        refused(
            &with(
                "{ policy = \"Jon, 2026-09-28\" }",
                "{ policy = \"Jon, 2026-09-28\", extra = 1 }",
            ),
            "unknown field `extra`",
        );
        refused(
            &format!("{}[fact.extra]\n", repo()),
            "unknown field `extra`",
        );
    }

    #[test]
    fn a_misspelt_choice_is_refused() {
        refused(
            &with("reenable = \"agent\"", "reenable = \"agnet\""),
            "agnet",
        );
        refused(&with("speed = \"fast\"", "speed = \"slow\""), "slow");
        refused(
            &with("rule = \"next-fuse-rating\"", "rule = \"next-fuse\""),
            "next-fuse",
        );
        refused(
            &with(
                "rests_on = [\"opendps_current_limit\"]",
                "rests_on = [\"opendps_limit\"]",
            ),
            "opendps_limit",
        );
    }

    #[test]
    fn a_value_must_be_finite_and_above_zero() {
        let key = "max_setpoint_volts = 3.6";
        for bad in ["inf", "nan", "0.0", "-3.6"] {
            refused(
                &with(key, &format!("max_setpoint_volts = {bad}")),
                "must be a finite number above 0",
            );
        }
    }

    #[test]
    fn the_dps_stays_within_the_ceiling() {
        refused(
            &with("max_setpoint_volts = 3.6", "max_setpoint_volts = 3.7"),
            "above [supply] max_volts",
        );
        refused(
            &with("current_limit_amps = 0.200", "current_limit_amps = 0.210"),
            "above [supply] max_amps",
        );
    }

    #[test]
    fn the_dps_gets_the_input_it_needs() {
        for low in ["5.9", "6.4"] {
            refused(
                &with("volts = 6.5", &format!("volts = {low}")),
                "the DPS5005 needs",
            );
        }
        for high in ["54.6", "56"] {
            refused(
                &with("volts = 6.5", &format!("volts = {high}")),
                "maximum input",
            );
        }
    }

    #[test]
    fn the_fuse_is_the_next_rating_up() {
        let key = "rating_amps = 0.250";
        for bad in ["0.200", "0.315", "0.24"] {
            refused(
                &with(key, &format!("rating_amps = {bad}")),
                "the next 5 x 20 mm rating",
            );
        }
        refused(
            &with("max_amps = 0.200\n# After", "max_amps = 20\n# After"),
            "above every 5 x 20 mm fuse rating",
        );
    }

    #[test]
    fn every_value_has_exactly_one_basis() {
        refused(
            &with("basis.speed = { policy = \"PR #11\" }\n", ""),
            "[supply.fuse] speed has no basis",
        );
        refused(
            &with(
                "basis.speed = { policy = \"PR #11\" }\n",
                "basis.speed = { policy = \"PR #11\" }\nbasis.sped = { policy = \"PR #11\" }\n",
            ),
            "basis.sped names no value",
        );
        refused(
            &with(
                "basis.speed = { policy = \"PR #11\" }",
                "basis.speed = { policy = \"PR #11\", rule = \"next-fuse-rating\" }",
            ),
            "exactly one of policy, same_as and rule",
        );
        refused(
            &with(
                "basis.speed = { policy = \"PR #11\" }",
                "basis.speed = { rests_on = [\"dps_input\"] }",
            ),
            "exactly one of policy, same_as and rule",
        );
        refused(
            &with(
                "basis.speed = { policy = \"PR #11\" }",
                "basis.speed = { policy = \"\" }",
            ),
            "policy is empty",
        );
    }

    #[test]
    fn a_copy_equals_its_original() {
        refused(
            &with(
                "max_amps = 0.200\nbasis.max_volts",
                "max_amps = 0.300\nbasis.max_volts",
            ),
            "supply.max_amps: same_as pico_3v3.max_amps, but",
        );
        refused(
            &with(
                "same_as = \"pico_3v3.max_volts\"",
                "same_as = \"pico_3v3.max_amps\"",
            ),
            "a different kind of value",
        );
        refused(
            &with(
                "same_as = \"pico_3v3.max_volts\"",
                "same_as = \"pico.max_volts\"",
            ),
            "which is not a value",
        );
    }

    #[test]
    fn a_rule_is_for_one_value() {
        refused(
            &with(
                "basis.volts = { rule = \"dps-input\"",
                "basis.volts = { rule = \"next-fuse-rating\"",
            ),
            "rule next-fuse-rating is for supply.fuse.rating_amps",
        );
    }

    #[test]
    fn a_value_rests_on_every_fact_its_rule_uses() {
        refused(
            &with(
                "rests_on = [\"dps_input\", \"bench_setting_error\"]",
                "rests_on = [\"dps_input\"]",
            ),
            "uses fact bench_setting_error",
        );
        refused(
            &with(
                "rests_on = [\"dps_input\", \"bench_setting_error\"]",
                "rests_on = [\"dps_input\", \"bench_setting_error\", \"dps_input\"]",
            ),
            "rests on dps_input twice",
        );
    }

    #[test]
    fn a_rule_s_value_names_it() {
        refused(
            &with(
                "basis.volts = { rule = \"dps-input\", rests_on = [\"dps_input\", \"bench_setting_error\"] }",
                "basis.volts = { policy = \"someone\" }",
            ),
            "is held to rule dps-input, so its basis must name it",
        );
        refused(
            &with(
                "basis.rating_amps = { rule = \"next-fuse-rating\" }",
                "basis.rating_amps = { policy = \"someone\" }",
            ),
            "is held to rule next-fuse-rating",
        );
    }

    #[test]
    fn the_dps_input_fact_is_self_consistent() {
        refused(&with("ratio = 1.1", "ratio = 0.11"), "not above 1");
        refused(
            &with("max_volts = 55.0", "max_volts = 5.0"),
            "is not below max_volts",
        );
    }

    #[test]
    fn no_value_comes_from_itself() {
        let text = with(
            "basis.max_volts = { policy = \"CLAUDE.md, \\\"Default ceilings: 3.6 V and 200 mA\\\"\" }",
            "basis.max_volts = { same_as = \"supply.max_volts\" }",
        );
        refused(&text, "comes from itself");
    }

    #[test]
    fn a_fact_has_no_empty_string() {
        refused(
            &with(
                "says = \"OpenDPS regulates its output current to the limit set\"",
                "says = \"\"",
            ),
            "[fact.opendps_current_limit] has an empty string",
        );
    }
}
