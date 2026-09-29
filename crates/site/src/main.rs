//! `cargo run -p tentzhen-site` from the repo root rewrites `site/builds/`.

use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let root = Path::new(".");
    let result = tentzhen_records::Catalogue::load(root)
        .and_then(|cat| tentzhen_site::write_site(&cat, root));
    match result {
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
