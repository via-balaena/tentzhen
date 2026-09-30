//! The lab log: every hardware action, as CLAUDE.md asks ("Log every hardware action (what, when,
//! parameters, result) to the lab's append-only log"). One JSON object per line, in
//! `lab/log/<yyyy-mm-dd>.jsonl` by the UTC day of the action, written by [`append`]
//! (`cargo run -p tentzhen -- log append`). A line must be exactly as [`append`] writes its entry,
//! which gives each key once.
//!
//! Each entry names the sha256 of the line before it, so an entry edited, dropped or moved breaks
//! the chain at the entry after it, and [`parse`] refuses the log. The chain cannot see a change to
//! the last entry, entries cut from the end, or a chain rewritten from an edit on: the Quality
//! Gate's step "The lab log only grows" refuses a change to main's log that is not an append.
//!
//! Every field is what its writer says, `by` included. The log is signed only as a commit is, and
//! an agent that can commit on the writer's machine signs with the same key as the person.

use crate::{Source, VALUE_UNITS, plain, read_folder, sha256_shaped, snake, unit_of};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Where the log lives, relative to the repo root.
pub const LOG: &str = "lab/log";

/// One hardware action.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    /// Its place in the log, from 1, across every day's file.
    pub seq: u64,
    /// When, in UTC, to the second: `2026-10-14T18:03:22Z`. Its day names the file it is in.
    pub at: String,
    /// Who acted: `person:<name>` or `agent:<name>`.
    pub by: String,
    /// The action, as words joined by dots: `supply.set`.
    pub what: String,
    /// What the action was asked to do. Written even when empty, as `{}`.
    pub params: BTreeMap<String, Datum>,
    /// What happened. Never empty.
    pub result: BTreeMap<String, Datum>,
    /// The sha256 of `lab/limits.toml` as the host read it when the entry was written.
    pub limits_sha256: String,
    /// The sha256 of the line before it. Only the first entry has none.
    pub prev: Option<String>,
    /// The sha256 of its own line, which the next entry names as its `prev`. Set on load.
    #[serde(skip)]
    pub sha256: String,
    /// The file it came from, relative to the repo root. Set on load, never written in a record.
    #[serde(skip)]
    pub record: String,
}

impl Entry {
    /// Who acted, as `person` or `agent`, and their name.
    pub fn actor(&self) -> (&str, &str) {
        self.by.split_once(':').unwrap_or(("", &self.by))
    }
}

/// A parameter or a result, here or in a measurement record. A number's key ends in its unit ([`VALUE_UNITS`]); a key that does
/// not holds a word or a yes/no.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Datum {
    Number(f64),
    Flag(bool),
    Text(String),
}

impl Datum {
    /// Reads a value typed as text, as its key says: a number if the key ends in a unit, else
    /// `true` or `false`, else the text itself.
    pub fn read(key: &str, value: &str) -> Result<Self, String> {
        if unit_of(key).is_some() {
            return value
                .parse()
                .map(Datum::Number)
                .map_err(|_| format!("{key} ends in its unit, so it is a number, not {value:?}"));
        }
        Ok(match value {
            "true" => Datum::Flag(true),
            "false" => Datum::Flag(false),
            _ => Datum::Text(value.into()),
        })
    }
}

/// The log's files under `root`, oldest day first: none if it has no log yet.
pub fn read(root: &Path) -> Result<Vec<Source>, String> {
    read_folder(root, LOG)
}

/// Parses the log's files, oldest day first whatever order they come in, and holds every entry to
/// its shape and to the chain through them.
pub fn parse(files: &[Source]) -> Result<Vec<Entry>, String> {
    let mut files: Vec<&Source> = files.iter().collect();
    files.sort_by(|a, b| a.path.cmp(&b.path));
    let mut entries: Vec<Entry> = Vec::new();
    for f in files {
        let day = day_of_file(&f.path).ok_or_else(|| {
            format!(
                "{}: the log's files are named {LOG}/<yyyy-mm-dd>.jsonl, by the UTC day",
                f.path
            )
        })?;
        if f.text.is_empty() || !f.text.ends_with('\n') || f.text.contains('\r') {
            return Err(format!(
                "{}: a day's file is one entry per line, each ending in a newline (\\n)",
                f.path
            ));
        }
        for (n, line) in (1..).zip(f.text.split_terminator('\n')) {
            let tail = entries.last().map(|e| (e.seq, e.sha256.as_str()));
            let mut e = read_line(&format!("{}:{n}", f.path), day, line, tail)?;
            e.record = f.path.clone();
            entries.push(e);
        }
    }
    Ok(entries)
}

