//! Loads the catalogue's records and the lab's limits into DuckDB: bronze (the files as read),
//! silver (typed rows the database itself constrains) and gold (views). The schema is
//! `schema.sql`; this only fills it.

use duckdb::{Connection, params};
use tentzhen_lab::{LIMITS, Limits, Value};
use tentzhen_records::Catalogue;

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
            tx.execute(
                "INSERT INTO silver.claim VALUES (?, ?, ?, ?)",
                params![key.0, key.1, i, c.says],
            )?;
            if let Some(sim) = &c.sim {
                tx.execute(
                    "INSERT INTO silver.referent VALUES (?, ?, ?, 'sim', ?, ?)",
                    params![key.0, key.1, i, sim.grade.as_str(), sim.by],
                )?;
            }
            if let Some(irl) = &c.irl {
                tx.execute(
                    "INSERT INTO silver.referent VALUES (?, ?, ?, 'irl', 'measured', ?)",
                    params![key.0, key.1, i, irl.record],
                )?;
            }
        }
    }
    if let Some(limits) = &cat.limits {
        load_limits(&tx, limits)?;
    }
    tx.commit()
}

fn load_limits(tx: &duckdb::Transaction, limits: &Limits) -> duckdb::Result<()> {
    for f in limits.facts() {
        tx.execute(
            "INSERT INTO silver.lab_fact VALUES (?, ?, ?, ?, ?)",
            params![f.id.as_str(), f.says, f.trusted, f.record, LIMITS],
        )?;
    }
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
        for f in &e.basis.rests_on {
            tx.execute(
                "INSERT INTO silver.lab_limit_rests_on VALUES (?, ?)",
                params![e.path, f.as_str()],
            )?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use tentzhen_records::Source;

    fn src(path: &str, text: &str) -> Source {
        Source {
            path: path.into(),
            text: text.into(),
        }
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
            "SELECT unknown FROM gold.grade_coverage WHERE build = 'debug-probe' AND version = 1",
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
             UNION ALL SELECT record FROM silver.lab_fact) s \
             LEFT JOIN bronze.record r ON r.path = s.record WHERE r.path IS NULL",
        );
        assert_eq!(orphans, 0);
        let hashed = one(
            &conn,
            "SELECT count(*) FROM bronze.record_hash WHERE regexp_full_match(sha256, '[0-9a-f]{64}')",
        );
        assert_eq!(hashed, one(&conn, "SELECT count(*) FROM bronze.record"));
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

    /// Each limit's weakest grade and the facts behind it, as gold.limit_grades gives them.
    fn grade(conn: &Connection, path: &str) -> (Option<String>, Option<String>) {
        conn.query_row(
            "SELECT weakest_grade, rests_on FROM gold.limit_grades WHERE path = ?",
            [path],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap()
    }

    fn some(grade: &str, facts: &str) -> (Option<String>, Option<String>) {
        (Some(grade.into()), Some(facts.into()))
    }

    #[test]
    fn every_limit_gets_the_weakest_grade_of_its_facts() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let conn = warehouse(&Catalogue::load(&root).unwrap());
        assert_eq!(
            one(&conn, "SELECT count(*) FROM silver.lab_limit"),
            one(&conn, "SELECT count(*) FROM gold.limit_grades"),
        );
        assert_eq!(
            grade(&conn, "supply.upstream.volts"),
            some("unknown", "bench_setting_error, dps_input"),
            "the bench supply's setting error is not measured",
        );
        assert_eq!(
            grade(&conn, "supply.dps.current_limit_amps"),
            some("trusted", "opendps_current_limit"),
        );
        assert_eq!(grade(&conn, "pico_3v3.max_volts"), (None, None));
    }

    #[test]
    fn a_grade_flows_through_every_copy_and_rule() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut cat = Catalogue::load(&root).unwrap();
        let text = fs::read_to_string(root.join(LIMITS)).unwrap().replace(
            "basis.max_amps = { policy = \"CLAUDE.md, \\\"Default ceilings: 3.6 V and 200 mA\\\"\" }",
            "basis.max_amps = { policy = \"x\", rests_on = [\"bench_setting_error\"] }",
        );
        cat.limits = Some(Limits::parse(&text).unwrap());
        let conn = warehouse(&cat);
        assert_eq!(
            grade(&conn, "pico_3v3.max_amps"),
            some("unknown", "bench_setting_error")
        );
        // A copy, a copy of a copy that rests on a trusted fact too, and a rule's input.
        assert_eq!(
            grade(&conn, "supply.max_amps"),
            some("unknown", "bench_setting_error")
        );
        assert_eq!(
            grade(&conn, "supply.dps.current_limit_amps"),
            some("unknown", "bench_setting_error, opendps_current_limit"),
        );
        assert_eq!(
            grade(&conn, "supply.fuse.rating_amps"),
            some("unknown", "bench_setting_error")
        );
        assert_eq!(grade(&conn, "pico_3v3.max_volts"), (None, None));
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

    #[test]
    fn grades_derive_from_referents() {
        let claims = "[[claim]]\nsays = \"a\"\nsim = { grade = \"tested\", by = \"t\" }\nirl = { record = \"lab-1\" }\n\n[[claim]]\nsays = \"b\"\n";
        let conn = warehouse(&Catalogue::from_sources(&[], &[build("probe", claims)]).unwrap());
        let (sim, irl): (String, String) = conn
            .query_row(
                "SELECT sim_grade, irl_grade FROM gold.claim_grades WHERE says = 'a'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!((sim.as_str(), irl.as_str()), ("tested", "measured"));
        assert_eq!(one(&conn, "SELECT measured FROM gold.grade_coverage"), 1);
        assert_eq!(one(&conn, "SELECT unknown FROM gold.grade_coverage"), 1);
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
            &Catalogue::from_sources(&[src("parts/rp2350.toml", PART)], &[build("probe", PROBE)])
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
        let irl_tested = "INSERT INTO silver.claim VALUES ('probe', 1, 1, 'x'); \
                          INSERT INTO silver.referent VALUES ('probe', 1, 1, 'irl', 'tested', 'x')";
        refused(conn.execute_batch(irl_tested), "the bench can only measure");
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
