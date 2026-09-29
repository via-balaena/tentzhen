# tentzhen-warehouse

The catalogue's records, loaded into DuckDB. `schema.sql` is the contract; read it first.

```
bronze   the record files exactly as read, and their sha256 (lineage)
silver   typed rows; the database enforces keys, foreign keys and checks
gold     views: latest, claim_grades, grade_coverage, bom_exploded, where_used
```

The records in `parts/` and `builds/` are the system of record. The warehouse is rebuilt from them
every time, so it can be deleted whenever you like:

```fish
cargo run -p tentzhen-warehouse
```

That writes `target/warehouse/tentzhen.duckdb` and prints each table's row count. Query it with
any DuckDB client, for example the full parts list of a build through every build it uses:

```sql
SELECT * FROM gold.bom_exploded WHERE build = 'debug-probe' AND version = 1;
```

Data we may not share stays out: LCSC and JLCPCB data is joined in locally, per user, never
loaded here (`docs/sourcing.md`).