/// Writes an entry for an action taken `now` to the log under `root`, chained to the last entry,
/// and returns it. Refuses if the log as it stands would not load, since an entry chained to a
/// broken log shows nothing, and refuses an entry the log would refuse. Holds a lock on the log's
/// folder while it reads and writes, so two writers on one machine cannot both chain to the same
/// entry.
pub fn append(
    root: &Path,
    now: SystemTime,
    by: &str,
    what: &str,
    params: BTreeMap<String, Datum>,
    result: BTreeMap<String, Datum>,
) -> Result<Entry, String> {
    let dir = root.join(LOG);
    fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let lock = File::open(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    lock.lock()
        .map_err(|e| format!("{}: cannot lock: {e}", dir.display()))?;
    let log = parse(&read(root)?)?;
    let at = utc(now)?;
    let day = &at[..10];
    if let Some(last) = log.last()
        && day < &last.at[..10]
    {
        return Err(format!(
            "the clock says {at}, before the log's last entry, {}",
            last.at
        ));
    }
    let limits = root.join(tentzhen_lab::LIMITS);
    let limits = fs::read(&limits).map_err(|e| format!("{}: {e}", limits.display()))?;
    let tail = log.last().map(|e| (e.seq, e.sha256.as_str()));
    let record = format!("{LOG}/{day}.jsonl");
    let entry = Entry {
        seq: tail.map_or(1, |(seq, _)| seq + 1),
        at: at.clone(),
        by: by.into(),
        what: what.into(),
        params,
        result,
        limits_sha256: sha256_hex(&limits),
        prev: tail.map(|(_, sha)| sha.to_string()),
        sha256: String::new(),
        record: String::new(),
    };
    // Checked before it is written as JSON too, which would write a NaN as null.
    check_entry(&record, day, &entry)?;
    let line = serde_json::to_string(&entry).map_err(|e| e.to_string())?;
    let mut written = read_line(&record, day, &line, tail)?;
    let path = root.join(&record);
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .and_then(|mut f| f.write_all(format!("{line}\n").as_bytes()))
        .map_err(|e| format!("{}: {e}", path.display()))?;
    written.record = record;
    Ok(written)
}

/// `t` in UTC, to the second, as an entry's `at`.
pub fn utc(t: SystemTime) -> Result<String, String> {
    let secs = t
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "the clock says before 1970".to_string())?
        .as_secs();
    let (y, m, d) = civil(secs / 86_400);
    let s = secs % 86_400;
    Ok(format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        s / 3_600,
        s / 60 % 60,
        s % 60
    ))
}

/// The date `days` after 1970-01-01 in the Gregorian calendar, by Howard Hinnant's
/// `civil_from_days`.
fn civil(days: u64) -> (u64, u64, u64) {
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z % 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + u64::from(m <= 2), m, d)
}

