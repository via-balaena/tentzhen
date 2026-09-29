//! The data catalogue: every table and view in the warehouse as DuckDB describes it, with the
//! comments `schema.sql` gives them and the crates that read them, written as Markdown.

use duckdb::Connection;
use std::collections::BTreeMap;
use std::fmt::Write;
use std::fs;
use std::path::Path;

/// Where the catalogue is written, relative to the repo root.
pub const PATH: &str = "docs/data-catalogue.md";

/// A table or view.
pub struct Relation {
    pub name: String,
    pub view: bool,
    pub comment: Option<String>,
    /// For a view, the relations its SQL names.
    pub reads: Vec<String>,
    /// The crates outside the warehouse whose source names it.
    pub read_by: Vec<String>,
}

/// Every table and view, bronze then silver then gold, each in the order `schema.sql` creates
/// them. `sources` is each other crate's name and source text, searched for relation names.
pub fn relations(conn: &Connection, sources: &[(String, String)]) -> duckdb::Result<Vec<Relation>> {
    let mut stmt = conn.prepare(
        "SELECT schema_name || '.' || name, view, comment,
                list_distinct(regexp_extract_all(coalesce(sql, ''), '\\b(bronze|silver|gold)\\.[a-z_]+'))
         FROM (SELECT schema_name, table_name AS name, false AS view, comment, NULL AS sql,
                      table_oid AS oid
               FROM duckdb_tables() WHERE NOT internal
               UNION ALL
               SELECT schema_name, view_name, true, comment, sql, view_oid
               FROM duckdb_views() WHERE NOT internal)
         WHERE schema_name IN ('bronze', 'silver', 'gold')
         ORDER BY list_position(['bronze', 'silver', 'gold'], schema_name), oid",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, bool>(1)?,
            r.get::<_, Option<String>>(2)?,
            r.get::<_, duckdb::types::Value>(3)?,
        ))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (name, view, comment, reads) = row?;
        let mut reads: Vec<String> = match reads {
            duckdb::types::Value::List(items) => items
                .into_iter()
                .filter_map(|v| match v {
                    duckdb::types::Value::Text(t) => Some(t),
                    _ => None,
                })
                .filter(|t| *t != name)
                .collect(),
            _ => Vec::new(),
        };
        reads.sort();
        let read_by = sources
            .iter()
            .filter(|(_, text)| names(text, &name))
            .map(|(krate, _)| krate.clone())
            .collect();
        out.push(Relation {
            name,
            view,
            comment,
            reads,
            read_by,
        });
    }
    Ok(out)
}

/// Whether `text` names `name` whole: `gold.page` is not named by `gold.page_line`.
fn names(text: &str, name: &str) -> bool {
    text.match_indices(name).any(|(i, _)| {
        let after = text[i + name.len()..].chars().next();
        let before = text[..i].chars().next_back();
        !after.is_some_and(|c| c.is_alphanumeric() || c == '_')
            && !before.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.')
    })
}

/// Every crate but the warehouse, with its source: the warehouse's own tests name everything.
pub fn sources(root: &Path) -> Result<Vec<(String, String)>, String> {
    let mut out = Vec::new();
    let crates = root.join("crates");
    let mut dirs: Vec<_> = fs::read_dir(&crates)
        .map_err(|e| format!("{}: {e}", crates.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir() && !p.ends_with("warehouse"))
        .collect();
    dirs.sort();
    for dir in dirs {
        let mut text = String::new();
        let mut files = Vec::new();
        rust_files(&dir.join("src"), &mut files)?;
        files.sort();
        for f in files {
            text.push_str(&fs::read_to_string(&f).map_err(|e| format!("{}: {e}", f.display()))?);
        }
        let name = dir
            .file_name()
            .map(|n| format!("crates/{}", n.to_string_lossy()))
            .unwrap_or_default();
        out.push((name, text));
    }
    Ok(out)
}

/// Every `.rs` file under `dir`, at any depth.
fn rust_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.is_dir() {
            rust_files(&path, out)?;
        } else if path.extension().is_some_and(|x| x == "rs") {
            out.push(path);
        }
    }
    Ok(())
}

/// Text for a Markdown table cell: a `|` would end the cell.
fn cell(text: &str) -> String {
    text.replace('|', "\\|")
}

fn id(name: &str) -> String {
    name.replace(['.', '/'], "_")
}

