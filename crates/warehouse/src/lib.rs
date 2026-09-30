//! Loads the catalogue's records and the lab's limits into DuckDB: bronze (the files as read),
//! silver (typed rows the database itself constrains) and gold (views). The schema is
//! `schema.sql`; this only fills it.

use duckdb::{Connection, params};
use tentzhen_lab::{LIMITS, Limits, Value};
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
    for p in cat.parts.values() {
        for (i, c) in (1u32..).zip(&p.claim) {
            let claim = format!("{}#{}", p.part, c.id);
            tx.execute(
                "INSERT INTO silver.claim VALUES (?, ?, NULL, NULL, ?, ?, ?)",
                params![claim, p.part, i, c.id, c.says],
            )?;
            load_referents(&tx, &claim, c)?;
        }
    }
    // Every version first: lines, uses and claims point at them.
    for b in cat.builds.values().flatten() {
        tx.execute(
            "INSERT INTO silver.build_version VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![
                b.build,
                b.version,
                b.status.as_str(),
                b.does,
                b.changes,
                b.drawing,
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
            load_referents(&tx, &claim, c)?;
        }
    }
    if let Some(limits) = &cat.limits {
        load_limits(&tx, limits)?;
    }
    tx.commit()
}

/// A claim's referents and values, under its citation. A trusted referent is a key into the
/// trusted base; the others are kept as named.
fn load_referents(tx: &duckdb::Transaction, claim: &str, c: &Claim) -> duckdb::Result<()> {
    for (kind, named) in c.referents() {
        let (evidence, trusted) = match kind {
            "trusted" => (None, Some(named)),
            _ => (Some(named), None),
        };
        tx.execute(
            "INSERT INTO silver.referent VALUES (?, ?, ?, ?)",
            params![claim, kind, evidence, trusted],
        )?;
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::Path;
    use tentzhen_records::{Source, TRUSTED_BASE};

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
            "SELECT unknown FROM gold.grade_coverage WHERE subject = 'debug-probe/v1'",
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
             UNION ALL SELECT record FROM silver.trusted_entry) s \
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
            "basis.max_amps = { policy = \"CLAUDE.md, \\\"Default ceilings: 3.6 V and 200 mA\\\"\" }",
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
        let claims = "[[claim]]\nid = \"a\"\nsays = \"x\"\ntest = \"t\"\nrecord = \"lab-1\"\n\n\
                      [[claim]]\nid = \"b\"\nsays = \"x\"\nproof = \"p\"\ncheck = \"c\"\ntest = \"t\"\n\n\
                      [[claim]]\nid = \"c\"\nsays = \"x\"\ntrusted = \"entry\"\nrecord = \"lab-2\"\n\n\
                      [[claim]]\nid = \"d\"\nsays = \"x\"\ntrusted = \"entry\"\n\n\
                      [[claim]]\nid = \"e\"\nsays = \"x\"\n\n\
                      [[claim]]\nid = \"f\"\nsays = \"x\"\ncheck = \"c\"\ntest = \"t\"\n";
        let conn =
            warehouse(&Catalogue::from_sources(&base(), &[], &[build("probe", claims)]).unwrap());
        let got = grades(&conn);
        assert_eq!(got["a"], g("tested", Some("t"), "measured", Some("lab-1")));
        assert_eq!(
            got["b"],
            g("proven", Some("p"), "unknown", None),
            "the strongest shows"
        );
        assert_eq!(
            got["c"],
            g("unknown", None, "measured", Some("lab-2")),
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
                      INSERT INTO silver.referent VALUES ('k5', 'tested', 't', NULL)";
        refused(
            conn.execute_batch(graded),
            "a referent is a kind of evidence, not a grade",
        );
        for (values, why) in [
            (
                "'trusted', NULL, 'nope'",
                "a trusted referent names an entry in the trusted base",
            ),
            ("'trusted', 'entry', NULL", "a trusted referent is a key"),
            (
                "'trusted', 'entry', 'entry'",
                "a trusted referent is a key, not a name too",
            ),
            (
                "'test', NULL, 'entry'",
                "only a trusted referent names an entry",
            ),
            ("'test', NULL, NULL", "a referent names something"),
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
            "INSERT INTO silver.referent VALUES ('k5', 'trusted', NULL, 'entry'); \
             INSERT INTO silver.referent VALUES ('k5', 'test', 't', NULL); \
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
    }
}
