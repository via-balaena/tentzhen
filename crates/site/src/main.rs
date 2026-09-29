//! `cargo run -p tentzhen-site` from the repo root rebuilds `site/builds/`: records into an
//! in-memory warehouse, pages out of its gold views.

use duckdb::Connection;
use std::path::Path;
use std::process::ExitCode;

fn build(root: &Path) -> Result<usize, String> {
    let cat = tentzhen_records::Catalogue::load(root)?;
    let mut conn = Connection::open_in_memory().map_err(|e| e.to_string())?;
    tentzhen_warehouse::load(&mut conn, &cat).map_err(|e| e.to_string())?;
    tentzhen_site::write_site(&conn, root)
}

fn main() -> ExitCode {
    match build(Path::new(".")) {
        Ok(n) => {
            println!("wrote {n} files under site/builds/");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