/// The catalogue as Markdown.
pub fn render(conn: &Connection, relations: &[Relation]) -> duckdb::Result<String> {
    let mut md = String::new();
    let w = &mut md;
    let _ = writeln!(
        w,
        "# Data catalogue\n\n\
         Every table and view in the warehouse, as DuckDB describes them, with the comments\n\
         `crates/warehouse/schema.sql` gives them. Generated by\n\
         `cargo run -p tentzhen-warehouse -- catalogue`; the test `the_catalogue_is_current` fails\n\
         when this file differs from what that would write. Change the schema, not this file.\n\n\
         ## Lineage\n\n\
         `crates/warehouse` loads bronze and silver from the records. An arrow runs from what is\n\
         read to what reads it: a view's SQL, or a crate's source.\n"
    );
    let _ = writeln!(w, "```mermaid\nflowchart LR");
    for layer in ["bronze", "silver", "gold"] {
        let _ = writeln!(w, "  subgraph {layer}");
        for r in relations.iter().filter(|r| r.name.starts_with(layer)) {
            let _ = writeln!(w, "    {}[\"{}\"]", id(&r.name), r.name);
        }
        let _ = writeln!(w, "  end");
    }
    let _ = writeln!(w, "  bronze -. crates/warehouse .-> silver");
    let mut readers: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for r in relations {
        for from in &r.reads {
            let _ = writeln!(w, "  {} --> {}", id(from), id(&r.name));
        }
        for krate in &r.read_by {
            readers.entry(krate).or_default().push(&r.name);
        }
    }
    for (krate, read) in &readers {
        let _ = writeln!(w, "  {}([\"{krate}\"])", id(krate));
        for name in read {
            let _ = writeln!(w, "  {} --> {}", id(name), id(krate));
        }
    }
    let _ = writeln!(w, "```");

    let mut read_by_views: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for r in relations {
        for from in &r.reads {
            read_by_views.entry(from).or_default().push(&r.name);
        }
    }
    let mut columns = conn.prepare(
        "SELECT column_name, data_type, is_nullable, coalesce(comment, '')
         FROM duckdb_columns() WHERE schema_name || '.' || table_name = ? ORDER BY column_index",
    )?;
    let mut constraints = conn.prepare(
        "SELECT constraint_text FROM duckdb_constraints()
         WHERE schema_name || '.' || table_name = ? AND constraint_type <> 'NOT NULL'
         GROUP BY constraint_text ORDER BY min(constraint_index), constraint_text",
    )?;
    for layer in ["bronze", "silver", "gold"] {
        let _ = writeln!(w, "\n## {layer}");
        for r in relations.iter().filter(|r| r.name.starts_with(layer)) {
            let kind = if r.view { "view" } else { "table" };
            let _ = writeln!(w, "\n### `{}` ({kind})\n", r.name);
            if let Some(c) = &r.comment {
                let _ = writeln!(w, "{c}\n");
            }
            if r.view {
                let _ = writeln!(w, "| column | type |\n|---|---|");
            } else {
                let _ = writeln!(w, "| column | type | null | about |\n|---|---|---|---|");
            }
            let cols = columns.query_map([&r.name], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, bool>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })?;
            for col in cols {
                let (name, ty, nullable, about) = col?;
                if r.view {
                    let _ = writeln!(w, "| `{name}` | {} |", cell(&ty));
                } else {
                    let null = if nullable { "yes" } else { "no" };
                    let _ = writeln!(
                        w,
                        "| `{name}` | {} | {null} | {} |",
                        cell(&ty),
                        cell(&about)
                    );
                }
            }
            let checks: Vec<String> = constraints
                .query_map([&r.name], |row| row.get(0))?
                .collect::<Result<_, _>>()?;
            if !checks.is_empty() {
                let _ = writeln!(w, "\nConstraints:\n");
                for c in checks {
                    let _ = writeln!(w, "- `{c}`");
                }
            }
            let list = |names: &[&str]| {
                names
                    .iter()
                    .map(|n| format!("`{n}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            if !r.reads.is_empty() {
                let reads: Vec<&str> = r.reads.iter().map(String::as_str).collect();
                let _ = writeln!(w, "\nReads: {}.", list(&reads));
            }
            let mut by: Vec<&str> = read_by_views
                .get(r.name.as_str())
                .cloned()
                .unwrap_or_default();
            by.extend(r.read_by.iter().map(String::as_str));
            if !by.is_empty() {
                let _ = writeln!(w, "\nRead by: {}.", list(&by));
            } else if layer == "gold" {
                let _ = writeln!(w, "\nRead by: no crate yet.");
            }
        }
    }
    Ok(md)
}

/// The catalogue of `schema.sql` alone. No records are loaded, so adding or changing a record never
/// changes it.
pub fn generate(root: &Path) -> Result<String, String> {
    let conn = Connection::open_in_memory().map_err(|e| e.to_string())?;
    conn.execute_batch(crate::SCHEMA)
        .map_err(|e| e.to_string())?;
    let rels = relations(&conn, &sources(root)?).map_err(|e| e.to_string())?;
    render(&conn, &rels).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::{cell, names, rust_files};
    use std::path::Path;

    #[test]
    fn a_pipe_stays_inside_its_cell() {
        assert_eq!(cell("a | b"), "a \\| b");
    }

    #[test]
    fn source_is_read_at_any_depth() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/catalogue-test/src");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("pages/deep")).unwrap();
        for f in [
            "lib.rs",
            "pages/mod.rs",
            "pages/deep/x.rs",
            "pages/notes.md",
        ] {
            std::fs::write(dir.join(f), "").unwrap();
        }
        let mut found = Vec::new();
        rust_files(&dir, &mut found).unwrap();
        let mut found: Vec<String> = found
            .iter()
            .map(|p| {
                p.strip_prefix(&dir)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect();
        found.sort();
        assert_eq!(found, ["lib.rs", "pages/deep/x.rs", "pages/mod.rs"]);
    }

    #[test]
    fn a_name_is_matched_whole() {
        assert!(names("SELECT * FROM gold.page WHERE", "gold.page"));
        assert!(names("\"gold.page\"", "gold.page"));
        assert!(!names("FROM gold.page_line", "gold.page"));
        assert!(!names("FROM xgold.page", "gold.page"));
        assert!(!names("FROM a.gold.page", "gold.page"));
    }
}
