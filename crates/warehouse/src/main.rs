//! `cargo run -p tentzhen-warehouse` from the repo root rebuilds target/warehouse/tentzhen.duckdb
//! from the records and prints what it holds. Query it with any DuckDB client.

use duckdb::Connection;
use std::fs;
use std::path::Path;
use std::process::ExitCode;
use tentzhen_records::Catalogue;

fn build() -> Result<String, String> {
    let root = Path::new(".");
    let out = root.join("target/warehouse/tentzhen.duckdb");
    fs::create_dir_all(out.parent().unwrap_or(root)).map_err(|e| e.to_string())?;
    for stale in [out.clone(), out.with_extension("duckdb.wal")] {
        if stale.exists() {
            fs::remove_file(&stale).map_err(|e| format!("{}: {e}", stale.display()))?;
        }
    }
    let cat = Catalogue::load(root)?;
    let mut conn = Connection::open(&out).map_err(|e| e.to_string())?;
    tentzhen_warehouse::load(&mut conn, &cat).map_err(|e| e.to_string())?;

    let mut report = format!("{}\n", out.display());
    let tables: Vec<String> = {
        let mut stmt = conn
            .prepare(
                "SELECT schema_name || '.' || table_name FROM duckdb_tables() \
                 ORDER BY schema_name, table_name",
            )
            .map_err(|e| e.to_string())?;
        stmt.query_map([], |r| r.get(0))
            .map_err(|e| e.to_string())?
            .collect::<Result<_, _>>()
            .map_err(|e| e.to_string())?
    };
    for table in tables {
        let n: i64 = conn
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        report.push_str(&format!("  {table:<22} {n:>4} rows\n"));
    }
    Ok(report)
}

fn main() -> ExitCode {
    match build() {
        Ok(report) => {
            print!("{report}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
