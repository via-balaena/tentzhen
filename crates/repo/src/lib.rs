//! Checks on the repo itself: the rules in docs/architecture.md that span crates. There is no
//! library here, only tests.

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use toml::{Table, Value};

    fn root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn manifest(path: &Path) -> Table {
        fs::read_to_string(path).unwrap().parse().unwrap()
    }

    /// The workspace's manifest and every crate's, by path.
    fn manifests() -> Vec<(String, Table)> {
        let mut dirs: Vec<PathBuf> = fs::read_dir(root().join("crates"))
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.join("Cargo.toml").is_file())
            .collect();
        dirs.sort();
        let mut out = vec![(
            "Cargo.toml".to_string(),
            manifest(&root().join("Cargo.toml")),
        )];
        for d in dirs {
            let name = format!(
                "crates/{}/Cargo.toml",
                d.file_name().unwrap().to_string_lossy()
            );
            out.push((name, manifest(&d.join("Cargo.toml"))));
        }
        out
    }

    /// Every dependency a manifest declares, of every kind, with its spec.
    fn dependencies(m: &Table) -> Vec<(String, Value)> {
        let mut tables: Vec<&Table> = Vec::new();
        for kind in ["dependencies", "dev-dependencies", "build-dependencies"] {
            tables.extend(m.get(kind).and_then(Value::as_table));
            if let Some(targets) = m.get("target").and_then(Value::as_table) {
                for t in targets.values().filter_map(Value::as_table) {
                    tables.extend(t.get(kind).and_then(Value::as_table));
                }
            }
        }
        if let Some(ws) = m.get("workspace").and_then(Value::as_table) {
            tables.extend(ws.get("dependencies").and_then(Value::as_table));
        }
        tables
            .into_iter()
            .flat_map(|t| t.iter().map(|(k, v)| (k.clone(), v.clone())))
            .collect()
    }

    /// The names a crate lists under [dependencies], sorted.
    fn runtime_dependencies(krate: &str) -> Vec<String> {
        let m = manifest(&root().join("crates").join(krate).join("Cargo.toml"));
        let mut names: Vec<String> = m
            .get("dependencies")
            .and_then(Value::as_table)
            .map(|t| t.keys().cloned().collect())
            .unwrap_or_default();
        names.sort();
        names
    }

    /// docs/architecture.md names the tests that hold its rules, and the files it describes. Up to
    /// its proposal, which is about what does not exist yet, each must exist: a renamed test or a
    /// moved file fails here, not silently in the doc.
    #[test]
    fn the_architecture_doc_names_real_tests_and_files() {
        let doc = fs::read_to_string(root().join("docs/architecture.md")).unwrap();
        let (now, _) = doc
            .split_once("\n## Proposal")
            .expect("docs/architecture.md has a Proposal section");
        // Fenced blocks (the diagram) are not prose, and their ``` would pair with the wrong `.
        let mut fenced = false;
        let now: String = now
            .lines()
            .filter(|line| {
                if line.starts_with("```") {
                    fenced = !fenced;
                    return false;
                }
                !fenced
            })
            .collect::<Vec<_>>()
            .join("\n");
        let mut source = String::new();
        let mut stack = vec![root().join("crates")];
        while let Some(dir) = stack.pop() {
            for entry in fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().is_some_and(|x| x == "rs") {
                    source.push_str(&fs::read_to_string(&path).unwrap());
                }
            }
        }
        let (mut tests, mut paths) = (0, 0);
        for (i, token) in now.split('`').enumerate() {
            if i % 2 == 0 {
                continue;
            }
            let is_name = token.matches('_').count() >= 2
                && token
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
            if is_name {
                tests += 1;
                assert!(
                    source.contains(&format!("fn {token}()")),
                    "docs/architecture.md names the test {token}, which does not exist"
                );
            } else if token.contains('<') {
                // A pattern such as `<build>/v<n>#<id>`, not a file.
            } else if token.contains('/') || token.ends_with(".toml") || token.ends_with(".md") {
                paths += 1;
                let path = root().join(token);
                let in_docs = root().join("docs").join(token);
                assert!(
                    path.exists() || in_docs.exists(),
                    "docs/architecture.md names {token}, which does not exist"
                );
            }
        }
        assert!(
            tests >= 10 && paths >= 10,
            "found {tests} tests and {paths} paths"
        );
    }

    /// The Quality Gate's step "The lab log only grows": its script, less the fetch, run by `bash -e`
    /// as GitHub runs the gate's steps (each step's log says `shell: /usr/bin/bash -e {0}`), against
    /// a scratch repo whose main holds a day of the log. It passes what adds to the log and fails
    /// what changes it.
    #[test]
    fn the_lab_log_step_passes_only_an_append() {
        let workflow =
            fs::read_to_string(root().join(".github/workflows/quality-gate.yml")).unwrap();
        let (_, step) = workflow
            .split_once("- name: The lab log only grows")
            .expect("the Quality Gate has the step");
        let indent = " ".repeat(10);
        let script: Vec<&str> = step
            .lines()
            .skip_while(|l| l.trim() != "run: |")
            .skip(1)
            .take_while(|l| l.starts_with(&indent) || l.trim().is_empty())
            .map(|l| l.get(indent.len()..).unwrap_or_default())
            .filter(|l| !l.starts_with("git fetch"))
            .collect();
        assert!(script.len() > 3, "{script:?}");
        let script = script.join("\n");

        let dir = std::env::temp_dir().join(format!("tentzhen-step-{}", std::process::id()));
        if dir.exists() {
            fs::remove_dir_all(&dir).unwrap();
        }
        fs::create_dir_all(dir.join("lab/log")).unwrap();
        let git = |args: &[&str]| {
            let ok = Command::new("git")
                .args(["-c", "user.name=t", "-c", "user.email=t@t"])
                .args(["-c", "commit.gpgsign=false"])
                .args(args)
                .current_dir(&dir)
                .status()
                .unwrap()
                .success();
            assert!(ok, "git {args:?}");
        };
        git(&["init", "-q", "-b", "main"]);
        fs::write(dir.join("lab/limits.toml"), "limits").unwrap();
        git(&["add", "-A"]);
        git(&["commit", "-q", "--no-verify", "-m", "main without a log"]);
        let passes = |change: &dyn Fn()| {
            change();
            let ok = Command::new("bash")
                .args(["-e", "-c", &script])
                .env("BASE", "main")
                .current_dir(&dir)
                .output()
                .unwrap()
                .status
                .success();
            git(&["checkout", "-q", "--", "."]);
            git(&["clean", "-q", "-f", "-d"]);
            ok
        };
        let write = |path: &Path, text: &str| {
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        };
        let day = dir.join("lab/log/2026-09-30.jsonl");
        assert!(passes(&|| write(&day, "one\n")), "main has no log yet");
        write(&day, "one\ntwo\n");
        git(&["add", "-A"]);
        git(&[
            "commit",
            "-q",
            "--no-verify",
            "-m",
            "main with a day of the log",
        ]);
        assert!(passes(&|| {}), "nothing changed");
        assert!(
            passes(&|| write(&day, "one\ntwo\nthree\n")),
            "an entry added"
        );
        let next = dir.join("lab/log/2026-10-01.jsonl");
        assert!(passes(&|| write(&next, "a\n")), "a new day");
        assert!(!passes(&|| write(&day, "one!\ntwo\n")), "an entry changed");
        assert!(!passes(&|| write(&day, "one\n")), "an entry cut");
        assert!(!passes(&|| write(&day, "one\ntwo")), "a newline cut");
        assert!(!passes(&|| fs::remove_file(&day).unwrap()), "a day removed");
        fs::remove_dir_all(&dir).unwrap();
    }

    /// Whether a dependency names exactly one version. None when its version is kept elsewhere: a
    /// path in this workspace, or [workspace.dependencies], which is checked in its own right.
    fn pinned(spec: &Value) -> Option<bool> {
        let exact = |v: Option<&str>| v.is_some_and(|v| v.starts_with('='));
        match spec {
            Value::String(v) => Some(exact(Some(v))),
            Value::Table(t) if t.contains_key("path") || t.contains_key("workspace") => None,
            Value::Table(t) => Some(exact(t.get("version").and_then(Value::as_str))),
            _ => Some(false),
        }
    }

    #[test]
    fn what_counts_as_pinned() {
        let spec = |s: &str| -> Value { s.parse::<Table>().unwrap().remove("d").unwrap() };
        assert_eq!(pinned(&spec("d = \"=1.2.3\"")), Some(true));
        assert_eq!(pinned(&spec("d = { version = \"=1.2.3\" }")), Some(true));
        assert_eq!(pinned(&spec("d = \"1.2.3\"")), Some(false));
        assert_eq!(pinned(&spec("d = { version = \"^1\" }")), Some(false));
        assert_eq!(pinned(&spec("d = { git = \"https://x\" }")), Some(false));
        assert_eq!(pinned(&spec("d = { path = \"../d\" }")), None);
        assert_eq!(pinned(&spec("d = { workspace = true }")), None);
    }

    /// CLAUDE.md: "Pin every dependency version".
    #[test]
    fn every_dependency_is_pinned() {
        let found = manifests();
        assert!(found.len() > 3, "found {} manifests", found.len());
        for (at, m) in found {
            for (name, spec) in dependencies(&m) {
                assert!(
                    pinned(&spec) != Some(false),
                    "{at}: {name} is not pinned to one version: {spec}"
                );
            }
        }
    }

    /// The Pico enforcer's build script is to read lab/limits.toml through crates/lab, so the
    /// safety path must not pull in the database or the catalogue.
    #[test]
    fn the_lab_crate_needs_only_serde_and_toml() {
        assert_eq!(runtime_dependencies("lab"), ["serde", "toml"]);
    }

    /// Reading the records is parsing and checking; the database comes after, in the warehouse. The
    /// lab log is JSON, chained by sha256.
    #[test]
    fn parsing_records_needs_no_database() {
        assert!(runtime_dependencies("warehouse").contains(&"duckdb".to_string()));
        assert_eq!(
            runtime_dependencies("records"),
            ["serde", "serde_json", "sha2", "tentzhen-lab", "toml"]
        );
    }
}
