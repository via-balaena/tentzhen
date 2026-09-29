//! Loads the catalogue's records into DuckDB: bronze (the files as read), silver (typed rows the
//! database itself constrains) and gold (views). The schema is `schema.sql`; this only fills it.

use duckdb::{Connection, params};
use tentzhen_records::Catalogue;

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
    tx.commit()
}

#[cfg(test)]
mod tests {
    use super::*;
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
            "SELECT count(*) FROM (SELECT record FROM silver.part UNION ALL SELECT record FROM silver.build_version) s \
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
    }
}
