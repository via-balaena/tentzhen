//! Measurement records: one file per experiment on the bench, as CLAUDE.md asks ("Every experiment
//! gets a record: hardware revision, gateware hash, firmware hash, toolchain versions, setup
//! notes, results"). Each is `lab/records/<id>.toml`, and a claim's `record` referent names it by
//! its id: the UTC day of its first log entry, then words, as in `2026-10-14-dps5005-draw`.
//!
//! ```toml
//! measures = "What was measured, in a sentence"
//! setup = "How it was wired and set"
//! log = { first = "<sha256 of its first lab-log line>", last = "<sha256 of its last>" }
//! toolchain = { tentzhen = "28efafe" }
//!
//! [[device]]                    # what was measured: a part, or a build and its version
//! part = "DPS5005"
//! revision = "<as marked on the board>"
//! firmware_sha256 = "<the sha256 of the image it runs>"
//!
//! [[meter]]                     # what read it, and the claim on its record giving its accuracy
//! part = "<the meter's part>"
//! accuracy = "<the meter's part>#<claim id>"
//!
//! [results]                     # a number's key ends in its unit; a raw ADC code in `codes`,
//! input_amps = 0.041            # beside the value converted from it
//! ```
//!
//! A record cites the log entries it came from by the sha256 of their lines, so it cannot be
//! matched to a log rewritten under it. Once merged it never changes: the Quality Gate's step "A
//! measurement record never changes". Like the log's, every field is what its writer says.

use crate::log::{self, Datum};
use crate::{Catalogue, Source, plain, read_folder, sha256_shaped, unit_of};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

/// Where the records live, relative to the repo root.
pub const RECORDS: &str = "lab/records";

/// One experiment on the bench.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Measurement {
    /// What was measured, in a sentence.
    pub measures: String,
    /// How it was wired and set.
    pub setup: String,
    /// The lab-log entries it came from.
    pub log: Span,
    /// The tools it used, by name, and their versions.
    #[serde(default)]
    pub toolchain: BTreeMap<String, String>,
    /// What was measured. At least one.
    pub device: Vec<Device>,
    /// What read it. At least one when a result is a number.
    #[serde(default)]
    pub meter: Vec<Device>,
    /// What it found: numbers, words or yes/no, keyed as the log's are. A raw ADC code
    /// (`input_codes`) sits beside the value converted from it (`input_volts`).
    pub results: BTreeMap<String, Datum>,
    /// Its id, from the file's name. Set on load, never written in a record.
    #[serde(skip)]
    pub id: String,
    /// The file it came from, relative to the repo root. Set on load, never written in a record.
    #[serde(skip)]
    pub record: String,
}

/// The first and last lab-log entries of an experiment, each by the sha256 of its line.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Span {
    pub first: String,
    pub last: String,
}

/// A device measured, or a meter that read it: a part, or a build version.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Device {
    pub part: Option<String>,
    pub build: Option<String>,
    pub version: Option<u32>,
    /// A part's hardware revision, as marked on it. A build's is its version.
    pub revision: Option<String>,
    /// The sha256 of the firmware it ran, if it runs any. A build version that pins its firmware
    /// ran that firmware, or it was not that version.
    pub firmware_sha256: Option<String>,
    /// The sha256 of the gateware it ran, if it runs any.
    pub gateware_sha256: Option<String>,
    /// A meter's: the claim on its own record that gives its accuracy, cited as a claim is.
    pub accuracy: Option<String>,
}

impl Device {
    /// The record it is, as a claim on it is cited before its `#`: `DPS5005`, or `supply/v1`.
    pub fn subject(&self) -> Option<String> {
        match (&self.part, &self.build, self.version) {
            (Some(p), None, None) => Some(p.clone()),
            (None, Some(b), Some(v)) => Some(format!("{b}/v{v}")),
            _ => None,
        }
    }
}

/// The records' files under `root`, by name: none until the first experiment.
pub fn read(root: &Path) -> Result<Vec<Source>, String> {
    read_folder(root, RECORDS)
}

