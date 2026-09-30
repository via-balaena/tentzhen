//! The lab-target list: the boards an agent may flash without asking a person first. CLAUDE.md
//! asks for the user's approval to "Flash firmware to a device that is not listed as a lab target".
//! The list is `lab/targets.toml`, one `[[target]]` per physical board, never per kind of board:
//!
//! ```toml
//! [[target]]
//! id = "probe-1"              # written on a label on the board
//! build = "debug-probe"       # what it is: a part (part = "RP2350"), or a build and its version
//! version = 1
//! serial = "<the serial it reports>"
//! listed = "2026-10-14"       # the first UTC day it may be flashed
//! approved_by = "person:jon"  # on whose decision: a person's
//! retired = "2026-12-01"      # the UTC day it stops being a target, once it has
//! ```
//!
//! Each flash is logged as [`FLASH`], naming in its params the target, the serial read from the
//! board and the image's sha256. [`Catalogue::with_log`] refuses a flash whose target is not on the
//! list that day, whose serial is not the target's, or whose image is not the firmware the target's
//! build version pins. So a target a flash names stays on the list with the serial it gave:
//! removing it, or changing its serial, refuses that flash. It stops being a target by `retired`.
//!
//! CLAUDE.md also allows a flash of a board that is not listed, on a person's approval in the
//! session. Such a flash names that person in its params, `approved_by = "person:<name>"`, and is
//! not held to the list. The Pico enforcer, which is never listed, is to be flashed that way.
//!
//! Like the log's, every field is its writer's word, a target's or a flash's `approved_by`
//! included.

use crate::log::{Datum, Entry, real_day};
use crate::{Catalogue, Source, plain, read_source, sha256_shaped, subject_of};
use serde::Deserialize;
use std::path::Path;

/// Where the list lives, relative to the repo root.
pub const TARGETS: &str = "lab/targets.toml";

/// The action a flash is logged as.
pub const FLASH: &str = "firmware.flash";

/// One board an agent may flash.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    /// Unique on the list, and written on a label on the board.
    pub id: String,
    /// What it is: a part, or a build and its version.
    pub part: Option<String>,
    pub build: Option<String>,
    pub version: Option<u32>,
    /// The serial the board reports, which no other target's is.
    pub serial: String,
    /// The first UTC day it may be flashed: `2026-10-14`.
    pub listed: String,
    /// On whose decision it was listed: `person:<name>`.
    pub approved_by: String,
    /// The UTC day it stops being a target, after `listed`, once it has.
    pub retired: Option<String>,
    /// The file it came from, relative to the repo root. Set on load, never written in a record.
    #[serde(skip)]
    pub record: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct List {
    #[serde(default)]
    target: Vec<Target>,
}

impl Target {
    /// The record it is, as a claim on it is cited before its `#`: `RP2350`, or `debug-probe/v1`.
    pub fn subject(&self) -> Option<String> {
        subject_of(self.part.as_deref(), self.build.as_deref(), self.version)
    }

    /// Holds what it names to `cat`: its part's or its build version's record.
    pub(crate) fn check_in(&self, cat: &Catalogue) -> Result<(), String> {
        let known = match (&self.part, &self.build, self.version) {
            (Some(p), _, _) => cat.parts.contains_key(p),
            (None, Some(b), Some(v)) => cat.version(b, v).is_some(),
            _ => false,
        };
        if !known {
            return Err(format!(
                "{}: target {} is {}, which has no record in parts/ or builds/",
                self.record,
                self.id,
                self.subject().unwrap_or_default()
            ));
        }
        if let (Some(b), Some(v)) = (&self.build, self.version)
            && let Some((never, why)) = cat.never_a_lab_target(b, v)
        {
            return Err(format!(
                "{}: target {} is {b}/v{v}, which is or uses {}/v{}, never a lab target: {why}",
                self.record, self.id, never.build, never.version
            ));
        }
        Ok(())
    }

    /// The sha256 of the only image it may take: the firmware its build version pins, if it pins
    /// one. A board listed as a part takes any image.
    fn pinned<'a>(&self, cat: &'a Catalogue) -> Option<&'a str> {
        let (build, version) = (self.build.as_ref()?, self.version?);
        let firmware = cat.version(build, version)?.firmware.as_ref()?;
        Some(firmware.sha256.as_str())
    }
}

/// The list's file under `root`.
pub fn read(root: &Path) -> Result<Source, String> {
    read_source(root, &root.join(TARGETS))
}

