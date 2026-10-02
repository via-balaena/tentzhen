//! Loads the catalogue's records, the lab's limits, the lab-target list, the lab log, the
//! measurement records and the lab's sourcing into DuckDB: bronze (the files as read), silver
//! (typed rows the database itself constrains) and gold (views). The schema is `schema.sql`; this
//! only fills it.

use duckdb::{Connection, params};
use tentzhen_lab::{LIMITS, Limits, Value};
use tentzhen_records::log::{Datum, Entry};
use tentzhen_records::measurement::Measurement;
use tentzhen_records::sourcing::Stage;
use tentzhen_records::{Catalogue, Claim};

pub mod catalogue;

pub const SCHEMA: &str = include_str!("../schema.sql");

/// Creates the schema on an empty database and loads every record in one transaction.
pub fn load(conn: &mut Connection, cat: &Catalogue) -> duckdb::Result<()> {
    conn.execute_batch(SCHEMA)?;
    let tx = conn.transaction()?;

    for s in &cat.sources {
        tx.execute(
            "INSERT INTO bronze.record VALUES (?, ?)",
            params![s.path, s.text],
        )?;
    }
    // The trusted base first: claims point at its entries.
    for (i, e) in (1u32..).zip(&cat.trusted_base) {
        tx.execute(
            "INSERT INTO silver.trusted_entry VALUES (?, ?, ?, ?)",
            params![e.id, i, e.assumes, e.record],
        )?;
    }
    for p in cat.parts.values() {
        tx.execute(
            "INSERT INTO silver.part VALUES (?, ?, ?, ?, ?, ?)",
            params![
                p.part,
                p.kind.as_str(),
                p.is,
                p.authorized_only,
                p.datasheet,
                p.record
            ],
        )?;
    }
    // Each claim under its citation; their referents go in last, once what they name is loaded.
    let mut claims: Vec<(String, &Claim)> = Vec::new();
    for p in cat.parts.values() {
        for (i, c) in (1u32..).zip(&p.claim) {
            let claim = format!("{}#{}", p.part, c.id);
            tx.execute(
                "INSERT INTO silver.claim VALUES (?, ?, NULL, NULL, ?, ?, ?)",
                params![claim, p.part, i, c.id, c.says],
            )?;
            load_values(&tx, &claim, c)?;
            claims.push((claim, c));
        }
    }
    // Every version first: lines, uses and claims point at them.
    for b in cat.builds.values().flatten() {
        tx.execute(
            "INSERT INTO silver.build_version VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                b.build,
                b.version,
                b.status.as_str(),
                b.does,
                b.changes,
                b.drawing,
                b.never_a_lab_target,
                b.record
            ],
        )?;
    }
    for b in cat.builds.values().flatten() {
        let key = (b.build.as_str(), b.version);
        for (i, l) in (1u32..).zip(&b.line) {
            tx.execute(
                "INSERT INTO silver.line VALUES (?, ?, ?, ?, ?, ?, ?)",
                params![key.0, key.1, i, l.part, l.commodity, l.form, l.qty],
            )?;
        }
        for (i, u) in (1u32..).zip(&b.uses) {
            tx.execute(
                "INSERT INTO silver.uses VALUES (?, ?, ?, ?, ?, ?)",
                params![key.0, key.1, i, u.build, u.version, u.qty],
            )?;
        }
        if let Some(f) = &b.firmware {
            tx.execute(
                "INSERT INTO silver.firmware VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    key.0, key.1, f.name, f.license, f.source, f.release, f.file, f.sha256,
                    f.pin_map
                ],
            )?;
        }
        for (i, p) in (1u32..).zip(&b.pin) {
            tx.execute(
                "INSERT INTO silver.pin VALUES (?, ?, ?, ?, ?, ?, ?)",
                params![key.0, key.1, i, p.name, p.board_pin, p.net, p.required],
            )?;
        }
        for (i, s) in (1u32..).zip(&b.step) {
            tx.execute(
                "INSERT INTO silver.step VALUES (?, ?, ?, ?, ?, ?, ?)",
                params![key.0, key.1, i, s.r#do, s.run, s.expect, s.agent],
            )?;
        }
        for (i, c) in (1u32..).zip(&b.claim) {
            let claim = format!("{}/v{}#{}", key.0, key.1, c.id);
            tx.execute(
                "INSERT INTO silver.claim VALUES (?, NULL, ?, ?, ?, ?, ?)",
                params![claim, key.0, key.1, i, c.id, c.says],
            )?;
            load_values(&tx, &claim, c)?;
            claims.push((claim, c));
        }
    }
    if let Some(limits) = &cat.limits {
        load_limits(&tx, limits)?;
    }
    for t in &cat.targets {
        tx.execute(
            "INSERT INTO silver.lab_target VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                t.id,
                t.part,
                t.build,
                t.version,
                t.serial,
                t.listed,
                t.approved_by,
                t.retired,
                t.record
            ],
        )?;
    }
    load_log(&tx, &cat.log)?;
    for m in cat.measurements.values() {
        load_measurement(&tx, m)?;
    }
    // After the measurement records its incoming QA names.
    for (i, l) in (1u32..).zip(&cat.sourcing) {
        tx.execute(
            "INSERT INTO silver.sourcing VALUES \
             (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                i,
                l.part,
                l.form,
                l.commodity,
                l.shop.map(|s| s.as_str()),
                l.store,
                l.item,
                l.used,
                l.salvaged_from,
                l.authorized,
                l.listing_checked.is_some(),
                l.in_cart.is_some(),
                l.ordered.is_some(),
                l.arrived.is_some(),
                l.listing_checked.as_ref().and_then(Stage::day),
                l.in_cart.as_ref().and_then(Stage::day),
                l.ordered.as_ref().and_then(Stage::day),
                l.arrived.as_ref().and_then(Stage::day),
                l.passed_qa,
                l.record
            ],
        )?;
    }
    for (claim, c) in &claims {
        load_referents(&tx, claim, c)?;
    }
    tx.commit()
}

/// A claim's referents, under its citation. A trusted referent is a key into the trusted base and a
/// record referent a key into the measurement records; the others are kept as named.
fn load_referents(tx: &duckdb::Transaction, claim: &str, c: &Claim) -> duckdb::Result<()> {
    for (kind, named) in c.referents() {
        let (evidence, trusted, measurement) = match kind {
            "trusted" => (None, Some(named), None),
            "record" => (None, None, Some(named)),
            _ => (Some(named), None, None),
        };
        tx.execute(
            "INSERT INTO silver.referent VALUES (?, ?, ?, ?, ?)",
            params![claim, kind, evidence, trusted, measurement],
        )?;
    }
    Ok(())
}

/// The numbers a claim states, under its citation.
fn load_values(tx: &duckdb::Transaction, claim: &str, c: &Claim) -> duckdb::Result<()> {
    for (name, amount) in &c.values {
        tx.execute(
            "INSERT INTO silver.claim_value VALUES (?, ?, ?)",
            params![claim, name, amount],
        )?;
    }
    Ok(())
}

fn load_limits(tx: &duckdb::Transaction, limits: &Limits) -> duckdb::Result<()> {
    let entries = limits.entries();
    for e in &entries {
        let (amount, unit, choice) = match e.value {
            Value::Volts(v) => (Some(v), Some("V"), None),
            Value::Amps(a) => (Some(a), Some("A"), None),
            Value::Choice(c) => (None, None, Some(c)),
        };
        tx.execute(
            "INSERT INTO silver.lab_limit VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                e.path,
                amount,
                unit,
                choice,
                e.basis.kind(),
                e.basis.policy,
                e.basis.rule.map(|r| r.as_str()),
                LIMITS
            ],
        )?;
    }
    // Every limit first: inputs point at them.
    for e in &entries {
        for input in e.basis.inputs() {
            tx.execute(
                "INSERT INTO silver.lab_limit_input VALUES (?, ?)",
                params![e.path, input],
            )?;
        }
        for c in &e.basis.rests_on {
            tx.execute(
                "INSERT INTO silver.lab_limit_rests_on VALUES (?, ?)",
                params![e.path, c],
            )?;
        }
        if let Some((claim, name)) = e.basis.bound() {
            tx.execute(
                "INSERT INTO silver.lab_limit_at_most VALUES (?, ?, ?)",
                params![e.path, claim, name],
            )?;
        }
    }
    Ok(())
}