/// A record's id, from its path: `lab/records/2026-10-14-dps5005-draw.toml` is
/// `2026-10-14-dps5005-draw`.
fn id_of(path: &str) -> Option<&str> {
    let id = path
        .strip_prefix(RECORDS)?
        .strip_prefix('/')?
        .strip_suffix(".toml")?;
    let words = id.get(10..)?.strip_prefix('-')?;
    (log::real_day(id.get(..10)?) && plain(words)).then_some(id)
}

/// Parses a record and holds it to its shape. What it names, in the catalogue and the log, is
/// checked by [`Catalogue::with_measurements`].
pub fn parse(s: &Source) -> Result<Measurement, String> {
    let at = s.path.as_str();
    let id = id_of(at).ok_or_else(|| {
        format!(
            "{at}: a measurement record is named {RECORDS}/<yyyy-mm-dd>-<words>.toml, by the UTC \
             day of its first log entry, the words in lowercase letters, digits and hyphens"
        )
    })?;
    let mut m: Measurement = toml::from_str(&s.text).map_err(|e| format!("{at}: {e}"))?;
    m.id = id.into();
    m.record = at.into();
    if m.measures.is_empty() || m.setup.is_empty() {
        return Err(format!("{at}: say what it measures and how it was set up"));
    }
    if !sha256_shaped(&m.log.first) || !sha256_shaped(&m.log.last) {
        return Err(format!(
            "{at}: log.first and log.last are the sha256s of lab-log lines, in lowercase hex"
        ));
    }
    for (tool, version) in &m.toolchain {
        if !plain(tool) || version.is_empty() {
            return Err(format!(
                "{at}: toolchain {tool:?} must name a tool in lowercase letters, digits and \
                 hyphens, and give its version"
            ));
        }
    }
    if m.device.is_empty() {
        return Err(format!("{at}: name what it measured under [[device]]"));
    }
    for d in &m.device {
        check_device(at, "device", d)?;
        if d.accuracy.is_some() {
            return Err(format!(
                "{at}: a device measured has no accuracy here; a meter does"
            ));
        }
    }
    for d in &m.meter {
        check_device(at, "meter", d)?;
        if d.accuracy.is_none() {
            return Err(format!(
                "{at}: meter {} names no accuracy: the claim on its record that gives it",
                d.subject().unwrap_or_default()
            ));
        }
    }
    log::check_data(at, "results", &m.results)?;
    if m.results.is_empty() {
        return Err(format!("{at}: the results are empty; say what it found"));
    }
    for key in m.results.keys() {
        let Some(stem) = key.strip_suffix("_codes") else {
            continue;
        };
        let converted = m.results.keys().any(|k| {
            unit_of(k).is_some_and(|unit| unit != "codes" && *k == format!("{stem}_{unit}"))
        });
        if !converted {
            return Err(format!(
                "{at}: results {key} is a raw ADC code, so the value converted from it sits \
                 beside it, as {stem}_<unit>"
            ));
        }
    }
    let numbers = m.results.values().any(|d| matches!(d, Datum::Number(_)));
    if numbers && m.meter.is_empty() {
        return Err(format!(
            "{at}: a result is a number, so name the meter that read it under [[meter]]"
        ));
    }
    Ok(m)
}

/// Holds a device or a meter to its shape.
fn check_device(at: &str, role: &str, d: &Device) -> Result<(), String> {
    let Some(subject) = d.subject() else {
        return Err(format!(
            "{at}: a {role} names a part, or a build and its version"
        ));
    };
    let strings = [&d.part, &d.build, &d.revision, &d.accuracy];
    if strings.iter().any(|s| s.as_deref() == Some("")) {
        return Err(format!("{at}: {role} {subject} has an empty string"));
    }
    if d.build.is_some() && d.revision.is_some() {
        return Err(format!(
            "{at}: {role} {subject} is a build, whose revision is its version"
        ));
    }
    for (name, h) in [
        ("firmware_sha256", &d.firmware_sha256),
        ("gateware_sha256", &d.gateware_sha256),
    ] {
        if h.as_deref().is_some_and(|h| !sha256_shaped(h)) {
            return Err(format!(
                "{at}: {role} {subject}'s {name} must be a sha256 in lowercase hex"
            ));
        }
    }
    Ok(())
}