/// Whether `at` is a real UTC time to the second, written `2026-10-14T18:03:22Z`.
fn real_time(at: &str) -> bool {
    const SHAPE: &[u8] = b"0000-00-00T00:00:00Z";
    let shaped = at.len() == SHAPE.len()
        && at.bytes().zip(SHAPE).all(|(c, s)| match s {
            b'0' => c.is_ascii_digit(),
            _ => c == *s,
        });
    let n = |from: usize, to: usize| at.get(from..to).and_then(|t| t.parse::<u32>().ok());
    let (Some(y), Some(mo), Some(d), Some(h), Some(mi), Some(s)) =
        (n(0, 4), n(5, 7), n(8, 10), n(11, 13), n(14, 16), n(17, 19))
    else {
        return false;
    };
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let days = match mo {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    shaped && (1..=12).contains(&mo) && (1..=days).contains(&d) && h < 24 && mi < 60 && s < 60
}

/// Whether `day` is a real day, written `2026-10-14`.
pub(crate) fn real_day(day: &str) -> bool {
    real_time(&format!("{day}T00:00:00Z"))
}

/// The day a log file is for, from its name: `lab/log/2026-10-14.jsonl` is for `2026-10-14`.
fn day_of_file(path: &str) -> Option<&str> {
    let day = path
        .strip_prefix(LOG)?
        .strip_prefix('/')?
        .strip_suffix(".jsonl")?;
    real_day(day).then_some(day)
}

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Parses one line of a day's file, holds it to an entry's shape, and chains it to `tail`: the
/// entry before it, as its number and the sha256 of its line.
fn read_line(
    place: &str,
    day: &str,
    line: &str,
    tail: Option<(u64, &str)>,
) -> Result<Entry, String> {
    if line.is_empty() {
        return Err(format!("{place}: a blank line"));
    }
    let mut e: Entry = serde_json::from_str(line).map_err(|err| format!("{place}: {err}"))?;
    check_entry(place, day, &e)?;
    // JSON lets a key appear twice, and parsing keeps one of them. A line exactly as the writer
    // writes its entry has each key once.
    if serde_json::to_string(&e).ok().as_deref() != Some(line) {
        return Err(format!(
            "{place}: the line is not as `tentzhen log append` writes it: a key given twice, or \
             spacing or order changed by hand"
        ));
    }
    match (tail, &e.prev) {
        (None, None) if e.seq == 1 => {}
        (None, _) => {
            return Err(format!(
                "{place}: the first entry is numbered 1 and names no prev"
            ));
        }
        (Some(_), None) => {
            return Err(format!(
                "{place}: entry {} names no prev, which only the first entry may do",
                e.seq
            ));
        }
        (Some((seq, sha)), Some(prev)) => {
            if prev != sha {
                return Err(format!(
                    "{place}: entry {}'s prev is not the sha256 of the line before it: an entry \
                     was edited, dropped or moved",
                    e.seq
                ));
            }
            if e.seq != seq + 1 {
                return Err(format!("{place}: entry {} follows entry {seq}", e.seq));
            }
        }
    }
    e.sha256 = sha256_hex(line.as_bytes());
    Ok(e)
}

/// Holds an entry to its shape, for the file of `day`.
fn check_entry(place: &str, day: &str, e: &Entry) -> Result<(), String> {
    if !real_time(&e.at) {
        return Err(format!(
            "{place}: at {:?} is not a UTC time to the second, like 2026-10-14T18:03:22Z",
            e.at
        ));
    }
    if &e.at[..10] != day {
        return Err(format!(
            "{place}: an entry at {} belongs in {LOG}/{}.jsonl",
            e.at,
            &e.at[..10]
        ));
    }
    match e.by.split_once(':') {
        Some(("person" | "agent", name)) if plain(name) => {}
        _ => {
            return Err(format!(
                "{place}: by {:?} must be person:<name> or agent:<name>, the name in lowercase \
                 letters, digits and hyphens",
                e.by
            ));
        }
    }
    if !e.what.split('.').all(plain) {
        return Err(format!(
            "{place}: what {:?} must be words of lowercase letters, digits and hyphens, joined by dots",
            e.what
        ));
    }
    check_data(place, "params", &e.params)?;
    check_data(place, "result", &e.result)?;
    if e.result.is_empty() {
        return Err(format!("{place}: the result is empty; say what happened"));
    }
    if !sha256_shaped(&e.limits_sha256) {
        return Err(format!(
            "{place}: limits_sha256 must be a sha256 in lowercase hex"
        ));
    }
    Ok(())
}

/// Holds params or results to their keys: a number's key ends in its unit, and only a number's.
pub(crate) fn check_data(
    place: &str,
    side: &str,
    data: &BTreeMap<String, Datum>,
) -> Result<(), String> {
    for (key, d) in data {
        if !snake(key) {
            return Err(format!(
                "{place}: {side} key {key:?} must be lowercase words joined by underscores"
            ));
        }
        match (d, unit_of(key)) {
            (Datum::Number(n), Some(_)) if !n.is_finite() => {
                return Err(format!("{place}: {side} {key} is {n}"));
            }
            (Datum::Number(_), None) => {
                return Err(format!(
                    "{place}: {side} {key} is a number, so its key must end in its unit, one of {}",
                    VALUE_UNITS.join(", ")
                ));
            }
            (Datum::Flag(_) | Datum::Text(_), Some(unit)) => {
                return Err(format!(
                    "{place}: {side} {key} ends in {unit}, so it must be a number"
                ));
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::Duration;

    const DAY: &str = "2026-10-14";

    fn entry(seq: u64, at: &str, prev: Option<&str>) -> Entry {
        Entry {
            seq,
            at: at.into(),
            by: "person:jon".into(),
            what: "supply.set".into(),
            params: BTreeMap::from([("set_volts".into(), Datum::Number(3.3))]),
            result: BTreeMap::from([("done".into(), Datum::Flag(true))]),
            limits_sha256: "a".repeat(64),
            prev: prev.map(Into::into),
            sha256: String::new(),
            record: String::new(),
        }
    }

    fn json(e: &Entry) -> String {
        serde_json::to_string(e).unwrap()
    }

    /// A log over `days`, each a day and how many entries it has, chained from the first.
    fn log(days: &[(&str, u64)]) -> Vec<Source> {
        let mut prev: Option<String> = None;
        let mut seq = 0;
        let mut files = Vec::new();
        for (day, n) in days {
            let mut text = String::new();
            for i in 0..*n {
                seq += 1;
                let line = json(&entry(
                    seq,
                    &format!("{day}T10:00:{i:02}Z"),
                    prev.as_deref(),
                ));
                prev = Some(sha256_hex(line.as_bytes()));
                text.push_str(&line);
                text.push('\n');
            }
            files.push(Source {
                path: format!("{LOG}/{day}.jsonl"),
                text,
            });
        }
        files
    }

    /// One day's file of these lines.
    fn day(lines: &[String]) -> Vec<Source> {
        vec![Source {
            path: format!("{LOG}/{DAY}.jsonl"),
            text: lines.iter().map(|l| format!("{l}\n")).collect(),
        }]
    }

    fn refused(files: &[Source], because: &str) {
        let err = parse(files).err().unwrap_or_default();
        assert!(err.contains(because), "{because}: {err}");
    }

    #[test]
    fn a_log_chains_each_entry_to_the_line_before() {
        let files = log(&[(DAY, 2), ("2026-10-15", 2)]);
        let entries = parse(&files).unwrap();
        let seqs: Vec<u64> = entries.iter().map(|e| e.seq).collect();
        assert_eq!(seqs, [1, 2, 3, 4]);
        assert_eq!(entries[0].prev, None);
        for pair in entries.windows(2) {
            assert_eq!(pair[1].prev.as_deref(), Some(pair[0].sha256.as_str()));
        }
        assert_eq!(entries[2].record, "lab/log/2026-10-15.jsonl");
        assert_eq!(entries[0].actor(), ("person", "jon"));
        let mut reversed = files.clone();
        reversed.reverse();
        assert_eq!(parse(&reversed).unwrap().len(), 4, "in any order");
    }

    #[test]
    fn an_edited_dropped_or_moved_entry_breaks_the_chain() {
        let ok: Vec<String> = log(&[(DAY, 4)])[0].text.lines().map(String::from).collect();
        let broken = "entry 3's prev is not the sha256 of the line before it";
        let mut edited = ok.clone();
        edited[1] = edited[1].replace("3.3", "3.2");
        assert_ne!(edited, ok);
        refused(&day(&edited), broken);
        let mut dropped = ok.clone();
        dropped.remove(1);
        refused(&day(&dropped), broken);
        let mut moved = ok.clone();
        moved.swap(1, 2);
        refused(&day(&moved), broken);
        // What the chain cannot see, so the Quality Gate's step "The lab log only grows" must: the
        // last entry changed, entries cut from the end, and a chain rewritten from an edit on.
        let mut last = ok.clone();
        last[3] = last[3].replace("3.3", "3.2");
        assert!(parse(&day(&last)).is_ok());
        assert!(parse(&day(&ok[..2])).is_ok());
        let mut rewritten = vec![ok[0].clone()];
        for line in &ok[1..] {
            let mut e: Entry = serde_json::from_str(&line.replace("3.3", "3.2")).unwrap();
            e.prev = rewritten.last().map(|l| sha256_hex(l.as_bytes()));
            rewritten.push(json(&e));
        }
        assert_ne!(rewritten, ok);
        assert!(parse(&day(&rewritten)).is_ok());
    }

    #[test]
    fn a_log_entry_keeps_its_shape() {
        let at = format!("{DAY}T10:00:00Z");
        let first = |edit: &dyn Fn(&mut Entry)| {
            let mut e = entry(1, &at, None);
            edit(&mut e);
            day(&[json(&e)])
        };
        assert!(parse(&first(&|_| {})).is_ok(), "the control is accepted");
        let unknown = json(&entry(1, &at, None)).replacen('{', "{\"grade\":\"measured\",", 1);
        refused(&day(&[unknown]), "unknown field `grade`");
        let no_params = json(&entry(1, &at, None)).replace("\"params\":{\"set_volts\":3.3},", "");
        refused(&day(&[no_params]), "missing field `params`");
        let written = json(&entry(1, &at, None));
        for by_hand in [
            written.replace("\"set_volts\":3.3", "\"set_volts\":3.3,\"set_volts\":9.9"),
            written.replace(",\"by\"", ", \"by\""),
            written.replace("3.3", "3.30"),
        ] {
            assert_ne!(by_hand, written);
            refused(&day(&[by_hand]), "not as `tentzhen log append` writes it");
        }
        for bad in [
            "2026-10-14 10:00:00Z",
            "2026-10-14T10:00:00+02:00",
            "2026-10-14T10:00:00",
            "2026-10-32T10:00:00Z",
            "2026-10-14T24:00:00Z",
        ] {
            refused(&first(&|e| e.at = bad.into()), "is not a UTC time");
        }
        refused(
            &first(&|e| e.at = "2026-10-15T10:00:00Z".into()),
            "belongs in lab/log/2026-10-15.jsonl",
        );
        for bad in ["jon", "robot:jon", "person:Jon", "person:"] {
            refused(&first(&|e| e.by = bad.into()), "must be person:<name>");
        }
        for bad in ["Supply.set", "supply..set", ""] {
            refused(&first(&|e| e.what = bad.into()), "joined by dots");
        }
        let one = |key: &str, d: Datum| BTreeMap::from([(key.to_string(), d)]);
        refused(
            &first(&|e| e.params = one("volts", Datum::Number(3.3))),
            "params volts is a number, so its key must end in its unit",
        );
        refused(
            &first(&|e| e.result = one("read_volts", Datum::Text("high".into()))),
            "result read_volts ends in volts, so it must be a number",
        );
        refused(
            &first(&|e| e.params = one("Set", Datum::Flag(true))),
            "params key \"Set\" must be lowercase words",
        );
        refused(&first(&|e| e.result.clear()), "the result is empty");
        refused(
            &first(&|e| e.limits_sha256 = "abc".into()),
            "limits_sha256 must be a sha256",
        );
        refused(
            &first(&|e| e.prev = Some("a".repeat(64))),
            "the first entry is numbered 1 and names no prev",
        );
        refused(
            &first(&|e| e.seq = 2),
            "the first entry is numbered 1 and names no prev",
        );
        let head = json(&entry(1, &at, None));
        let sha = sha256_hex(head.as_bytes());
        refused(
            &day(&[head.clone(), json(&entry(2, &at, None))]),
            "entry 2 names no prev",
        );
        refused(
            &day(&[head.clone(), json(&entry(3, &at, Some(&sha)))]),
            "entry 3 follows entry 1",
        );
        refused(&day(&[head.clone(), String::new()]), "a blank line");
        let file = |path: &str, text: &str| {
            vec![Source {
                path: path.into(),
                text: text.into(),
            }]
        };
        let text = format!("{head}\n");
        for bad in [
            "lab/log/2026-10-14.json",
            "lab/log/14-10-2026.jsonl",
            "lab/log/2026-02-29.jsonl",
            "lab/2026-10-14.jsonl",
        ] {
            refused(&file(bad, &text), "the log's files are named");
        }
        let path = format!("{LOG}/{DAY}.jsonl");
        for bad in [String::new(), head.clone(), format!("{head}\r\n")] {
            refused(&file(&path, &bad), "each ending in a newline");
        }
    }

    /// A repo root of its own under the system's temp folder, with a limits file.
    fn scratch(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("tentzhen-log-{}-{name}", std::process::id()));
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        fs::create_dir_all(root.join("lab")).unwrap();
        fs::write(root.join(tentzhen_lab::LIMITS), "limits").unwrap();
        root
    }

    fn at(secs: u64) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(secs)
    }

    #[test]
    fn appending_extends_the_chain() {
        let root = scratch("extends");
        let set = |secs: u64, volts: f64| {
            append(
                &root,
                at(secs),
                "person:jon",
                "supply.set",
                BTreeMap::from([("set_volts".into(), Datum::Number(volts))]),
                BTreeMap::from([("done".into(), Datum::Flag(true))]),
            )
        };
        let a = set(1_790_000_000, 3.3).unwrap();
        let b = set(1_790_000_001, 3.2).unwrap();
        let c = set(1_790_100_000, 3.1).unwrap();
        assert_eq!((a.seq, b.seq, c.seq), (1, 2, 3));
        assert_eq!(a.at, "2026-09-21T14:13:20Z");
        assert_eq!(c.record, "lab/log/2026-09-22.jsonl");
        assert_eq!(c.prev.as_deref(), Some(b.sha256.as_str()));
        let entries = parse(&read(&root).unwrap()).unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[2].sha256, c.sha256);
        assert!(
            entries
                .iter()
                .all(|e| e.limits_sha256 == sha256_hex(b"limits"))
        );

        let err = set(1_790_000_002, 3.0).err().unwrap_or_default();
        assert!(err.contains("before the log's last entry"), "{err}");
        let err = set(1_790_100_001, f64::NAN).err().unwrap_or_default();
        assert!(err.contains("params set_volts is NaN"), "{err}");
        assert_eq!(parse(&read(&root).unwrap()).unwrap().len(), 3);

        let first = root.join("lab/log/2026-09-21.jsonl");
        let text = fs::read_to_string(&first).unwrap();
        fs::write(&first, text.replacen("3.3", "3.4", 1)).unwrap();
        let err = set(1_790_100_001, 3.0).err().unwrap_or_default();
        assert!(
            err.contains("prev is not the sha256"),
            "a broken log: {err}"
        );
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn appends_from_many_threads_keep_one_chain() {
        let root = scratch("threads");
        std::thread::scope(|s| {
            for _ in 0..8 {
                s.spawn(|| {
                    for _ in 0..5 {
                        append(
                            &root,
                            at(1_790_000_000),
                            "agent:test",
                            "supply.read",
                            BTreeMap::new(),
                            BTreeMap::from([("read_volts".into(), Datum::Number(3.3))]),
                        )
                        .unwrap();
                    }
                });
            }
        });
        assert_eq!(parse(&read(&root).unwrap()).unwrap().len(), 40);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn utc_times_from_the_clock() {
        // Each as `date -u -r <secs>` gives it.
        for (secs, want) in [
            (0, "1970-01-01T00:00:00Z"),
            (1_790_000_000, "2026-09-21T14:13:20Z"),
            (1_835_395_199, "2028-02-28T23:59:59Z"),
            (1_835_395_200, "2028-02-29T00:00:00Z"),
            (4_107_542_399, "2100-02-28T23:59:59Z"),
            (4_107_542_400, "2100-03-01T00:00:00Z"),
        ] {
            assert_eq!(utc(at(secs)).unwrap(), want);
            assert!(real_time(want), "{want}");
        }
        assert!(!real_time("2100-02-29T00:00:00Z"), "2100 is no leap year");
        assert!(real_time("2000-02-29T00:00:00Z"), "2000 is");
        assert!(utc(UNIX_EPOCH - Duration::from_secs(1)).is_err());
    }

    #[test]
    fn a_value_is_read_as_its_key_says() {
        assert_eq!(Datum::read("set_volts", "3.3"), Ok(Datum::Number(3.3)));
        assert!(Datum::read("set_volts", "high").is_err());
        assert_eq!(Datum::read("enabled", "true"), Ok(Datum::Flag(true)));
        assert_eq!(Datum::read("channel", "3.3"), Ok(Datum::Text("3.3".into())));
    }
}