/// Parses the list and holds each target to its shape. What each names in the catalogue is checked
/// by [`Catalogue::with_targets`].
pub fn parse(s: &Source) -> Result<Vec<Target>, String> {
    let at = s.path.as_str();
    let List { mut target } = toml::from_str(&s.text).map_err(|e| format!("{at}: {e}"))?;
    for (i, t) in target.iter().enumerate() {
        let id = &t.id;
        if !plain(id) {
            return Err(format!(
                "{at}: target id {id:?} must be lowercase letters, digits and hyphens"
            ));
        }
        if target[..i].iter().any(|u| u.id == *id) {
            return Err(format!("{at}: target {id} is listed twice"));
        }
        if t.subject().is_none() {
            return Err(format!(
                "{at}: target {id} names a part, or a build and its version"
            ));
        }
        let strings = [&t.part, &t.build];
        if strings.iter().any(|s| s.as_deref() == Some("")) || t.serial.is_empty() {
            return Err(format!("{at}: target {id} has an empty string"));
        }
        if let Some(u) = target[..i].iter().find(|u| u.serial == t.serial) {
            return Err(format!(
                "{at}: targets {} and {id} give the same serial, which cannot tell them apart",
                u.id
            ));
        }
        if !real_day(&t.listed) || t.retired.as_deref().is_some_and(|r| !real_day(r)) {
            return Err(format!(
                "{at}: target {id}'s listed and retired are UTC days, like 2026-10-14"
            ));
        }
        if t.retired.as_deref().is_some_and(|r| r <= t.listed.as_str()) {
            return Err(format!(
                "{at}: target {id} is retired on or before the day it was listed"
            ));
        }
        if !a_person(&t.approved_by) {
            return Err(format!(
                "{at}: target {id}'s approved_by names the person who approved it, as \
                 person:<name>, the name in lowercase letters, digits and hyphens"
            ));
        }
    }
    for t in &mut target {
        t.record = at.into();
    }
    Ok(target)
}

/// Whether `s` names a person, as `person:<name>`, the name in lowercase letters, digits and
/// hyphens.
fn a_person(s: &str) -> bool {
    matches!(s.split_once(':'), Some(("person", name)) if plain(name))
}