impl Measurement {
    /// Holds what it names to `cat`: each device's and meter's record, the firmware a build version
    /// pins, each meter's accuracy as a claim on its own record, and its first and last log
    /// entries, in order, the first on the day its id starts with.
    pub(crate) fn check_in(&self, cat: &Catalogue) -> Result<(), String> {
        let at = &self.record;
        let roles = self.device.iter().map(|d| ("device", d));
        for (role, d) in roles.chain(self.meter.iter().map(|d| ("meter", d))) {
            let subject = d.subject().unwrap_or_default();
            let known = match (&d.part, &d.build, d.version) {
                (Some(p), _, _) => cat.parts.contains_key(p),
                (None, Some(b), Some(v)) => cat.version(b, v).is_some(),
                _ => false,
            };
            if !known {
                return Err(format!(
                    "{at}: {role} {subject} has no record in parts/ or builds/"
                ));
            }
            // A build version pins its firmware: running other firmware, it is not that version.
            let pinned = match (&d.build, d.version) {
                (Some(b), Some(v)) => cat.version(b, v).and_then(|bv| bv.firmware.as_ref()),
                _ => None,
            };
            if let Some(f) = pinned
                && d.firmware_sha256.as_deref() != Some(f.sha256.as_str())
            {
                return Err(format!(
                    "{at}: {role} {subject} pins its firmware, so its firmware_sha256 is that \
                     firmware's, {}",
                    f.sha256
                ));
            }
            if let Some(acc) = &d.accuracy {
                let on_it = acc.split_once('#').is_some_and(|(s, _)| s == subject);
                if !on_it || cat.claim(acc).is_none() {
                    return Err(format!(
                        "{at}: meter {subject}'s accuracy {acc:?} is no claim on {subject}"
                    ));
                }
            }
        }
        let entry = |end: &str, sha: &str| {
            cat.log
                .iter()
                .find(|e| e.sha256 == sha)
                .ok_or_else(|| format!("{at}: log.{end} names no entry in the lab log"))
        };
        let (first, last) = (
            entry("first", &self.log.first)?,
            entry("last", &self.log.last)?,
        );
        if first.seq > last.seq {
            return Err(format!(
                "{at}: log.first is entry {}, after log.last, entry {}",
                first.seq, last.seq
            ));
        }
        let day = &first.at[..10];
        if self.id.get(..10) != Some(day) {
            return Err(format!(
                "{at}: its first log entry is on {day}, so its name starts {day}"
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log::Entry;

    fn src(path: &str, text: &str) -> Source {
        Source {
            path: path.into(),
            text: text.into(),
        }
    }

    const PATH: &str = "lab/records/2026-10-14-probe-draw.toml";

    /// A record of the probe, read by the meter, over the log entries `first` to `last`.
    fn record(first: &str, last: &str) -> String {
        format!(
            "measures = \"The probe's draw\"\nsetup = \"On the bench\"\n\
             log = {{ first = \"{first}\", last = \"{last}\" }}\n\
             toolchain = {{ probe-rs = \"0.29.1\" }}\n\n\
             [[device]]\nbuild = \"probe\"\nversion = 1\nfirmware_sha256 = \"{}\"\n\n\
             [[meter]]\npart = \"DMM\"\naccuracy = \"DMM#dc-volts\"\n\n\
             [results]\nread_volts = 3.3\nread_codes = 4095\nenumerates = true\n",
            "b".repeat(64)
        )
    }

    fn shaped() -> String {
        record(&"a".repeat(64), &"a".repeat(64))
    }

    fn refused(path: &str, text: &str, because: &str) {
        let err = parse(&src(path, text)).err().unwrap_or_default();
        assert!(err.contains(because), "{because}: {err}");
    }

    /// `shaped()` with `from` replaced by `to`, which must be in it.
    fn edited(from: &str, to: &str) -> String {
        let text = shaped();
        assert!(text.contains(from), "{from:?} is not in the record");
        text.replace(from, to)
    }

    #[test]
    fn a_measurement_record_keeps_its_shape() {
        let m = parse(&src(PATH, &shaped())).unwrap();
        assert_eq!(m.id, "2026-10-14-probe-draw");
        assert_eq!(m.record, PATH);
        assert_eq!(m.device[0].subject().as_deref(), Some("probe/v1"));
        assert_eq!(m.results["read_codes"], Datum::Number(4095.0));
        for bad in [
            "lab/records/probe-draw.toml",
            "lab/records/2026-10-14.toml",
            "lab/records/2026-10-14-.toml",
            "lab/records/2026-02-30-probe.toml",
            "lab/records/2026-10-14-Probe.toml",
            "lab/records/2026-10-14-probe.json",
            "lab/records/x/2026-10-14-probe.toml",
            "lab/2026-10-14-probe.toml",
        ] {
            refused(
                bad,
                &shaped(),
                "is named lab/records/<yyyy-mm-dd>-<words>.toml",
            );
        }
        let at =
            |edit: (&str, &str), because: &str| refused(PATH, &edited(edit.0, edit.1), because);
        at(
            ("measures", "grade = \"measured\"\nmeasures"),
            "unknown field `grade`",
        );
        at(
            ("version = 1\n", "version = 1\nserial = \"x\"\n"),
            "unknown field `serial`",
        );
        at(
            (" }\ntoolchain", ", extra = 1 }\ntoolchain"),
            "unknown field `extra`",
        );
        at(("\"The probe's draw\"", "\"\""), "say what it measures");
        at(("\"On the bench\"", "\"\""), "say what it measures");
        at(
            ("first = \"aaaa", "first = \"Aaaa"),
            "log.first and log.last are the sha256s",
        );
        at(
            ("last = \"aaaa", "last = \"aaa"),
            "log.first and log.last are the sha256s",
        );
        at(
            ("probe-rs = \"0.29.1\"", "\"Probe RS\" = \"0.29.1\""),
            "toolchain \"Probe RS\"",
        );
        at(
            ("probe-rs = \"0.29.1\"", "probe-rs = \"\""),
            "toolchain \"probe-rs\"",
        );
        let device = "[[device]]\nbuild = \"probe\"\nversion = 1\n";
        let gone = edited(
            &format!("{device}firmware_sha256 = \"{}\"\n\n", "b".repeat(64)),
            "",
        );
        refused(PATH, &gone, "missing field `device`");
        let tools = "toolchain = { probe-rs = \"0.29.1\" }\n";
        let none = gone.replace(tools, &format!("{tools}device = []\n"));
        assert_ne!(none, gone);
        refused(PATH, &none, "name what it measured under [[device]]");
        at(
            (device, "[[device]]\nbuild = \"probe\"\n"),
            "a device names a part, or a build",
        );
        at(
            (device, "[[device]]\npart = \"DMM\"\nversion = 1\n"),
            "a device names a part, or a build",
        );
        at(
            (
                device,
                "[[device]]\npart = \"DMM\"\nbuild = \"probe\"\nversion = 1\n",
            ),
            "a device names a part, or a build",
        );
        at(
            (device, "[[device]]\nversion = 1\n"),
            "a device names a part, or a build",
        );
        at(
            (device, format!("{device}revision = \"B\"\n").as_str()),
            "is a build, whose revision is its version",
        );
        at(
            (device, "[[device]]\npart = \"DMM\"\nrevision = \"\"\n"),
            "device DMM has an empty string",
        );
        at(
            (
                device,
                format!("{device}accuracy = \"probe/v1#x\"\n").as_str(),
            ),
            "a device measured has no accuracy",
        );
        at(
            ("firmware_sha256 = \"bbbb", "firmware_sha256 = \"bbb"),
            "firmware_sha256 must be a sha256",
        );
        at(
            ("version = 1\n", "version = 1\ngateware_sha256 = \"x\"\n"),
            "gateware_sha256 must be a sha256",
        );
        at(
            ("accuracy = \"DMM#dc-volts\"\n", ""),
            "meter DMM names no accuracy",
        );
        at(
            ("accuracy = \"DMM#dc-volts\"", "accuracy = \"\""),
            "meter DMM has an empty string",
        );
        at(
            ("read_volts = 3.3", "read = 3.3"),
            "results read is a number, so its key must end in its unit",
        );
        at(
            ("read_volts = 3.3", "read_volts = \"high\""),
            "results read_volts ends in volts, so it must be a number",
        );
        at(
            (
                "[results]\nread_volts = 3.3\nread_codes = 4095\nenumerates = true\n",
                "[results]\n",
            ),
            "the results are empty",
        );
        at(
            (
                "[results]\nread_volts = 3.3\nread_codes = 4095\nenumerates = true\n",
                "",
            ),
            "missing field `results`",
        );
        at(
            (
                "[[meter]]\npart = \"DMM\"\naccuracy = \"DMM#dc-volts\"\n\n",
                "",
            ),
            "name the meter that read it",
        );
        // Words and yes/no need no meter: what a person or a host saw, such as a USB device.
        let seen = edited(
            "[[meter]]\npart = \"DMM\"\naccuracy = \"DMM#dc-volts\"\n\n",
            "",
        )
        .replace(
            "read_volts = 3.3\nread_codes = 4095\n",
            "product = \"Debugprobe\"\n",
        );
        assert!(parse(&src(PATH, &seen)).is_ok(), "the control is accepted");
    }

    #[test]
    fn a_raw_code_sits_beside_its_converted_value() {
        let because =
            "results read_codes is a raw ADC code, so the value converted from it sits beside it";
        for (from, to) in [
            ("read_volts = 3.3\n", ""),
            ("read_volts", "input_volts"),
            ("read_volts", "read_max_volts"),
            ("read_volts", "read_codes_volts"),
            ("read_volts = 3.3", "read = \"3.3 V\""),
        ] {
            refused(PATH, &edited(from, to), because);
        }
        for (from, to) in [
            ("read_volts", "read_amps"),
            ("read_volts = 3.3", "read_volts = 3.3\nread_ohms = 2"),
        ] {
            assert!(
                parse(&src(PATH, &edited(from, to))).is_ok(),
                "{from} → {to}"
            );
        }
    }

    /// A lab log of `n` entries, one a day from 2026-10-14, each line as `tentzhen log append`
    /// writes it.
    fn lab_log(n: u64) -> Vec<Source> {
        let mut prev = None;
        (1..=n)
            .map(|seq| {
                let day = format!("2026-10-{}", 13 + seq);
                let e = Entry {
                    seq,
                    at: format!("{day}T10:00:00Z"),
                    by: "person:jon".into(),
                    what: "supply.read".into(),
                    params: BTreeMap::new(),
                    result: BTreeMap::from([("read_volts".into(), Datum::Number(3.3))]),
                    limits_sha256: "a".repeat(64),
                    prev: prev.take(),
                    sha256: String::new(),
                    record: String::new(),
                };
                let line = serde_json::to_string(&e).unwrap();
                prev = Some(log::sha256_hex(line.as_bytes()));
                src(&format!("{}/{day}.jsonl", log::LOG), &format!("{line}\n"))
            })
            .collect()
    }

    /// A catalogue with a meter part, the RP2350 and a probe build that pins its firmware, each
    /// with a claim, and a lab log of three entries.
    fn catalogue() -> Catalogue {
        citing("", "")
    }

    /// [`catalogue`], with these claims added to the RP2350 and to the probe.
    fn citing(on_rp2350: &str, on_probe: &str) -> Catalogue {
        records(on_rp2350, on_probe).with_log(lab_log(3)).unwrap()
    }

    /// [`catalogue`] with another log.
    fn catalogue_with_log(log: Vec<Source>) -> Catalogue {
        records("", "").with_log(log).unwrap()
    }

    /// [`catalogue`]'s records, with no log yet.
    fn records(on_rp2350: &str, on_probe: &str) -> Catalogue {
        let base = src(
            crate::TRUSTED_BASE,
            "[[entry]]\nid = \"entry\"\nassumes = \"x\"\n",
        );
        let claim =
            |id: &str| format!("\n[[claim]]\nid = \"{id}\"\nsays = \"x\"\ntrusted = \"entry\"\n");
        let dmm = format!(
            "part = \"DMM\"\nkind = \"product\"\nis = \"multimeter\"\n{}",
            claim("dc-volts")
        );
        let rp2350 = format!(
            "part = \"RP2350\"\nkind = \"chip\"\nis = \"microcontroller\"\n{}{on_rp2350}",
            claim("io")
        );
        let firmware = format!(
            "\n[firmware]\nname = \"f\"\nlicense = \"MIT\"\nsource = \"s\"\nrelease = \"r\"\n\
             file = \"f\"\nsha256 = \"{}\"\n",
            "b".repeat(64)
        );
        let probe = format!(
            "build = \"probe\"\nversion = 1\nstatus = \"draft\"\ndoes = \"a thing\"\n{firmware}{}{on_probe}",
            claim("reads")
        );
        Catalogue::from_sources(
            &base,
            &[
                src("parts/dmm.toml", &dmm),
                src("parts/rp2350.toml", &rp2350),
            ],
            &[src("builds/probe/v1.toml", &probe)],
        )
        .unwrap()
    }

    /// The sha256 of log entry `seq`'s line.
    fn sha(cat: &Catalogue, seq: usize) -> String {
        cat.log[seq - 1].sha256.clone()
    }

    #[test]
    fn a_record_names_what_the_catalogue_and_its_log_hold() {
        let cat = catalogue();
        let good = record(&sha(&cat, 1), &sha(&cat, 2));
        let with = |path: &str, text: &str| catalogue().with_measurements(vec![src(path, text)]);
        let m = with(PATH, &good).unwrap();
        assert_eq!(
            m.measurements["2026-10-14-probe-draw"].log.last,
            sha(&cat, 2)
        );
        assert!(m.sources.iter().any(|s| s.path == PATH), "bronze holds it");
        let refused = |path: &str, text: String, because: &str| {
            let err = with(path, &text).err().unwrap_or_default();
            assert!(err.contains(because), "{because}: {err}");
        };
        let edit = |from: &str, to: &str| {
            assert!(good.contains(from), "{from:?}");
            good.replace(from, to)
        };
        refused(
            PATH,
            edit("build = \"probe\"\nversion = 1", "part = \"RP9999\""),
            "device RP9999 has no record",
        );
        refused(
            PATH,
            edit("version = 1", "version = 2"),
            "device probe/v2 has no record",
        );
        refused(
            PATH,
            edit("part = \"DMM\"", "part = \"DMM2\""),
            "meter DMM2 has no record",
        );
        refused(
            PATH,
            edit("DMM#dc-volts", "DMM#nope"),
            "meter DMM's accuracy \"DMM#nope\" is no claim on DMM",
        );
        refused(
            PATH,
            edit("DMM#dc-volts", "RP2350#io"),
            "meter DMM's accuracy \"RP2350#io\" is no claim on DMM",
        );
        let nothing = "f".repeat(64);
        refused(
            PATH,
            record(&nothing, &sha(&cat, 2)),
            "log.first names no entry in the lab log",
        );
        refused(
            PATH,
            record(&sha(&cat, 1), &nothing),
            "log.last names no entry in the lab log",
        );
        refused(
            PATH,
            record(&sha(&cat, 2), &sha(&cat, 1)),
            "log.first is entry 2, after log.last, entry 1",
        );
        refused(
            "lab/records/2026-10-15-probe-draw.toml",
            good.clone(),
            "its first log entry is on 2026-10-14, so its name starts 2026-10-14",
        );
        let twice = catalogue().with_measurements(vec![src(PATH, &good), src(PATH, &good)]);
        assert!(
            twice
                .err()
                .unwrap_or_default()
                .contains("record 2026-10-14-probe-draw is given twice")
        );
        // A meter may be a build, with its accuracy on that build version; one entry is a span.
        // A build version is the firmware it pins, too.
        let pin = format!("firmware_sha256 = \"{}\"\n", "b".repeat(64));
        let pins = "device probe/v1 pins its firmware, so its firmware_sha256 is that firmware's";
        refused(PATH, edit(&pin, &pin.replace('b', "c")), pins);
        refused(PATH, edit(&pin, ""), pins);
        let built = edit(
            "part = \"DMM\"\naccuracy = \"DMM#dc-volts\"",
            &format!("build = \"probe\"\nversion = 1\naccuracy = \"probe/v1#reads\"\n{pin}"),
        );
        assert!(with(PATH, &built).is_ok(), "a build as a meter");
        let meter = "accuracy = \"probe/v1#reads\"\n";
        let unpinned = built.replace(&format!("{meter}{pin}"), meter);
        assert_ne!(unpinned, built);
        let err = with(PATH, &unpinned).err().unwrap_or_default();
        assert!(err.contains("meter probe/v1 pins its firmware"), "{err}");
        // The log rewritten under the record, from its first entry on: each line after the edit
        // names the new line before it, so the log loads, and the record's entries are gone.
        let mut rewritten = lab_log(3);
        let mut prev: Option<String> = None;
        for file in &mut rewritten {
            let mut e: Entry = serde_json::from_str(file.text.trim_end()).unwrap();
            if e.seq == 1 {
                e.result.insert("read_volts".into(), Datum::Number(3.2));
            }
            e.prev = prev.take();
            let line = serde_json::to_string(&e).unwrap();
            prev = Some(log::sha256_hex(line.as_bytes()));
            file.text = format!("{line}\n");
        }
        let under = catalogue_with_log(rewritten).with_measurements(vec![src(PATH, &good)]);
        let err = under.err().unwrap_or_default();
        assert!(
            err.contains("log.first names no entry in the lab log"),
            "{err}"
        );
        let one = record(&sha(&cat, 2), &sha(&cat, 2));
        assert!(
            with("lab/records/2026-10-15-probe-draw.toml", &one).is_ok(),
            "a span of one entry"
        );
    }

    #[test]
    fn a_record_referent_names_a_record_that_measured_its_subject() {
        let good = {
            let cat = catalogue();
            record(&sha(&cat, 1), &sha(&cat, 2))
        };
        let draws =
            "\n[[claim]]\nid = \"draws\"\nsays = \"x\"\nrecord = \"2026-10-14-probe-draw\"\n";
        let refused = |cat: Catalogue, records: Vec<Source>, because: &str| {
            let err = cat.with_measurements(records).err().unwrap_or_default();
            assert!(err.contains(because), "{because}: {err}");
        };
        refused(
            citing("", draws),
            Vec::new(),
            "builds/probe/v1.toml: claim draws cites record \"2026-10-14-probe-draw\", which is no \
             file in lab/records",
        );
        let cat = citing("", draws).with_measurements(vec![src(PATH, &good)]);
        assert!(cat.is_ok(), "the record measured probe/v1");
        refused(
            citing(draws, ""),
            vec![src(PATH, &good)],
            "parts/rp2350.toml: claim draws cites record 2026-10-14-probe-draw, which does not list \
             RP2350 among the devices it measured",
        );
        // A meter is not what was measured.
        let meter = good.replace(
            "part = \"DMM\"\naccuracy = \"DMM#dc-volts\"",
            "part = \"RP2350\"\naccuracy = \"RP2350#io\"",
        );
        assert_ne!(meter, good);
        refused(
            citing(draws, ""),
            vec![src(PATH, &meter)],
            "which does not list RP2350 among the devices it measured",
        );
        let both = good.replace(
            "\n[[meter]]",
            "\n[[device]]\npart = \"RP2350\"\n\n[[meter]]",
        );
        assert_ne!(both, good);
        let cat = citing(draws, "").with_measurements(vec![src(PATH, &both)]);
        assert!(cat.is_ok(), "the record measured the RP2350 too");
    }
}
