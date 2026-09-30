//! The lab's safety records. For now that is `lab/limits.toml`: what the hardware below any agent
//! will allow, each value's basis, and the claims the values rest on. [`Limits::parse`] refuses,
//! among other things, a key it does not know, a value that is not a finite number above zero, a
//! value without exactly one basis, a copy that differs from its original, a DPS5005 setpoint or
//! current limit above the supply's ceiling, and a fuse that is not the next rating above
//! `max_amps`.
//!
//! The claims live in other records, which this crate does not read, so the Pico enforcer builds
//! from the limits file alone. [`Limits::check_with_claims`] checks what needs them: that each
//! claim cited exists, that a value is no more than the claim's number its `at_most` names, and an
//! upstream voltage within the DPS5005's input rule.

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

/// Each value's basis, keyed by the value's name in its table.
pub type Bases = BTreeMap<String, Basis>;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    pub pico_3v3: Ceiling,
    pub supply: Supply,
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

/// Where a value comes from: exactly one of `policy`, `same_as` and `rule`, and the claims it
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
    /// Each cited as `<part>#<id>` or `<build>/v<n>#<id>`.
    #[serde(default)]
    pub rests_on: Vec<String>,
    /// A number this value may not exceed, as `<claim>.<name>`: a value, in this value's unit, of a
    /// claim it rests on.
    pub at_most: Option<String>,
}