/// The lab log in its order, so each entry's prev names one already loaded.
fn load_log(tx: &duckdb::Transaction, log: &[Entry]) -> duckdb::Result<()> {
    for e in log {
        let (kind, name) = e.actor();
        tx.execute(
            "INSERT INTO silver.lab_log_entry VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                e.seq,
                e.at,
                kind,
                name,
                e.what,
                e.limits_sha256,
                e.sha256,
                e.prev,
                e.record
            ],
        )?;
        for (side, data) in [("params", &e.params), ("result", &e.result)] {
            for (key, d) in data {
                let (amount, text, flag) = columns(d);
                tx.execute(
                    "INSERT INTO silver.lab_log_value VALUES (?, ?, ?, ?, ?, ?)",
                    params![e.seq, side, key, amount, text, flag],
                )?;
            }
        }
    }
    Ok(())
}

/// A value as its three columns, of which it fills one: a number, a word or a yes/no.
fn columns(d: &Datum) -> (Option<f64>, Option<&str>, Option<bool>) {
    match d {
        Datum::Number(n) => (Some(*n), None, None),
        Datum::Text(t) => (None, Some(t.as_str()), None),
        Datum::Flag(f) => (None, None, Some(*f)),
    }
}

/// A measurement record, after the log entries, parts and builds it names.
fn load_measurement(tx: &duckdb::Transaction, m: &Measurement) -> duckdb::Result<()> {
    tx.execute(
        "INSERT INTO silver.measurement VALUES (?, ?, ?, ?, ?, ?)",
        params![m.id, m.measures, m.setup, m.log.first, m.log.last, m.record],
    )?;
    for (role, devices) in [("device", &m.device), ("meter", &m.meter)] {
        for (i, d) in (1u32..).zip(devices) {
            tx.execute(
                "INSERT INTO silver.measurement_device VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    m.id,
                    role,
                    i,
                    d.part,
                    d.build,
                    d.version,
                    d.revision,
                    d.firmware_sha256,
                    d.gateware_sha256,
                    d.accuracy
                ],
            )?;
        }
    }
    for (tool, version) in &m.toolchain {
        tx.execute(
            "INSERT INTO silver.measurement_tool VALUES (?, ?, ?)",
            params![m.id, tool, version],
        )?;
    }
    for (name, d) in &m.results {
        let (amount, text, flag) = columns(d);
        tx.execute(
            "INSERT INTO silver.measurement_value VALUES (?, ?, ?, ?, ?)",
            params![m.id, name, amount, text, flag],
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::Path;
    use std::time::{Duration, UNIX_EPOCH};
    use tentzhen_records::{Source, TRUSTED_BASE, log};

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

    fn build(name: &str, extra: &str) -> Source {
        src(
            &format!("builds/{name}/v1.toml"),
            &format!(
                "build = \"{name}\"\nversion = 1\nstatus = \"draft\"\ndoes = \"a thing\"\n{extra}"
            ),
        )
    }

    fn warehouse(cat: &Catalogue) -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        load(&mut conn, cat).unwrap();
        conn
    }

    fn one(conn: &Connection, sql: &str) -> i64 {
        conn.query_row(sql, [], |r| r.get(0)).unwrap()
    }

    const PROBE: &str =
        "[[line]]\npart = \"RP2350\"\nqty = 1\n\n[[line]]\ncommodity = \"wire\"\nqty = 6\n";

    #[test]
    fn the_repo_loads() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let conn = warehouse(&Catalogue::load(&root).unwrap());
        assert!(one(&conn, "SELECT count(*) FROM silver.part") >= 1);
        let claims = one(
            &conn,
            "SELECT count(*) FROM silver.claim WHERE build = 'debug-probe'",
        );
        let unknown = one(
            &conn,
            "SELECT CAST(sum(unknown) AS BIGINT) FROM gold.grade_coverage \
             WHERE subject LIKE 'debug-probe/v%'",
        );
        assert_eq!(claims, unknown, "nothing about debug-probe is shown yet");
    }

    #[test]
    fn every_row_traces_to_the_bytes_it_came_from() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let conn = warehouse(&Catalogue::load(&root).unwrap());
        let orphans = one(
            &conn,
            "SELECT count(*) FROM (SELECT record FROM silver.part \
             UNION ALL SELECT record FROM silver.build_version \
             UNION ALL SELECT record FROM silver.lab_limit \
             UNION ALL SELECT record FROM silver.trusted_entry \
             UNION ALL SELECT record FROM silver.lab_target \
             UNION ALL SELECT record FROM silver.lab_log_entry \
             UNION ALL SELECT record FROM silver.measurement \
             UNION ALL SELECT record FROM silver.sourcing) s \
             LEFT JOIN bronze.record r ON r.path = s.record WHERE r.path IS NULL",
        );
        assert_eq!(orphans, 0);
        let hashed = one(
            &conn,
            "SELECT count(*) FROM bronze.record_hash WHERE regexp_full_match(sha256, '[0-9a-f]{64}')",
        );
        assert_eq!(hashed, one(&conn, "SELECT count(*) FROM bronze.record"));
    }

    /// Lineage names the file the trusted base was read from, wherever it was.
    #[test]
    fn a_trusted_entry_traces_to_the_file_it_came_from() {
        let base = src(
            "elsewhere/trusted.toml",
            "[[entry]]\nid = \"a\"\nassumes = \"x\"\n",
        );
        let conn = warehouse(&Catalogue::from_sources(&base, &[], &[]).unwrap());
        let traced = one(
            &conn,
            "SELECT count(*) FROM silver.trusted_entry t JOIN bronze.record r ON r.path = t.record \
             WHERE t.record = 'elsewhere/trusted.toml'",
        );
        assert_eq!(traced, 1);
    }

    /// Schema first: a file in the records' folders is loaded, or CI fails; a record is parsed by its
    /// schema on the way in. A folder's README describes the folder and is not a record.
    #[test]
    fn every_file_in_the_records_folders_is_loaded() {
        fn walk(dir: &Path, root: &Path, out: &mut Vec<String>) {
            for entry in fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    walk(&path, root, out);
                } else if !path.ends_with("README.md") {
                    let rel = path.strip_prefix(root).unwrap();
                    out.push(rel.to_string_lossy().replace('\\', "/"));
                }
            }
        }
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut files = vec!["DISCLAIMER.md".to_string(), TRUSTED_BASE.to_string()];
        for dir in ["parts", "builds", "lab"] {
            walk(&root.join(dir), &root, &mut files);
        }
        let conn = warehouse(&Catalogue::load(&root).unwrap());
        for f in files {
            let n: i64 = conn
                .query_row(
                    "SELECT count(*) FROM bronze.record WHERE path = ?",
                    [&f],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(n, 1, "{f} sits with the records, but nothing loads it");
        }
    }

    /// A lab log of three entries over two days, written as `tentzhen log append` writes them, in
    /// a folder of its own named for `test`.
    fn lab_log(test: &str) -> Vec<Source> {
        let root =
            std::env::temp_dir().join(format!("tentzhen-warehouse-{}-{test}", std::process::id()));
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        fs::create_dir_all(root.join("lab")).unwrap();
        fs::write(root.join(LIMITS), "limits").unwrap();
        let at = |secs| UNIX_EPOCH + Duration::from_secs(secs);
        let one = |key: &str, d| BTreeMap::from([(key.to_string(), d)]);
        let none = BTreeMap::new;
        for (secs, by, what, params, result) in [
            (
                1_790_000_000,
                "person:jon",
                "supply.set",
                one("set_volts", log::Datum::Number(3.3)),
                one("done", log::Datum::Flag(true)),
            ),
            (
                1_790_000_060,
                "agent:claude",
                "supply.read",
                none(),
                one("read_volts", log::Datum::Number(3.29)),
            ),
            (
                1_790_100_000,
                "person:jon",
                "supply.off",
                none(),
                one("note", log::Datum::Text("done for the day".into())),
            ),
        ] {
            log::append(&root, at(secs), by, what, params, result).unwrap();
        }
        let files = log::read(&root).unwrap();
        fs::remove_dir_all(&root).unwrap();
        files
    }

    /// `cat` with the lab log of [`lab_log`], and a measurement record under each of `ids` of build
    /// probe v1 over the log's first two entries: seen, with no meter.
    fn measured(cat: Catalogue, test: &str, ids: &[&str]) -> Catalogue {
        let cat = cat.with_log(lab_log(test)).unwrap();
        let (first, last) = (cat.log[0].sha256.clone(), cat.log[1].sha256.clone());
        let records = ids
            .iter()
            .map(|id| {
                src(
                    &format!("lab/records/{id}.toml"),
                    &format!(
                        "measures = \"x\"\nsetup = \"x\"\nlog = {{ first = \"{first}\", last = \"{last}\" }}\n\n\
                         [[device]]\nbuild = \"probe\"\nversion = 1\n\n[results]\nseen = true\n"
                    ),
                )
            })
            .collect();
        cat.with_measurements(records).unwrap()
    }

    #[test]
    fn a_measurement_record_loads_with_the_claim_it_shows() {
        let dmm = "part = \"DMM\"\nkind = \"product\"\nis = \"multimeter\"\n\n\
                   [[claim]]\nid = \"dc-volts\"\nsays = \"x\"\ntrusted = \"entry\"\n";
        let claim = "[[claim]]\nid = \"draws\"\nsays = \"x\"\nrecord = \"2026-09-21-draw\"\n";
        let cat = Catalogue::from_sources(
            &base(),
            &[src("parts/dmm.toml", dmm), src("parts/rp2350.toml", PART)],
            &[build("probe", claim)],
        )
        .unwrap()
        .with_log(lab_log("measurement"))
        .unwrap();
        let (first, last) = (cat.log[0].sha256.clone(), cat.log[2].sha256.clone());
        let record = format!(
            "measures = \"x\"\nsetup = \"x\"\nlog = {{ first = \"{first}\", last = \"{last}\" }}\n\
             toolchain = {{ probe-rs = \"0.29.1\" }}\n\n\
             [[device]]\nbuild = \"probe\"\nversion = 1\nfirmware_sha256 = \"{}\"\n\n\
             [[device]]\npart = \"RP2350\"\nrevision = \"B2\"\n\n\
             [[meter]]\npart = \"DMM\"\naccuracy = \"DMM#dc-volts\"\n\n\
             [results]\ninput_volts = 3.3\ninput_codes = 4095\nenumerates = true\nproduct = \"x\"\n",
            "b".repeat(64)
        );
        let path = "lab/records/2026-09-21-draw.toml";
        let cat = cat.with_measurements(vec![src(path, &record)]).unwrap();
        let conn = warehouse(&cat);
        assert_eq!(
            one(
                &conn,
                "SELECT count(*) FROM silver.measurement m \
                 JOIN silver.lab_log_entry f ON f.sha256 = m.log_first AND f.seq = 1 \
                 JOIN silver.lab_log_entry l ON l.sha256 = m.log_last AND l.seq = 3 \
                 JOIN bronze.record r ON r.path = m.record \
                 WHERE m.measurement = '2026-09-21-draw'"
            ),
            1,
            "it names its log entries and traces to its file"
        );
        let rows = |sql: &str| one(&conn, &format!("SELECT count(*) FROM {sql}"));
        assert_eq!(
            [
                rows("silver.measurement_device WHERE role = 'device'"),
                rows(
                    "silver.measurement_device WHERE role = 'meter' AND accuracy = 'DMM#dc-volts'"
                ),
                rows("silver.measurement_device WHERE revision = 'B2' AND part = 'RP2350'"),
                rows("silver.measurement_tool WHERE tool = 'probe-rs' AND version = '0.29.1'"),
            ],
            [2, 1, 1, 1]
        );
        let kinds = |col: &str| rows(&format!("silver.measurement_value WHERE {col} IS NOT NULL"));
        assert_eq!([kinds("amount"), kinds("text"), kinds("flag")], [2, 1, 1]);
        let (grade, by): (String, String) = conn
            .query_row(
                "SELECT bench_grade, bench_evidence FROM gold.claim_grades WHERE claim = 'probe/v1#draws'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(
            (grade.as_str(), by.as_str()),
            ("measured", "2026-09-21-draw")
        );
    }

    #[test]
    fn a_lab_target_loads_with_the_board_it_names() {
        let list = "[[target]]\nid = \"pico-1\"\npart = \"RP2350\"\nserial = \"E661\"\n\
                    listed = \"2026-10-14\"\napproved_by = \"person:jon\"\n\n\
                    [[target]]\nid = \"probe-1\"\nbuild = \"probe\"\nversion = 1\n\
                    serial = \"E662\"\nlisted = \"2026-10-14\"\napproved_by = \"person:jon\"\n\
                    retired = \"2026-10-20\"\n";
        let cat = Catalogue::from_sources(
            &base(),
            &[src("parts/rp2350.toml", PART)],
            &[build("probe", PROBE)],
        )
        .unwrap()
        .with_targets(src("lab/targets.toml", list))
        .unwrap();
        let conn = warehouse(&cat);
        let rows = |sql: &str| one(&conn, &format!("SELECT count(*) FROM {sql}"));
        assert_eq!(
            [
                rows(
                    "silver.lab_target t JOIN silver.part p USING (part) \
                     WHERE t.target = 'pico-1' AND t.retired IS NULL"
                ),
                rows(
                    "silver.lab_target t JOIN silver.build_version b USING (build, version) \
                     WHERE t.target = 'probe-1' AND t.retired = DATE '2026-10-20'"
                ),
                rows(
                    "silver.lab_target t JOIN bronze.record r ON r.path = t.record \
                     WHERE t.listed = DATE '2026-10-14' AND t.approved_by = 'person:jon'"
                ),
            ],
            [1, 1, 2]
        );
    }

    #[test]
    fn the_lab_log_loads_in_order() {
        let cat = Catalogue::from_sources(&base(), &[], &[])
            .unwrap()
            .with_log(lab_log("order"))
            .unwrap();
        let conn = warehouse(&cat);
        assert_eq!(one(&conn, "SELECT count(*) FROM silver.lab_log_entry"), 3);
        assert_eq!(
            one(
                &conn,
                "SELECT count(*) FROM silver.lab_log_entry e \
                 JOIN silver.lab_log_entry p ON e.prev = p.sha256 AND e.seq = p.seq + 1"
            ),
            2,
            "each entry after the first names the one before it"
        );
        assert_eq!(
            one(
                &conn,
                "SELECT count(*) FROM silver.lab_log_entry e JOIN bronze.record r ON r.path = e.record"
            ),
            3
        );
        assert_eq!(
            one(
                &conn,
                "SELECT count(*) FROM bronze.record WHERE path LIKE 'lab/log/%'"
            ),
            2
        );
        let who: (String, String, String) = conn
            .query_row(
                "SELECT logged_at::VARCHAR, by_kind, by_name FROM silver.lab_log_entry WHERE seq = 2",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(
            who,
            (
                "2026-09-21 14:14:20".into(),
                "agent".into(),
                "claude".into()
            )
        );
        let kinds = |col: &str| {
            one(
                &conn,
                &format!("SELECT count(*) FROM silver.lab_log_value WHERE {col} IS NOT NULL"),
            )
        };
        assert_eq!([kinds("amount"), kinds("text"), kinds("flag")], [2, 1, 1]);
    }

    #[test]
    fn the_lab_limits_have_a_hash() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let conn = warehouse(&Catalogue::load(&root).unwrap());
        assert_eq!(
            one(
                &conn,
                "SELECT count(*) FROM bronze.record_hash WHERE path = 'lab/limits.toml'"
            ),
            1
        );
    }

    /// Each limit's weakest grade and the claims behind it, as gold.limit_grades gives them.
    fn grade(conn: &Connection, path: &str) -> (Option<String>, Option<String>) {
        conn.query_row(
            "SELECT weakest_grade, rests_on FROM gold.limit_grades WHERE path = ?",
            [path],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap()
    }

    fn some(grade: &str, claims: &str) -> (Option<String>, Option<String>) {
        (Some(grade.into()), Some(claims.into()))
    }

    #[test]
    fn every_limit_gets_the_weakest_grade_of_its_claims() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let conn = warehouse(&Catalogue::load(&root).unwrap());
        assert_eq!(
            one(&conn, "SELECT count(*) FROM silver.lab_limit"),
            one(&conn, "SELECT count(*) FROM gold.limit_grades"),
        );
        let io = "RP2350#io-supply";
        assert_eq!(
            grade(&conn, "supply.upstream.volts"),
            some(
                "unknown",
                &format!("DPS5005#input-range, {io}, bench-supply#setting-error")
            ),
            "the bench supply's setting error is not measured",
        );
        assert_eq!(
            grade(&conn, "supply.dps.current_limit_amps"),
            some("trusted", "DPS5005#current-limit"),
        );
        // The Pico ceiling, and every value copied from it.
        for path in [
            "pico_3v3.max_volts",
            "supply.max_volts",
            "supply.dps.max_setpoint_volts",
        ] {
            assert_eq!(grade(&conn, path), some("trusted", io), "{path}");
        }
        assert_eq!(grade(&conn, "pico_3v3.max_amps"), (None, None));
        assert_eq!(
            one(
                &conn,
                "SELECT count(*) FROM gold.limit_grades WHERE weakest_grade IS NOT NULL"
            ),
            5,
            "only those five rest on a claim",
        );
        assert_eq!(
            one(
                &conn,
                "SELECT count(*) FROM silver.lab_limit_at_most WHERE path = 'pico_3v3.max_volts' \
                 AND claim = 'RP2350#io-supply' AND name = 'max_io_supply_volts'"
            ),
            1
        );
    }

    #[test]
    fn a_grade_flows_through_every_copy_and_rule() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut cat = Catalogue::load(&root).unwrap();
        let text = fs::read_to_string(root.join(LIMITS)).unwrap().replace(
            "basis.max_amps = { policy = \"CLAUDE.md, \\\"Default ceilings: 3.4 V and 200 mA\\\"\" }",
            "basis.max_amps = { policy = \"x\", rests_on = [\"bench-supply#setting-error\"] }",
        );
        cat.limits = Some(Limits::parse(&text).unwrap());
        let conn = warehouse(&cat);
        let bench = "bench-supply#setting-error";
        assert_eq!(grade(&conn, "pico_3v3.max_amps"), some("unknown", bench));
        // A copy, a copy of a copy that rests on a trusted claim too, and a rule's input.
        assert_eq!(grade(&conn, "supply.max_amps"), some("unknown", bench));
        assert_eq!(
            grade(&conn, "supply.dps.current_limit_amps"),
            some("unknown", &format!("DPS5005#current-limit, {bench}")),
        );
        assert_eq!(
            grade(&conn, "supply.fuse.rating_amps"),
            some("unknown", bench)
        );
        assert_eq!(
            grade(&conn, "pico_3v3.max_volts"),
            some("trusted", "RP2350#io-supply")
        );
    }

    #[test]
    fn the_catalogue_is_current() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let committed = fs::read_to_string(root.join(catalogue::PATH)).unwrap_or_default();
        assert!(
            committed == catalogue::generate(&root).unwrap(),
            "{} is stale: run `cargo run -p tentzhen-warehouse -- catalogue`",
            catalogue::PATH
        );
        assert!(
            !committed.contains("\n- ``\n"),
            "{} lists a constraint with no text",
            catalogue::PATH
        );
    }

    fn relations() -> Vec<catalogue::Relation> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(SCHEMA).unwrap();
        catalogue::relations(&conn, &catalogue::sources(&root).unwrap()).unwrap()
    }

    #[test]
    fn every_table_and_view_says_what_it_is() {
        let rels = relations();
        assert!(
            rels.len() > 20,
            "the catalogue found {} relations",
            rels.len()
        );
        for r in rels {
            assert!(
                r.comment.as_deref().is_some_and(|c| !c.is_empty()),
                "{} has no COMMENT ON in schema.sql",
                r.name
            );
        }
    }

    /// The site's pages come from gold alone: a page never reaches past the marts.
    #[test]
    fn the_site_reads_only_gold() {
        let rels = relations();
        let read: Vec<&str> = rels
            .iter()
            .filter(|r| r.read_by.iter().any(|c| c == "crates/site"))
            .map(|r| r.name.as_str())
            .collect();
        assert!(
            !read.is_empty(),
            "the site reads nothing from the warehouse"
        );
        for name in read {
            assert!(name.starts_with("gold."), "the site reads {name}");
        }
    }

    #[test]
    fn every_version_has_one_page_and_its_drawing() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let conn = warehouse(&Catalogue::load(&root).unwrap());
        assert_eq!(
            one(&conn, "SELECT count(*) FROM gold.page"),
            one(&conn, "SELECT count(*) FROM silver.build_version"),
        );
        let missing = one(
            &conn,
            "SELECT count(*) FROM silver.build_version bv JOIN gold.page p USING (build, version) \
             WHERE bv.drawing IS NOT NULL AND p.drawing_svg IS NULL",
        );
        assert_eq!(missing, 0, "a declared drawing reaches its page");
        assert_eq!(one(&conn, "SELECT count(*) FROM gold.legal"), 1);
    }

    #[test]
    fn quantities_multiply_through_builds() {
        let bench = build(
            "bench",
            "[[uses]]\nbuild = \"probe\"\nversion = 1\nqty = 2\n",
        );
        let cat = Catalogue::from_sources(
            &base(),
            &[src("parts/rp2350.toml", PART)],
            &[build("probe", PROBE), bench],
        )
        .unwrap();
        let conn = warehouse(&cat);
        let q = |what: &str| {
            one(
                &conn,
                &format!("SELECT qty FROM gold.bom_exploded WHERE build = 'bench' AND {what}"),
            )
        };
        assert_eq!(q("part = 'RP2350'"), 2);
        assert_eq!(q("commodity = 'wire'"), 12);
        assert_eq!(
            one(
                &conn,
                "SELECT count(*) FROM gold.where_used WHERE part = 'RP2350'"
            ),
            2
        );
    }

    /// A build version of `name` that writes `extra`.
    fn version(name: &str, v: u32, extra: &str) -> Source {
        src(
            &format!("builds/{name}/v{v}.toml"),
            &format!(
                "build = \"{name}\"\nversion = {v}\nstatus = \"draft\"\ndoes = \"a thing\"\n\
                 changes = \"more\"\n{extra}"
            ),
        )
    }

    /// Each row of the parts page as (part or commodity, with its form if it has one, qty, status,
    /// since), and the build versions it is used in, with how many each needs.
    fn parts_page(conn: &Connection) -> Vec<(String, i64, String, Option<String>, String)> {
        let mut stmt = conn
            .prepare(
                "SELECT coalesce(p.part, p.commodity) || coalesce(' (' || p.form || ')', ''), \
                 p.qty, p.status, CAST(p.since AS TEXT), \
                 string_agg(u.build || ' v' || u.version || ' x' || u.qty, ', ' \
                            ORDER BY u.build, u.version) \
                 FROM gold.page_parts p JOIN gold.page_parts_used_in u USING (parts_no) \
                 GROUP BY p.parts_no, p.part, p.form, p.commodity, p.qty, p.status, p.since \
                 ORDER BY p.parts_no",
            )
            .unwrap();
        stmt.query_map([], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })
        .unwrap()
        .collect::<duckdb::Result<_>>()
        .unwrap()
    }

    /// The parts list buys for the newest version of each build, once: a build another build's
    /// newest version uses is counted inside it, and an older version not at all, nor what only an
    /// older version uses. A part in two forms is two things to buy.
    #[test]
    fn the_parts_list_counts_each_build_once() {
        let bench = build(
            "bench",
            "[[uses]]\nbuild = \"probe\"\nversion = 1\nqty = 2\n\n\
             [[line]]\ncommodity = \"wire\"\nqty = 1\n",
        );
        let tool = |v: u32, qty: u32| {
            let line = format!("[[line]]\npart = \"RP2350\"\nqty = {qty}\n");
            if v == 1 {
                build("tool", &line)
            } else {
                version("tool", v, &line)
            }
        };
        let gizmo = build(
            "gizmo",
            "[[line]]\ncommodity = \"glue\"\nqty = 1\n\n\
             [[line]]\npart = \"RP2350\"\nform = \"chip\"\nqty = 1\n",
        );
        let rig = [
            build("rig", "[[uses]]\nbuild = \"gizmo\"\nversion = 1\n"),
            version("rig", 2, "[[line]]\ncommodity = \"tape\"\nqty = 1\n"),
        ];
        let cat = Catalogue::from_sources(
            &base(),
            &[src("parts/rp2350.toml", PART)],
            &[
                &[build("probe", PROBE), bench, tool(1, 7), tool(2, 3), gizmo],
                &rig[..],
            ]
            .concat(),
        )
        .unwrap();
        let conn = warehouse(&cat);
        let bench: Vec<(String, i64)> = conn
            .prepare("SELECT build, version FROM gold.bench ORDER BY build")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .collect::<duckdb::Result<_>>()
            .unwrap();
        let bv = |b: &str, v| (b.to_string(), v);
        assert_eq!(
            bench,
            [bv("bench", 1), bv("gizmo", 1), bv("rig", 2), bv("tool", 2)]
        );
        let specced =
            |what: &str, qty, used: &str| (what.into(), qty, "specced".into(), None, used.into());
        assert_eq!(
            parts_page(&conn),
            [
                specced("wire", 13, "bench v1 x1, probe v1 x12"),
                specced("glue", 1, "gizmo v1 x1"),
                specced("RP2350 (chip)", 1, "gizmo v1 x1"),
                specced("RP2350", 5, "probe v1 x2, tool v2 x3"),
                specced("tape", 1, "rig v2 x1"),
            ]
        );
    }

    /// A probe build, and a kit of commodities, one for each stage, with `sourcing` and the records
    /// of [`measured`].
    fn sourced(test: &str, ids: &[&str], sourcing: &str) -> Connection {
        let kit: String = [
            "listed",
            "carted",
            "ordered",
            "arrived",
            "inspected",
            "none",
            "undated",
        ]
        .iter()
        .map(|c| format!("[[line]]\ncommodity = \"{c}\"\nqty = 1\n\n"))
        .collect();
        let cat = Catalogue::from_sources(
            &base(),
            &[src("parts/rp2350.toml", PART)],
            &[build("probe", PROBE), build("kit", &kit)],
        )
        .unwrap();
        let cat = measured(cat, test, ids)
            .with_sourcing(src("lab/sourcing.toml", sourcing))
            .unwrap();
        warehouse(&cat)
    }

    /// A status is the last stage a line has reached, from the days its stages give and, for
    /// incoming QA, its record's.
    #[test]
    fn a_status_comes_from_its_stages() {
        // Each line reaches every stage up to its own, on a day of its own, so the last one wins.
        // `none` names a shop and has reached no stage; `wire` has no line. `undated` was ordered on
        // a day the file does not give, after its listing was checked on one it does.
        let stages = [
            "shop = \"lcsc\"\nitem = \"C2040\"\nlisting_checked = \"2026-09-01\"",
            "in_cart = \"2026-09-02\"",
            "ordered = \"2026-09-03\"",
            "arrived = \"2026-09-04\"",
            "passed_qa = \"2026-09-21-kit-qa\"",
        ];
        let line = |what: &str, reached: usize| {
            format!("[[line]]\n{what}\n{}\n\n", stages[..reached].join("\n"))
        };
        let sourcing = [
            line("part = \"RP2350\"", 3),
            line("commodity = \"listed\"", 1),
            line("commodity = \"carted\"", 2),
            line("commodity = \"ordered\"", 3),
            line("commodity = \"arrived\"", 4),
            line("commodity = \"inspected\"", 5),
            "[[line]]\ncommodity = \"none\"\nshop = \"lcsc\"\nitem = \"C2040\"\n\n".into(),
            "[[line]]\ncommodity = \"undated\"\nshop = \"lcsc\"\nitem = \"C2040\"\n\
             listing_checked = \"2026-09-01\"\nordered = true\n\n"
                .into(),
        ]
        .concat();
        let conn = sourced("status", &["2026-09-21-kit-qa"], &sourcing);
        let got: Vec<(String, String, Option<String>)> = parts_page(&conn)
            .into_iter()
            .map(|(what, _, status, since, _)| (what, status, since))
            .collect();
        let row = |what: &str, status: &str, since: Option<&str>| {
            (
                what.to_string(),
                status.to_string(),
                since.map(String::from),
            )
        };
        assert_eq!(
            got,
            [
                row("listed", "listing checked", Some("2026-09-01")),
                row("carted", "in cart", Some("2026-09-02")),
                row("ordered", "ordered", Some("2026-09-03")),
                row("arrived", "arrived", Some("2026-09-04")),
                row("inspected", "passed incoming QA", Some("2026-09-21")),
                row("none", "specced", None),
                row("undated", "ordered", None),
                row("RP2350", "ordered", Some("2026-09-03")),
                row("wire", "specced", None),
            ]
        );
        let qa: String = conn
            .query_row(
                "SELECT passed_qa FROM gold.page_parts WHERE commodity = 'inspected'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(qa, "2026-09-21-kit-qa");
    }

    /// Sourcing lines the parts page does not show.
    const OFF_THE_PARTS_LIST: &str = "SELECT count(*) FROM silver.sourcing s WHERE NOT EXISTS ( \
         SELECT 1 FROM gold.page_parts p WHERE p.part IS NOT DISTINCT FROM s.part \
         AND p.form IS NOT DISTINCT FROM s.form AND p.commodity IS NOT DISTINCT FROM s.commodity)";

    /// crates/records holds a sourcing line to a line some build version writes; this holds the
    /// repo's to the parts list, which buys for the newest versions only. The control is a line
    /// only an older version writes, which loads and is counted.
    #[test]
    fn every_sourcing_line_is_on_the_parts_list() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let conn = warehouse(&Catalogue::load(&root).unwrap());
        assert!(one(&conn, "SELECT count(*) FROM silver.sourcing") >= 1);
        assert_eq!(
            one(&conn, OFF_THE_PARTS_LIST),
            0,
            "a line in lab/sourcing.toml is for a build's older version: drop it, or name the \
             line as the newest version writes it"
        );

        let newer = version("probe", 2, "[[line]]\npart = \"RP2350\"\nqty = 1\n");
        let cat = Catalogue::from_sources(
            &base(),
            &[src("parts/rp2350.toml", PART)],
            &[build("probe", PROBE), newer],
        )
        .unwrap()
        .with_sourcing(src("lab/sourcing.toml", "[[line]]\ncommodity = \"wire\"\n"))
        .unwrap();
        assert_eq!(one(&warehouse(&cat), OFF_THE_PARTS_LIST), 1);
    }

    /// Each claim's (sim grade, sim evidence, bench grade, bench evidence), by its id.
    fn grades(conn: &Connection) -> BTreeMap<String, [Option<String>; 4]> {
        let mut stmt = conn
            .prepare("SELECT id, sim_grade, sim_evidence, bench_grade, bench_evidence FROM gold.claim_grades")
            .unwrap();
        stmt.query_map([], |r| {
            Ok((r.get(0)?, [r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?]))
        })
        .unwrap()
        .map(|r| r.unwrap())
        .collect()
    }

    fn g(
        sim: &str,
        sim_by: Option<&str>,
        bench: &str,
        bench_by: Option<&str>,
    ) -> [Option<String>; 4] {
        [
            Some(sim.into()),
            sim_by.map(Into::into),
            Some(bench.into()),
            bench_by.map(Into::into),
        ]
    }

    #[test]
    fn grades_derive_from_referents() {
        let claims = "[[claim]]\nid = \"a\"\nsays = \"x\"\ntest = \"t\"\nrecord = \"2026-09-21-lab-1\"\n\n\
                      [[claim]]\nid = \"b\"\nsays = \"x\"\nproof = \"p\"\ncheck = \"c\"\ntest = \"t\"\n\n\
                      [[claim]]\nid = \"c\"\nsays = \"x\"\ntrusted = \"entry\"\nrecord = \"2026-09-21-lab-2\"\n\n\
                      [[claim]]\nid = \"d\"\nsays = \"x\"\ntrusted = \"entry\"\n\n\
                      [[claim]]\nid = \"e\"\nsays = \"x\"\n\n\
                      [[claim]]\nid = \"f\"\nsays = \"x\"\ncheck = \"c\"\ntest = \"t\"\n";
        let cat = Catalogue::from_sources(&base(), &[], &[build("probe", claims)]).unwrap();
        let conn = warehouse(&measured(
            cat,
            "grades",
            &["2026-09-21-lab-1", "2026-09-21-lab-2"],
        ));
        let got = grades(&conn);
        assert_eq!(
            got["a"],
            g("tested", Some("t"), "measured", Some("2026-09-21-lab-1"))
        );
        assert_eq!(
            got["b"],
            g("proven", Some("p"), "unknown", None),
            "the strongest shows"
        );
        assert_eq!(
            got["c"],
            g("unknown", None, "measured", Some("2026-09-21-lab-2")),
            "measured beats trusted"
        );
        assert_eq!(got["d"], g("unknown", None, "trusted", Some("entry")));
        assert_eq!(got["e"], g("unknown", None, "unknown", None));
        assert_eq!(
            got["f"],
            g("checked", Some("c"), "unknown", None),
            "a check beats a test"
        );
        let cover = |col: &str| {
            one(
                &conn,
                &format!("SELECT {col} FROM gold.grade_coverage WHERE subject = 'probe/v1'"),
            )
        };
        assert_eq!(
            [
                cover("claims"),
                cover("shown_in_sim"),
                cover("measured"),
                cover("trusted"),
                cover("unknown")
            ],
            [6, 3, 2, 1, 1]
        );
    }

    #[test]
    fn a_part_s_claims_are_graded_like_a_build_s() {
        let part = format!(
            "{PART}[[claim]]\nid = \"io-supply\"\nsays = \"x\"\ntrusted = \"entry\"\nvalues = {{ max_volts = 3.63 }}\n"
        );
        let conn = warehouse(
            &Catalogue::from_sources(
                &base(),
                &[src("parts/rp2350.toml", &part)],
                &[build("probe", PROBE)],
            )
            .unwrap(),
        );
        let (claim, sim, bench): (String, String, String) = conn
            .query_row(
                "SELECT claim, sim_grade, bench_grade FROM gold.claim_grades WHERE part = 'RP2350'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(
            (claim.as_str(), sim.as_str(), bench.as_str()),
            ("RP2350#io-supply", "unknown", "trusted")
        );
        let v: f64 = conn
            .query_row(
                "SELECT amount FROM silver.claim_value WHERE claim = 'RP2350#io-supply' AND name = 'max_volts'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(v, 3.63);
        assert_eq!(
            one(
                &conn,
                "SELECT trusted FROM gold.grade_coverage WHERE subject = 'RP2350'"
            ),
            1
        );
    }

    /// Fails unless the database refused the write with a constraint error, not some other error.
    fn refused(result: duckdb::Result<()>, why: &str) {
        let err = result
            .err()
            .unwrap_or_else(|| panic!("accepted, but {why}"))
            .to_string();
        assert!(
            err.contains("Constraint Error"),
            "{why}: refused for another reason: {err}"
        );
    }

    #[test]
    fn the_database_refuses_what_the_records_refuse() {
        let conn = warehouse(
            &Catalogue::from_sources(
                &base(),
                &[src("parts/rp2350.toml", PART)],
                &[build("probe", PROBE)],
            )
            .unwrap(),
        );
        let both = "INSERT INTO silver.line VALUES ('probe', 1, 9, 'RP2350', 'wire', NULL, 1)";
        refused(
            conn.execute(both, []).map(|_| ()),
            "a line names a part or a commodity, not both",
        );
        let unrecorded = "INSERT INTO silver.line VALUES ('probe', 1, 9, 'RP9999', NULL, NULL, 1)";
        refused(
            conn.execute(unrecorded, []).map(|_| ()),
            "a part must have a record",
        );
        let never = |why: &str| {
            format!(
                "INSERT INTO silver.build_version VALUES \
                 ('quiet', 1, 'draft', 'a thing', NULL, NULL, '{why}', 'builds/quiet/v1.toml')"
            )
        };
        refused(
            conn.execute(&never("  "), []).map(|_| ()),
            "a build that is never a lab target says why",
        );
        conn.execute(&never("it holds the limits"), []).unwrap();
        for (values, why) in [
            (
                "'k1', 'RP2350', 'probe', 1",
                "a claim is on a part or a build version, not both",
            ),
            ("'k2', NULL, NULL, NULL", "a claim is on something"),
            (
                "'k3', NULL, 'probe', NULL",
                "a build claim names its version",
            ),
            (
                "'k4', 'RP9999', NULL, NULL",
                "a part claim's part has a record",
            ),
            ("'k6', NULL, 'probe', 9", "a build claim's version exists"),
        ] {
            let sql = format!("INSERT INTO silver.claim VALUES ({values}, 1, 'a', 'x')");
            refused(conn.execute(&sql, []).map(|_| ()), why);
        }
        let graded = "INSERT INTO silver.claim VALUES ('k5', 'RP2350', NULL, NULL, 1, 'a', 'x'); \
                      INSERT INTO silver.referent VALUES ('k5', 'tested', 't', NULL, NULL)";
        refused(
            conn.execute_batch(graded),
            "a referent is a kind of evidence, not a grade",
        );
        for (values, why) in [
            (
                "'trusted', NULL, 'nope', NULL",
                "a trusted referent names an entry in the trusted base",
            ),
            (
                "'trusted', 'entry', NULL, NULL",
                "a trusted referent is a key",
            ),
            (
                "'trusted', 'entry', 'entry', NULL",
                "a trusted referent is a key, not a name too",
            ),
            (
                "'test', NULL, 'entry', NULL",
                "only a trusted referent names an entry",
            ),
            ("'test', NULL, NULL, NULL", "a referent names something"),
            (
                "'record', NULL, NULL, 'nope'",
                "a record referent names a measurement record",
            ),
            ("'record', 'm1', NULL, NULL", "a record referent is a key"),
        ] {
            let sql = format!("INSERT INTO silver.referent VALUES ('k5', {values})");
            refused(conn.execute(&sql, []).map(|_| ()), why);
        }
        conn.execute(
            "INSERT INTO silver.lab_limit VALUES ('held', 1.0, 'V', NULL, 'policy', 'p', NULL, 'r')",
            [],
        )
        .unwrap();
        refused(
            conn.execute(
                "INSERT INTO silver.lab_limit_rests_on VALUES ('held', 'RP2350#nope')",
                [],
            )
            .map(|_| ()),
            "a limit rests on a claim that exists",
        );
        // Each was refused for what it names: rows for the same claim and limit naming what
        // exists are accepted.
        conn.execute_batch(
            "INSERT INTO silver.referent VALUES ('k5', 'trusted', NULL, 'entry', NULL); \
             INSERT INTO silver.referent VALUES ('k5', 'test', 't', NULL, NULL); \
             INSERT INTO silver.lab_limit_rests_on VALUES ('held', 'k5')",
        )
        .unwrap();
        // A bound is a value of a claim the limit rests on.
        conn.execute_batch(
            "INSERT INTO silver.claim_value VALUES ('k5', 'max_volts', 3.63); \
             INSERT INTO silver.claim VALUES ('k7', 'RP2350', NULL, NULL, 2, 'b', 'x'); \
             INSERT INTO silver.claim_value VALUES ('k7', 'max_volts', 3.63)",
        )
        .unwrap();
        for (values, why) in [
            (
                "'held', 'k5', 'min_volts'",
                "a bound is a value its claim states",
            ),
            (
                "'held', 'k7', 'max_volts'",
                "a bound is on a claim the limit rests on",
            ),
            (
                "'nothing', 'k5', 'max_volts'",
                "a bound is on a limit that exists",
            ),
        ] {
            let sql = format!("INSERT INTO silver.lab_limit_at_most VALUES ({values})");
            refused(conn.execute(&sql, []).map(|_| ()), why);
        }
        conn.execute(
            "INSERT INTO silver.lab_limit_at_most VALUES ('held', 'k5', 'max_volts')",
            [],
        )
        .unwrap();
        let both =
            "INSERT INTO silver.lab_limit VALUES ('x', 1.0, 'V', 'y', 'policy', 'p', NULL, 'r')";
        refused(
            conn.execute(both, []).map(|_| ()),
            "a limit is an amount or a choice, not both",
        );
        let no_policy =
            "INSERT INTO silver.lab_limit VALUES ('x', 1.0, 'V', NULL, 'policy', NULL, NULL, 'r')";
        refused(
            conn.execute(no_policy, []).map(|_| ()),
            "a policy basis names its policy",
        );
        for (values, why) in [
            (
                "1.0, 'W', NULL, 'policy', 'p', NULL",
                "a limit is in volts or amps",
            ),
            (
                "1.0, NULL, NULL, 'policy', 'p', NULL",
                "an amount has a unit",
            ),
            (
                "1.0, 'V', NULL, 'guess', NULL, NULL",
                "a basis is policy, same_as or rule",
            ),
            (
                "1.0, 'V', NULL, 'rule', NULL, NULL",
                "a rule basis names its rule",
            ),
        ] {
            let sql = format!("INSERT INTO silver.lab_limit VALUES ('x', {values}, 'r')");
            refused(conn.execute(&sql, []).map(|_| ()), why);
        }
        // The lab log: one chain from one first entry, that never forks.
        let sha = |c: char| format!("'{}'", c.to_string().repeat(64));
        let entry = |seq: u32, own: char, prev: &str, by: &str| {
            format!(
                "INSERT INTO silver.lab_log_entry VALUES ({seq}, '2026-10-14 10:00:00', '{by}', \
                 'jon', 'supply.set', {}, {}, {prev}, 'r')",
                sha('a'),
                sha(own)
            )
        };
        conn.execute_batch(&entry(1, 'b', "NULL", "person"))
            .unwrap();
        for (sql, why) in [
            (
                entry(2, 'c', &sha('f'), "person"),
                "a prev names an entry in the log",
            ),
            (
                entry(2, 'c', "NULL", "person"),
                "only the first entry names no prev",
            ),
            (
                entry(2, 'c', &sha('b'), "robot"),
                "a person or an agent acts",
            ),
        ] {
            refused(conn.execute_batch(&sql), why);
        }
        conn.execute_batch(&entry(2, 'c', &sha('b'), "agent"))
            .unwrap();
        refused(
            conn.execute_batch(&entry(3, 'd', &sha('b'), "person")),
            "the log cannot fork: no two entries name the same prev",
        );
        conn.execute_batch(&entry(3, 'd', &sha('c'), "person"))
            .unwrap();
        let value = |values: &str| format!("INSERT INTO silver.lab_log_value VALUES ({values})");
        for (values, why) in [
            (
                "2, 'result', 'x', 1.0, 'y', NULL",
                "a value is a number, a word or a yes/no, not two",
            ),
            ("2, 'result', 'x', NULL, NULL, NULL", "a value is something"),
            (
                "2, 'other', 'x', 1.0, NULL, NULL",
                "a value is a param or a result",
            ),
            (
                "9, 'result', 'x', 1.0, NULL, NULL",
                "a value belongs to an entry",
            ),
        ] {
            refused(conn.execute_batch(&value(values)), why);
        }
        conn.execute_batch(&value("2, 'result', 'x', 1.0, NULL, NULL"))
            .unwrap();
        // Measurement records: over entries in the log, of parts and builds that exist, each
        // meter's accuracy a claim on its own record, and cited by record referents.
        let measurement = |name: &str, first: &str| {
            format!(
                "INSERT INTO silver.measurement VALUES ('{name}', 'x', 'x', {first}, {}, 'r')",
                sha('c')
            )
        };
        refused(
            conn.execute_batch(&measurement("m0", &sha('f'))),
            "a record's log entries are in the log",
        );
        conn.execute_batch(&measurement("m1", &sha('b'))).unwrap();
        conn.execute_batch("INSERT INTO silver.claim VALUES ('k8', NULL, 'probe', 1, 1, 'c', 'x')")
            .unwrap();
        let device =
            |values: &str| format!("INSERT INTO silver.measurement_device VALUES ('m1', {values})");
        for (values, why) in [
            (
                "'device', 1, 'RP2350', 'probe', 1, NULL, NULL, NULL, NULL",
                "a device is a part or a build, not both",
            ),
            (
                "'device', 1, NULL, 'probe', NULL, NULL, NULL, NULL, NULL",
                "a build names its version",
            ),
            (
                "'device', 1, 'RP9999', NULL, NULL, NULL, NULL, NULL, NULL",
                "a device's part has a record",
            ),
            (
                "'device', 1, NULL, 'probe', 9, NULL, NULL, NULL, NULL",
                "a device's build version exists",
            ),
            (
                "'device', 1, NULL, 'probe', 1, 'B', NULL, NULL, NULL",
                "a build's revision is its version",
            ),
            (
                "'device', 1, 'RP2350', NULL, NULL, NULL, 'abc', NULL, NULL",
                "a firmware hash is a sha256",
            ),
            (
                "'device', 1, 'RP2350', NULL, NULL, NULL, NULL, NULL, 'k5'",
                "a device measured has no accuracy",
            ),
            (
                "'meter', 1, 'RP2350', NULL, NULL, NULL, NULL, NULL, NULL",
                "a meter names its accuracy",
            ),
            (
                "'meter', 1, 'RP2350', NULL, NULL, NULL, NULL, NULL, 'k8'",
                "a part's accuracy is a claim on that part",
            ),
            (
                "'meter', 1, NULL, 'probe', 1, NULL, NULL, NULL, 'k5'",
                "a build's accuracy is a claim on that build version",
            ),
            (
                "'meter', 1, 'RP2350', NULL, NULL, NULL, NULL, NULL, 'nope'",
                "an accuracy is a claim",
            ),
        ] {
            refused(conn.execute_batch(&device(values)), why);
        }
        conn.execute_batch(&format!(
            "{}; {}; {}",
            device("'device', 1, 'RP2350', NULL, NULL, 'B2', NULL, NULL, NULL"),
            device("'meter', 1, 'RP2350', NULL, NULL, NULL, NULL, NULL, 'k5'"),
            device("'meter', 2, NULL, 'probe', 1, NULL, NULL, NULL, 'k8'"),
        ))
        .unwrap();
        for (sql, why) in [
            (
                "INSERT INTO silver.measurement_value VALUES ('m1', 'x', 1.0, NULL, true)",
                "a result is a number, a word or a yes/no, not two",
            ),
            (
                "INSERT INTO silver.measurement_value VALUES ('m9', 'x', 1.0, NULL, NULL)",
                "a result belongs to a record",
            ),
            (
                "INSERT INTO silver.measurement_tool VALUES ('m9', 'probe-rs', '0.29.1')",
                "a tool belongs to a record",
            ),
            (
                "INSERT INTO silver.referent VALUES ('k7', 'record', 'm1', NULL, 'm1')",
                "a record referent is a key, not a name too",
            ),
            (
                "INSERT INTO silver.referent VALUES ('k7', 'test', 't', NULL, 'm1')",
                "only a record referent names a measurement record",
            ),
        ] {
            refused(conn.execute_batch(sql), why);
        }
        // The lab-target list: one board each, a part or a build version with a record, told apart
        // by its serial, listed by a person, and retired only after it was listed.
        let target = |values: &str, by: &str, retired: &str| {
            format!(
                "INSERT INTO silver.lab_target VALUES ({values}, '2026-10-14', {by}, {retired}, 'r')"
            )
        };
        let jon = |values: &str| target(values, "'person:jon'", "NULL");
        conn.execute_batch(&jon("'t1', 'RP2350', NULL, NULL, 'S1'"))
            .unwrap();
        for (sql, why) in [
            (
                jon("'t2', 'RP2350', 'probe', 1, 'S2'"),
                "a target is a part or a build, not both",
            ),
            (jon("'t2', NULL, NULL, NULL, 'S2'"), "a target is something"),
            (
                jon("'t2', NULL, 'probe', NULL, 'S2'"),
                "a build target names its version",
            ),
            (
                jon("'t2', 'RP9999', NULL, NULL, 'S2'"),
                "a target's part has a record",
            ),
            (
                jon("'t2', NULL, 'probe', 9, 'S2'"),
                "a target's build version exists",
            ),
            (
                jon("'t2', 'RP2350', NULL, NULL, 'S1'"),
                "no two targets give the same serial",
            ),
            (
                jon("'t1', 'RP2350', NULL, NULL, 'S2'"),
                "no two targets have the same id",
            ),
            (
                target("'t2', 'RP2350', NULL, NULL, 'S2'", "'agent:claude'", "NULL"),
                "a person lists a target",
            ),
            (
                target(
                    "'t2', 'RP2350', NULL, NULL, 'S2'",
                    "'person:jon'",
                    "'2026-10-14'",
                ),
                "a target retires after the day it was listed",
            ),
        ] {
            refused(conn.execute_batch(&sql), why);
        }
        conn.execute_batch(&target(
            "'t2', NULL, 'probe', 1, 'S2'",
            "'person:jon'",
            "'2026-10-15'",
        ))
        .unwrap();
        conn.execute_batch(
            "INSERT INTO silver.measurement_value VALUES ('m1', 'x', 1.0, NULL, NULL); \
             INSERT INTO silver.measurement_tool VALUES ('m1', 'probe-rs', '0.29.1'); \
             INSERT INTO silver.referent VALUES ('k7', 'record', NULL, NULL, 'm1')",
        )
        .unwrap();
        // Part, form, commodity, shop, store, item; whether each stage is reached (listing_checked,
        // in_cart, ordered, arrived), then the day of each; passed_qa.
        let sourcing = |line: &str| {
            format!(
                "INSERT INTO silver.sourcing (line_no, part, form, commodity, shop, store, item, \
                 listing_checked, in_cart, ordered, arrived, listing_checked_on, in_cart_on, \
                 ordered_on, arrived_on, passed_qa, record) \
                 VALUES (9, {line}, 'lab/sourcing.toml')"
            )
        };
        let none = "false, false, false, false, NULL, NULL, NULL, NULL, NULL";
        for (line, why) in [
            (
                format!("'RP2350', NULL, 'wire', NULL, NULL, NULL, {none}"),
                "a sourcing line names a part or a commodity, not both",
            ),
            (
                format!("NULL, NULL, NULL, NULL, NULL, NULL, {none}"),
                "a sourcing line names something",
            ),
            (
                format!("'RP9999', NULL, NULL, NULL, NULL, NULL, {none}"),
                "a sourcing line's part has a record",
            ),
            (
                format!("'RP2350', NULL, NULL, 'ebay', NULL, '1', {none}"),
                "a shop is one the records know",
            ),
            (
                format!("'RP2350', NULL, NULL, 'lcsc', NULL, NULL, {none}"),
                "a shop comes with its item number",
            ),
            (
                "'RP2350', NULL, NULL, NULL, NULL, NULL, false, false, true, false, \
                 NULL, NULL, NULL, NULL, NULL"
                    .into(),
                "an order names its shop",
            ),
            (
                "'RP2350', NULL, NULL, 'lcsc', NULL, 'C1', false, false, false, false, \
                 NULL, NULL, '2026-10-02', NULL, NULL"
                    .into(),
                "a stage's day comes with the stage",
            ),
            (
                "'RP2350', NULL, NULL, NULL, NULL, NULL, false, false, false, false, \
                 NULL, NULL, NULL, NULL, 'nope'"
                    .into(),
                "incoming QA names a measurement record",
            ),
        ] {
            refused(conn.execute(&sourcing(&line), []).map(|_| ()), why);
        }
        // Each pair of stages, the later a day before the earlier, with any stage between not
        // reached: listing_checked, in_cart, ordered, arrived.
        for (i, j) in [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)] {
            let (mut reached, mut days) = (["false"; 4], ["NULL"; 4]);
            (reached[i], reached[j]) = ("true", "true");
            days[i] = "'2026-10-03'";
            days[j] = "'2026-10-02'";
            let line = format!(
                "'RP2350', NULL, NULL, 'lcsc', NULL, 'C1', {}, {}, NULL",
                reached.join(", "),
                days.join(", ")
            );
            refused(
                conn.execute(&sourcing(&line), []).map(|_| ()),
                "the days run in the stages' order",
            );
        }
        // A part salvaged with a shop, and one salvaged and marked used as well.
        let salvaged = |shop: &str, item: &str, used: bool| {
            format!(
                "INSERT INTO silver.sourcing (line_no, part, shop, item, used, salvaged_from, \
                 listing_checked, in_cart, ordered, arrived, record) VALUES (8, 'RP2350', {shop}, \
                 {item}, {used}, 'a printer', false, false, false, true, 'lab/sourcing.toml')"
            )
        };
        refused(
            conn.execute(&salvaged("'lcsc'", "'C1'", false), [])
                .map(|_| ()),
            "a salvaged part has no listing",
        );
        refused(
            conn.execute(&salvaged("NULL", "NULL", true), [])
                .map(|_| ()),
            "a salvaged part is used already",
        );
        conn.execute(&salvaged("NULL", "NULL", false), []).unwrap();
        // Where the maker lists a seller: with no seller named, and beside a salvaged part.
        for (cols, vals, why) in [
            ("authorized", "'a list'", "authorized names its seller"),
            (
                "store, authorized, salvaged_from",
                "'Goodwill', 'a list', 'a printer'",
                "an authorized distributor sells a part new",
            ),
        ] {
            refused(
                conn.execute(
                    &format!(
                        "INSERT INTO silver.sourcing (line_no, part, {cols}, listing_checked, \
                         in_cart, ordered, arrived, record) VALUES (6, 'RP2350', {vals}, false, \
                         false, false, true, 'lab/sourcing.toml')"
                    ),
                    [],
                )
                .map(|_| ()),
                why,
            );
        }
        conn.execute(
            "INSERT INTO silver.sourcing (line_no, part, store, authorized, listing_checked, \
             in_cart, ordered, arrived, record) VALUES (6, 'RP2350', 'Micro Center', 'a list', \
             false, false, false, true, 'lab/sourcing.toml')",
            [],
        )
        .unwrap();
        // A store with no shop, bought used.
        conn.execute(
            "INSERT INTO silver.sourcing (line_no, commodity, store, used, listing_checked, \
             in_cart, ordered, arrived, record) VALUES (7, 'wire', 'Goodwill', true, false, false, \
             false, true, 'lab/sourcing.toml')",
            [],
        )
        .unwrap();
        // A day given, a stage with none, and a day after both.
        conn.execute(
            &sourcing(
                "'RP2350', NULL, NULL, 'lcsc', NULL, 'C1', true, false, true, true, \
                 '2026-10-02', NULL, NULL, '2026-10-09', 'm1'",
            ),
            [],
        )
        .unwrap();
    }
}
