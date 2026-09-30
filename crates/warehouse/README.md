# tentzhen-warehouse

The catalogue's records, loaded into DuckDB. `schema.sql` is the contract; read it first, or
[`docs/data-catalogue.md`](../../docs/data-catalogue.md), which is generated from it: every table
and view, its columns and constraints, what it reads and what reads it.

```
bronze   the record files exactly as read, and their sha256 (lineage)
silver   typed rows; the database enforces keys, foreign keys and checks
gold     views: latest, claim_grades, grade_coverage, bom_exploded, where_used,
         limit_grades
```

The records in `parts/`, `builds/` and `lab/` are the system of record. The warehouse is rebuilt
from them every time, so it can be deleted whenever you like:

```fish
cargo run -p tentzhen-warehouse
```

That writes `target/warehouse/tentzhen.duckdb` and prints each table's row count. Query it with
any DuckDB client, for example the full parts list of a build through every build it uses:

```sql
SELECT * FROM gold.bom_exploded WHERE build = 'debug-probe' AND version = 1;
```

`lab/limits.toml` is checked by `crates/lab`, then loaded like the records: each limit with its
basis (a person's policy, a copy of another limit, or a rule in `crates/lab`) and the claims it
rests on. Which limits rest on a claim that is neither measured nor trusted:

```sql
SELECT path, weakest_grade, rests_on FROM gold.limit_grades WHERE weakest_grade = 'unknown';
```

The lab-target list (`lab/targets.toml`), the boards an agent may flash without asking, is checked
by `crates/records` and loaded into `silver.lab_target`, one row per board.

The lab log (`lab/log/`) is checked by `crates/records`, its chain included, then loaded: each
hardware action in `silver.lab_log_entry`, and what it was asked and what happened in
`silver.lab_log_value`.

A measurement record (`lab/records/<id>.toml`) is checked by `crates/records` against the
catalogue and the log, then loaded into `silver.measurement` and its `_device`, `_tool` and
`_value` tables. A claim's `record` referent is a key into `silver.measurement`, and the claims
measured on the bench are:

```sql
SELECT claim, bench_evidence FROM gold.claim_grades WHERE bench_grade = 'measured';
```

Data we may not share stays out: LCSC and JLCPCB data is joined in locally, per user, never
loaded here (`docs/sourcing.md`).