/// A rule a value is held to. Its check is in `check_values` or, when it reads claims,
/// `check_with_claims`, marked with its name. [`Rule::inputs`] names the values that check reads,
/// kept in step by hand, and gives the warehouse its lineage. A rule reads numbers only from the
/// claims its value rests on, so those need no list.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Rule {
    /// The upstream voltage: at least the DPS5005's input floor for its setpoint plus the bench
    /// supply's setting error, and at most its maximum input less that error.
    DpsInput,
    /// The fuse: the next 5 x 20 mm rating above the supply's `max_amps`.
    NextFuseRating,
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

    /// `at_most` as the claim it cites and the name of that claim's value.
    pub fn bound(&self) -> Option<(&str, &str)> {
        self.at_most.as_deref()?.rsplit_once('.')
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
    pub fn dps_input_floor(&self, min_input_volts: f64, input_ratio: f64) -> f64 {
        min_input_volts.max(input_ratio * self.dps.max_setpoint_volts.get())
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

    fn check(&self) -> Result<(), String> {
        self.check_values()?;
        self.check_bases()
    }

    /// Checks what needs the claims the values rest on: that each claim cited exists, that each
    /// value with an `at_most` is no more than the number it names, and each rule that reads
    /// claims. `claim(citation)` gives a claim's values by name, or `None` when there is no such
    /// claim; crates/records passes the catalogue's. A rule reads only the claims its value rests
    /// on, and a name two of them state is refused, since the rule could read either.
    pub fn check_with_claims<'c>(
        &self,
        claim: impl Fn(&str) -> Option<&'c BTreeMap<String, f64>>,
    ) -> Result<(), String> {
        for e in self.entries() {
            let reads = e.basis.rule == Some(Rule::DpsInput);
            let mut read: BTreeMap<&str, (f64, &str)> = BTreeMap::new();
            for c in &e.basis.rests_on {
                let Some(values) = claim(c) else {
                    return Err(format!("{}: rests on {c}, which is no claim", e.path));
                };
                if !reads {
                    continue;
                }
                for (name, v) in values {
                    if let Some((_, first)) = read.insert(name, (*v, c)) {
                        return Err(format!(
                            "{}: {name} is stated by both {first} and {c}",
                            e.path
                        ));
                    }
                }
            }
            if let Some((c, name)) = e.basis.bound()
                && let Value::Volts(v) | Value::Amps(v) = e.value
            {
                let Some(&most) = claim(c).and_then(|values| values.get(name)) else {
                    return Err(format!(
                        "{}: at_most names {name}, which {c} does not state",
                        e.path
                    ));
                };
                if v > most {
                    return Err(format!("{}: is {v}, above {c}'s {name} of {most}", e.path));
                }
            }
            if reads {
                self.check_dps_input(&e.path, &read)?;
            }
        }
        Ok(())
    }

    /// Rule::DpsInput, reading the claims `at` rests on: their values by name, and who states each.
    fn check_dps_input(&self, at: &str, read: &BTreeMap<&str, (f64, &str)>) -> Result<(), String> {
        let get = |name: &str| {
            read.get(name).copied().ok_or_else(|| {
                format!("{at}: rule dps-input reads {name}, which no claim it rests on states")
            })
        };
        let volts = |name: &str| {
            let (v, by) = get(name)?;
            Volts::try_from(v).map_err(|e| format!("{by}: {name}: {e}"))
        };
        let (min, max, err) = (
            volts("min_input_volts")?,
            volts("max_input_volts")?,
            volts("setting_error_volts")?.get(),
        );
        let (ratio, by) = get("input_ratio")?;
        if !(ratio > 1.0 && ratio.is_finite()) {
            return Err(format!("{by}: input_ratio is {ratio}, not above 1"));
        }
        if min >= max {
            return Err(format!(
                "{}: min_input_volts {min} is not below max_input_volts {max}",
                get("min_input_volts")?.1
            ));
        }
        let s = &self.supply;
        let (volts, least) = (
            s.upstream.volts.get(),
            s.dps_input_floor(min.get(), ratio) + err,
        );
        if volts < least {
            return Err(format!(
                "[supply.upstream] volts is {volts} V, below the {least} V the DPS5005 needs to \
                 put out {}: at least {min}, and {ratio} x its output, plus the bench supply's \
                 setting error of {err} V",
                s.dps.max_setpoint_volts
            ));
        }
        if volts + err > max.get() {
            return Err(format!(
                "[supply.upstream] volts is {volts} V, which with the bench supply's setting \
                 error of {err} V can pass the DPS5005's maximum input of {max}"
            ));
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
            for (i, c) in b.rests_on.iter().enumerate() {
                let cited = c
                    .split_once('#')
                    .is_some_and(|(on, id)| !on.is_empty() && !id.is_empty() && !id.contains('#'));
                if !cited {
                    return Err(format!(
                        "{at}: rests on {c:?}, which is not a claim's citation, <subject>#<id>"
                    ));
                }
                if b.rests_on[..i].contains(c) {
                    return Err(format!("{at}: rests on {c} twice"));
                }
            }
            if let Some(at_most) = &b.at_most {
                let Some((c, name)) = b.bound().filter(|(c, n)| !c.is_empty() && !n.is_empty())
                else {
                    return Err(format!(
                        "{at}: at_most {at_most:?} is not <claim>.<value name>"
                    ));
                };
                if !b.rests_on.iter().any(|r| r == c) {
                    return Err(format!("{at}: at_most names {c}, so it must rest on it"));
                }
                let unit = match e.value {
                    Value::Volts(_) => "volts",
                    Value::Amps(_) => "amps",
                    Value::Choice(_) => return Err(format!("{at}: a choice has no at_most")),
                };
                if name.rsplit('_').next() != Some(unit) {
                    return Err(format!(
                        "{at}: at_most names {name}, which is not in {unit} like this value"
                    ));
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
            if let Some(r) = b.rule
                && r.target() != at
            {
                return Err(format!(
                    "{at}: rule {} is for {}, not this",
                    r.as_str(),
                    r.target()
                ));
            }
        }
        // Every rule's value is held to it, so its basis must say so, or its grade would leave out
        // the claims the rule reads.
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

    type Claims = BTreeMap<String, BTreeMap<String, f64>>;

    /// Claims like the ones the repo's limits cite, each with its values by name.
    fn claims() -> Claims {
        let values = |vs: &[(&str, f64)]| vs.iter().map(|(k, v)| (k.to_string(), *v)).collect();
        BTreeMap::from([
            (
                "DPS5005#input-range".into(),
                values(&[
                    ("min_input_volts", 6.0),
                    ("max_input_volts", 55.0),
                    ("input_ratio", 1.1),
                ]),
            ),
            ("DPS5005#current-limit".into(), values(&[])),
            (
                "RP2350#io-supply".into(),
                values(&[("max_io_supply_volts", 3.63)]),
            ),
            (
                "bench-supply#setting-error".into(),
                values(&[("setting_error_volts", 0.5)]),
            ),
        ])
    }

    /// `claims()` with one value set.
    fn claims_with(claim: &str, name: &str, v: f64) -> Claims {
        let mut c = claims();
        c.get_mut(claim).unwrap().insert(name.into(), v);
        c
    }

    /// Parses `text` and checks it against `claims`, as crates/records does with the catalogue's.
    fn refused_with(text: &str, claims: &Claims, because: &str) {
        let err = Limits::parse(text)
            .and_then(|l| l.check_with_claims(|c| claims.get(c)))
            .err()
            .unwrap_or_else(|| panic!("accepted, but {because}"));
        assert!(err.contains(because), "refused for another reason: {err}");
    }

    #[test]
    fn the_repo_limits_hold_with_their_claims() {
        let claims = claims();
        let l = Limits::parse(&repo()).unwrap();
        l.check_with_claims(|c| claims.get(c)).unwrap();
    }

    #[test]
    fn the_repo_limits_load() {
        Limits::load(&root()).expect("lab/limits.toml loads");
    }

    /// A policy that cites CLAUDE.md quotes it, as `CLAUDE.md, "<words>"`, and the words must be
    /// there: no policy may say CLAUDE.md says something without quoting it.
    #[test]
    fn every_policy_quoting_claude_md_matches_it() {
        let claude = fs::read_to_string(root().join("CLAUDE.md")).unwrap();
        let l = Limits::parse(&repo()).unwrap();
        let mut quoted = Vec::new();
        for e in l.entries() {
            let Some(p) = e
                .basis
                .policy
                .as_deref()
                .filter(|p| p.contains("CLAUDE.md"))
            else {
                continue;
            };
            let Some(quote) = p
                .strip_prefix("CLAUDE.md, \"")
                .and_then(|q| q.strip_suffix('"'))
            else {
                panic!("{}: cites CLAUDE.md without quoting it: {p:?}", e.path);
            };
            assert!(
                claude.contains(quote),
                "{}: CLAUDE.md does not say {quote:?}",
                e.path
            );
            quoted.push(e.path);
        }
        for ceiling in ["pico_3v3.max_volts", "pico_3v3.max_amps"] {
            assert!(
                quoted.iter().any(|q| q == ceiling),
                "{ceiling} quotes CLAUDE.md"
            );
        }
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
                    toml::Value::Table(_) if k == "basis" => {}
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
    }

    #[test]
    fn a_value_must_be_finite_and_above_zero() {
        let key = "max_setpoint_volts = 3.4";
        for bad in ["inf", "nan", "0.0", "-3.4"] {
            refused(
                &with(key, &format!("max_setpoint_volts = {bad}")),
                "must be a finite number above 0",
            );
        }
    }

    #[test]
    fn the_dps_stays_within_the_ceiling() {
        refused(
            &with("max_setpoint_volts = 3.4", "max_setpoint_volts = 3.5"),
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
            refused_with(
                &with("volts = 6.5", &format!("volts = {low}")),
                &claims(),
                "the DPS5005 needs",
            );
        }
        for high in ["54.6", "56"] {
            refused_with(
                &with("volts = 6.5", &format!("volts = {high}")),
                &claims(),
                "maximum input",
            );
        }
        let raised = claims_with("DPS5005#input-range", "min_input_volts", 6.1);
        refused_with(&repo(), &raised, "the DPS5005 needs");
        let worse = claims_with("bench-supply#setting-error", "setting_error_volts", 0.6);
        refused_with(&repo(), &worse, "the DPS5005 needs");
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
                "basis.speed = { rests_on = [\"DPS5005#input-range\"] }",
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

    const UPSTREAM_RESTS_ON: &str =
        "rests_on = [\"DPS5005#input-range\", \"bench-supply#setting-error\"]";

    #[test]
    fn a_rule_reads_only_the_claims_its_value_rests_on() {
        refused_with(
            &with(UPSTREAM_RESTS_ON, "rests_on = [\"DPS5005#input-range\"]"),
            &claims(),
            "reads setting_error_volts, which no claim it rests on states",
        );
        let twice = claims_with("bench-supply#setting-error", "input_ratio", 1.2);
        refused_with(
            &repo(),
            &twice,
            "input_ratio is stated by both DPS5005#input-range and bench-supply#setting-error",
        );
        // A value no rule reads may rest on claims that state the same name.
        let mut shared = claims();
        shared.insert(
            "RP2350#x".into(),
            BTreeMap::from([("min_input_volts".into(), 1.8)]),
        );
        let text = with(
            "basis.reenable = { policy = \"Jon, 2026-09-28\" }",
            "basis.reenable = { policy = \"Jon, 2026-09-28\", rests_on = [\"DPS5005#input-range\", \"RP2350#x\"] }",
        );
        let l = Limits::parse(&text).unwrap();
        l.check_with_claims(|c| shared.get(c)).unwrap();
    }

    #[test]
    fn a_value_rests_on_claims_that_exist() {
        let current = "rests_on = [\"DPS5005#current-limit\"]";
        refused_with(
            &with(current, "rests_on = [\"DPS5005#current-limt\"]"),
            &claims(),
            "rests on DPS5005#current-limt, which is no claim",
        );
        for bad in ["DPS5005", "#input-range", "DPS5005#", "a#b#c"] {
            refused(
                &with(current, &format!("rests_on = [\"{bad}\"]")),
                "which is not a claim's citation",
            );
        }
        refused(
            &with(
                UPSTREAM_RESTS_ON,
                "rests_on = [\"DPS5005#input-range\", \"bench-supply#setting-error\", \"DPS5005#input-range\"]",
            ),
            "rests on DPS5005#input-range twice",
        );
    }

    #[test]
    fn a_rule_s_value_names_it() {
        refused(
            &with(
                "basis.volts = { rule = \"dps-input\", rests_on = [\"DPS5005#input-range\", \"bench-supply#setting-error\"] }",
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
    fn the_dps_input_claim_is_self_consistent() {
        let at = "DPS5005#input-range";
        for (name, v, because) in [
            (
                "input_ratio",
                0.11,
                "DPS5005#input-range: input_ratio is 0.11, not above 1",
            ),
            ("input_ratio", 1.0, "not above 1"),
            ("input_ratio", f64::NAN, "not above 1"),
            (
                "max_input_volts",
                5.0,
                "min_input_volts 6 V is not below max_input_volts 5 V",
            ),
            (
                "min_input_volts",
                -6.0,
                "DPS5005#input-range: min_input_volts: volts must be",
            ),
        ] {
            refused_with(&repo(), &claims_with(at, name, v), because);
        }
    }

    #[test]
    fn no_value_comes_from_itself() {
        let text = with(
            "basis.max_volts.policy = \"CLAUDE.md, \\\"Default ceilings: 3.4 V and 200 mA\\\"\"",
            "basis.max_volts.same_as = \"supply.max_volts\"",
        );
        refused(&text, "comes from itself");
    }

    const PICO_AT_MOST: &str = "basis.max_volts.at_most = \"RP2350#io-supply.max_io_supply_volts\"";

    #[test]
    fn a_value_stays_at_most_the_number_it_names() {
        let lower = claims_with("RP2350#io-supply", "max_io_supply_volts", 3.3);
        refused_with(
            &repo(),
            &lower,
            "pico_3v3.max_volts: is 3.4, above RP2350#io-supply's max_io_supply_volts of 3.3",
        );
        let mut unstated = claims();
        unstated.get_mut("RP2350#io-supply").unwrap().clear();
        refused_with(
            &repo(),
            &unstated,
            "at_most names max_io_supply_volts, which RP2350#io-supply does not state",
        );
        // The bound is on a claim the value rests on, in its unit, and a number is bounded.
        refused(
            &with(
                "basis.max_volts.rests_on = [\"RP2350#io-supply\"]\n",
                "basis.max_volts.rests_on = [\"DPS5005#input-range\"]\n",
            ),
            "at_most names RP2350#io-supply, so it must rest on it",
        );
        refused(
            &with(
                PICO_AT_MOST,
                "basis.max_volts.at_most = \"RP2350#io-supply.max_io_supply_amps\"",
            ),
            "not in volts like this value",
        );
        for bad in [
            "RP2350#io-supply",
            "RP2350#io-supply.",
            ".max_io_supply_volts",
        ] {
            refused(
                &with(
                    PICO_AT_MOST,
                    &format!("basis.max_volts.at_most = \"{bad}\""),
                ),
                "is not <claim>.<value name>",
            );
        }
        refused(
            &with(
                "basis.reenable = { policy = \"Jon, 2026-09-28\" }",
                "basis.reenable = { policy = \"Jon, 2026-09-28\", rests_on = [\"RP2350#io-supply\"], at_most = \"RP2350#io-supply.max_io_supply_volts\" }",
            ),
            "a choice has no at_most",
        );
    }
}