/// Holds a flash in the lab log to the list in `cat`: its target is on the list on the flash's
/// day, the serial read is the target's, and the image is the firmware the target's build version
/// pins, if it pins one. A flash that names the person who approved it is not held to the list.
pub(crate) fn check_flash(cat: &Catalogue, e: &Entry) -> Result<(), String> {
    let at = format!("{}: entry {}", e.record, e.seq);
    let param = |key: &str| match e.params.get(key) {
        Some(Datum::Text(t)) => Some(t.as_str()),
        _ => None,
    };
    let (Some(id), Some(serial), Some(image)) =
        (param("target"), param("serial"), param("image_sha256"))
    else {
        return Err(format!(
            "{at}: a {FLASH} names, as words in its params, the target, the serial read from the \
             board and the image's sha256: target, serial and image_sha256"
        ));
    };
    if !sha256_shaped(image) {
        return Err(format!(
            "{at}: image_sha256 must be a sha256 in lowercase hex"
        ));
    }
    match e.params.get("approved_by") {
        None => {}
        Some(Datum::Text(p)) if a_person(p) => return Ok(()),
        Some(_) => {
            return Err(format!(
                "{at}: approved_by names the person who approved the flash, as person:<name>, \
                 the name in lowercase letters, digits and hyphens"
            ));
        }
    }
    let Some(t) = cat.targets.iter().find(|t| t.id == id) else {
        return Err(format!(
            "{at}: flashes {id}, which is no target in {TARGETS}"
        ));
    };
    let day = &e.at[..10];
    if day < t.listed.as_str() {
        return Err(format!(
            "{at}: flashes {id} on {day}, before it was listed, on {}",
            t.listed
        ));
    }
    if let Some(retired) = &t.retired
        && day >= retired.as_str()
    {
        return Err(format!(
            "{at}: flashes {id} on {day}, but it was retired on {retired}"
        ));
    }
    if serial != t.serial {
        return Err(format!(
            "{at}: the serial read, {serial:?}, is not {id}'s, {:?}",
            t.serial
        ));
    }
    if let Some(pinned) = t.pinned(cat)
        && image != pinned
    {
        return Err(format!(
            "{at}: {id} is {}, which pins its firmware, so the image is that firmware, {pinned}",
            t.subject().unwrap_or_default()
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log::{self, Entry};
    use std::collections::BTreeMap;

    fn src(path: &str, text: &str) -> Source {
        Source {
            path: path.into(),
            text: text.into(),
        }
    }

    /// Three boards: a bare RP2350 board; a probe listed as the build that pins its firmware,
    /// retired on 2026-10-20; and a board listed as a build that pins none.
    const LIST: &str = "\
[[target]]
id = \"pico-1\"
part = \"RP2350\"
serial = \"E661\"
listed = \"2026-10-14\"
approved_by = \"person:jon\"

[[target]]
id = \"probe-1\"
build = \"probe\"
version = 1
serial = \"E662\"
listed = \"2026-10-14\"
approved_by = \"person:jon\"
retired = \"2026-10-20\"

[[target]]
id = \"bare-1\"
build = \"bare\"
version = 1
serial = \"E663\"
listed = \"2026-10-14\"
approved_by = \"person:jon\"
";

    fn refused(text: &str, because: &str) {
        let err = parse(&src(TARGETS, text)).err().unwrap_or_default();
        assert!(err.contains(because), "{because}: {err}");
    }

    /// [`LIST`] with `from` replaced by `to`, which must be in it.
    fn edited(from: &str, to: &str) -> String {
        assert!(LIST.contains(from), "{from:?} is not in the list");
        LIST.replacen(from, to, 1)
    }

    #[test]
    fn a_lab_target_keeps_its_shape() {
        let targets = parse(&src(TARGETS, LIST)).unwrap();
        assert_eq!(targets.len(), 3);
        assert_eq!(targets[1].subject().as_deref(), Some("probe/v1"));
        assert_eq!(targets[1].record, TARGETS);
        let empty = parse(&src(TARGETS, "# Nothing is a target yet.\n")).unwrap();
        assert!(empty.is_empty(), "a list with no targets");
        let at = |from: &str, to: &str, because: &str| refused(&edited(from, to), because);
        at(
            "[[target]]",
            "extra = 1\n[[target]]",
            "unknown field `extra`",
        );
        at(
            "id = \"pico-1\"",
            "id = \"pico-1\"\nextra = 1",
            "unknown field `extra`",
        );
        at(
            "id = \"pico-1\"",
            "id = \"pico-1\"\nfirmware_sha256 = \"x\"",
            "unknown field `firmware_sha256`",
        );
        at("id = \"pico-1\"", "id = \"Pico 1\"", "target id \"Pico 1\"");
        at("id = \"pico-1\"", "id = \"\"", "target id \"\"");
        at(
            "id = \"probe-1\"",
            "id = \"pico-1\"",
            "target pico-1 is listed twice",
        );
        at("serial = \"E661\"\n", "", "missing field `serial`");
        at("listed = \"2026-10-14\"\n", "", "missing field `listed`");
        at(
            "approved_by = \"person:jon\"\n",
            "",
            "missing field `approved_by`",
        );
        let what = "target pico-1 names a part, or a build and its version";
        at("part = \"RP2350\"\n", "", what);
        at(
            "part = \"RP2350\"",
            "part = \"RP2350\"\nbuild = \"probe\"\nversion = 1",
            what,
        );
        at("part = \"RP2350\"", "part = \"RP2350\"\nversion = 1", what);
        at("part = \"RP2350\"", "build = \"probe\"", what);
        at(
            "part = \"RP2350\"",
            "part = \"\"",
            "target pico-1 has an empty string",
        );
        at(
            "serial = \"E661\"",
            "serial = \"\"",
            "target pico-1 has an empty string",
        );
        at(
            "serial = \"E662\"",
            "serial = \"E661\"",
            "targets pico-1 and probe-1 give the same serial",
        );
        let day = "target pico-1's listed and retired are UTC days";
        at("listed = \"2026-10-14\"", "listed = \"2026-02-30\"", day);
        at(
            "listed = \"2026-10-14\"",
            "listed = \"2026-10-14T00:00:00Z\"",
            day,
        );
        at(
            "approved_by = \"person:jon\"\n",
            "approved_by = \"person:jon\"\nretired = \"soon\"\n",
            day,
        );
        let early = "target probe-1 is retired on or before the day it was listed";
        at(
            "retired = \"2026-10-20\"",
            "retired = \"2026-10-14\"",
            early,
        );
        at(
            "retired = \"2026-10-20\"",
            "retired = \"2026-10-13\"",
            early,
        );
        let person = "target pico-1's approved_by names the person who approved it, as person:";
        at(
            "approved_by = \"person:jon\"",
            "approved_by = \"agent:claude\"",
            person,
        );
        at(
            "approved_by = \"person:jon\"",
            "approved_by = \"jon\"",
            person,
        );
        at(
            "approved_by = \"person:jon\"",
            "approved_by = \"person:\"",
            person,
        );
        at(
            "approved_by = \"person:jon\"",
            "approved_by = \"person:Jon\"",
            person,
        );
    }

    /// A catalogue with the RP2350, a probe build that pins its firmware, `b` x 64, a bare build
    /// that pins none, an enforcer build that is never a lab target, and a bench that uses it.
    fn catalogue() -> Catalogue {
        let base = src(
            crate::TRUSTED_BASE,
            "[[entry]]\nid = \"entry\"\nassumes = \"x\"\n",
        );
        let rp2350 = "part = \"RP2350\"\nkind = \"chip\"\nis = \"microcontroller\"\n";
        let probe = format!(
            "build = \"probe\"\nversion = 1\nstatus = \"draft\"\ndoes = \"a thing\"\n\n\
             [firmware]\nname = \"f\"\nlicense = \"MIT\"\nsource = \"s\"\nrelease = \"r\"\n\
             file = \"f\"\nsha256 = \"{}\"\n",
            "b".repeat(64)
        );
        let bare = "build = \"bare\"\nversion = 1\nstatus = \"draft\"\ndoes = \"a thing\"\n";
        let enforcer = "build = \"enforcer\"\nversion = 1\nstatus = \"draft\"\ndoes = \"a thing\"\n\
                        never_a_lab_target = \"it holds the limits\"\n";
        let bench = "build = \"bench\"\nversion = 1\nstatus = \"draft\"\ndoes = \"a thing\"\n\n\
                     [[uses]]\nbuild = \"enforcer\"\nversion = 1\n";
        Catalogue::from_sources(
            &base,
            &[src("parts/rp2350.toml", rp2350)],
            &[
                src("builds/probe/v1.toml", &probe),
                src("builds/bare/v1.toml", bare),
                src("builds/enforcer/v1.toml", enforcer),
                src("builds/bench/v1.toml", bench),
            ],
        )
        .unwrap()
    }

    #[test]
    fn a_lab_target_names_a_board_the_catalogue_holds() {
        let cat = catalogue().with_targets(src(TARGETS, LIST)).unwrap();
        assert_eq!(cat.targets.len(), 3);
        assert!(
            cat.sources.iter().any(|s| s.path == TARGETS),
            "bronze holds it"
        );
        for (from, to, because) in [
            (
                "part = \"RP2350\"",
                "part = \"RP9999\"",
                "target pico-1 is RP9999, which has no record",
            ),
            (
                "version = 1",
                "version = 2",
                "target probe-1 is probe/v2, which has no record",
            ),
            (
                "build = \"probe\"",
                "build = \"enforcer\"",
                "target probe-1 is enforcer/v1, which is or uses enforcer/v1, never a lab target: it \
                 holds the limits",
            ),
            (
                "build = \"probe\"",
                "build = \"bench\"",
                "target probe-1 is bench/v1, which is or uses enforcer/v1, never a lab target",
            ),
        ] {
            let err = catalogue()
                .with_targets(src(TARGETS, &edited(from, to)))
                .err()
                .unwrap_or_default();
            assert!(err.contains(because), "{because}: {err}");
        }
    }

    /// A lab log of one entry, at `at`, as `tentzhen log append` writes it.
    fn one_entry(at: &str, what: &str, params: &[(&str, &str)]) -> Vec<Source> {
        let e = Entry {
            seq: 1,
            at: at.into(),
            by: "agent:claude".into(),
            what: what.into(),
            params: params
                .iter()
                .map(|(k, v)| (k.to_string(), Datum::read(k, v).unwrap()))
                .collect(),
            result: BTreeMap::from([("done".into(), Datum::Flag(true))]),
            limits_sha256: "a".repeat(64),
            prev: None,
            sha256: String::new(),
            record: String::new(),
        };
        let line = serde_json::to_string(&e).unwrap();
        vec![src(
            &format!("{}/{}.jsonl", log::LOG, &at[..10]),
            &format!("{line}\n"),
        )]
    }

    #[test]
    fn a_flash_is_held_to_the_lab_target_list() {
        let pinned = "b".repeat(64);
        let other = "c".repeat(64);
        let flash = |at: &str, target: &str, serial: &str, image: &str| {
            let params = [
                ("target", target),
                ("serial", serial),
                ("image_sha256", image),
            ];
            catalogue()
                .with_targets(src(TARGETS, LIST))
                .unwrap()
                .with_log(one_entry(at, FLASH, &params))
        };
        let day = "2026-10-15T10:00:00Z";
        // A board listed as a part takes any image; one listed as a build version, the firmware
        // it pins, if it pins one, on the days it is listed.
        for (at, target, serial, image) in [
            (day, "pico-1", "E661", &other),
            (day, "probe-1", "E662", &pinned),
            (day, "bare-1", "E663", &other),
            ("2026-10-14T00:00:00Z", "probe-1", "E662", &pinned),
            ("2026-10-19T23:59:59Z", "probe-1", "E662", &pinned),
        ] {
            let cat = flash(at, target, serial, image);
            assert!(cat.is_ok(), "{at} {target}: {:?}", cat.err());
        }
        let refused = |result: Result<Catalogue, String>, because: &str| {
            let err = result.err().unwrap_or_default();
            assert!(err.contains(because), "{because}: {err}");
        };
        refused(
            flash(day, "pico-9", "E661", &other),
            "lab/log/2026-10-15.jsonl: entry 1: flashes pico-9, which is no target in \
             lab/targets.toml",
        );
        refused(
            flash("2026-10-13T23:59:59Z", "pico-1", "E661", &other),
            "flashes pico-1 on 2026-10-13, before it was listed, on 2026-10-14",
        );
        for at in ["2026-10-20T00:00:00Z", "2026-10-21T10:00:00Z"] {
            refused(
                flash(at, "probe-1", "E662", &pinned),
                "but it was retired on 2026-10-20",
            );
        }
        refused(
            flash(day, "pico-1", "E662", &other),
            "the serial read, \"E662\", is not pico-1's, \"E661\"",
        );
        refused(
            flash(day, "probe-1", "E662", &other),
            &format!(
                "probe-1 is probe/v1, which pins its firmware, so the image is that firmware, \
                 {pinned}"
            ),
        );
        refused(
            flash(day, "pico-1", "E661", "bbb"),
            "image_sha256 must be a sha256 in lowercase hex",
        );
        let named = "a firmware.flash names, as words in its params, the target, the serial";
        let listed = || catalogue().with_targets(src(TARGETS, LIST)).unwrap();
        let all = [
            ("target", "pico-1"),
            ("serial", "E661"),
            ("image_sha256", other.as_str()),
        ];
        for gone in 0..all.len() {
            let mut params = all.to_vec();
            params.remove(gone);
            refused(listed().with_log(one_entry(day, FLASH, &params)), named);
        }
        let flag = [
            ("target", "pico-1"),
            ("serial", "true"),
            ("image_sha256", other.as_str()),
        ];
        refused(listed().with_log(one_entry(day, FLASH, &flag)), named);
        // With no list, as before any is loaded, nothing may be flashed.
        refused(
            catalogue().with_log(one_entry(day, FLASH, &all)),
            "flashes pico-1, which is no target",
        );
        // A flash a person approved is not held to the list, and needs no list at all. It still
        // names what was flashed.
        let approved = |by: &'static str| {
            let mut params = all.to_vec();
            params[0] = ("target", "enforcer-1");
            params.push(("approved_by", by));
            params
        };
        for cat in [listed(), catalogue()] {
            let ok = cat.with_log(one_entry(day, FLASH, &approved("person:jon")));
            assert!(ok.is_ok(), "{:?}", ok.err());
        }
        let by = "approved_by names the person who approved the flash, as person:<name>";
        for who in ["agent:claude", "jon", "person:", "true"] {
            refused(listed().with_log(one_entry(day, FLASH, &approved(who))), by);
        }
        let mut unnamed = approved("person:jon");
        unnamed.retain(|(key, _)| *key != "serial");
        refused(listed().with_log(one_entry(day, FLASH, &unnamed)), named);
        let mut bad = approved("person:jon");
        bad[2] = ("image_sha256", "bbb");
        refused(
            listed().with_log(one_entry(day, FLASH, &bad)),
            "image_sha256 must be a sha256",
        );
        // Only a flash is held to the list.
        let set = catalogue().with_log(one_entry(day, "supply.set", &[("set_volts", "3.3")]));
        assert!(set.is_ok(), "{:?}", set.err());
    }
}
