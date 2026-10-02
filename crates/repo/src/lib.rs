//! Checks on the repo itself: the rules in docs/architecture.md that span crates. There is no
//! library here, only tests.

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
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

    /// Every crate in the workspace, as its manifest's `members` names them: a directory, or
    /// `<dir>/*` for each directory under it that has a manifest. Relative to the root.
    fn members() -> Vec<String> {
        let ws = manifest(&root().join("Cargo.toml"));
        let mut out = Vec::new();
        for m in ws["workspace"]["members"].as_array().unwrap() {
            let m = m.as_str().unwrap();
            let Some(parent) = m.strip_suffix("/*") else {
                out.push(m.to_string());
                continue;
            };
            for e in fs::read_dir(root().join(parent)).unwrap() {
                let name = e.unwrap().file_name().to_string_lossy().into_owned();
                if root().join(parent).join(&name).join("Cargo.toml").is_file() {
                    out.push(format!("{parent}/{name}"));
                }
            }
        }
        out.sort();
        out
    }

    /// The workspace's manifest and every crate's, by path.
    fn manifests() -> Vec<(String, Table)> {
        let mut out = vec![(
            "Cargo.toml".to_string(),
            manifest(&root().join("Cargo.toml")),
        )];
        for m in members() {
            let path = format!("{m}/Cargo.toml");
            out.push((path.clone(), manifest(&root().join(path))));
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

    /// The names the crate in `dir` lists under [dependencies], sorted.
    fn runtime_dependencies(dir: &str) -> Vec<String> {
        dependencies_of(dir, "dependencies")
    }

    /// The names the crate in `dir` lists under the table `kind`, sorted.
    fn dependencies_of(dir: &str, kind: &str) -> Vec<String> {
        let m = manifest(&root().join(dir).join("Cargo.toml"));
        let mut names: Vec<String> = m
            .get(kind)
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
        let mut stack: Vec<PathBuf> = members().iter().map(|m| root().join(m)).collect();
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

    /// The script of the Quality Gate's step `name`, less its fetch.
    fn gate_step(name: &str) -> String {
        let workflow =
            fs::read_to_string(root().join(".github/workflows/quality-gate.yml")).unwrap();
        let (_, step) = workflow
            .split_once(&format!("- name: {name}\n"))
            .unwrap_or_else(|| panic!("the Quality Gate has no step {name:?}"));
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
        script.join("\n")
    }

    /// A scratch git repo to run a step of the Quality Gate in, as GitHub runs the gate's steps:
    /// by `bash -e` (each step's log says `shell: /usr/bin/bash -e {0}`), with main as the base.
    struct Scratch {
        dir: PathBuf,
        script: String,
    }

    impl Scratch {
        fn new(test: &str, step: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("tentzhen-step-{}-{test}", std::process::id()));
            if dir.exists() {
                fs::remove_dir_all(&dir).unwrap();
            }
            fs::create_dir_all(&dir).unwrap();
            let repo = Scratch {
                dir,
                script: gate_step(step),
            };
            repo.git(&["init", "-q", "-b", "main"]);
            repo.write("lab/limits.toml", "limits");
            repo.commit("main");
            repo
        }

        fn git(&self, args: &[&str]) {
            let ok = Command::new("git")
                .args(["-c", "user.name=t", "-c", "user.email=t@t"])
                .args(["-c", "commit.gpgsign=false"])
                .args(args)
                .current_dir(&self.dir)
                .status()
                .unwrap()
                .success();
            assert!(ok, "git {args:?}");
        }

        fn write(&self, path: &str, text: &str) {
            let path = self.dir.join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        }

        fn remove(&self, path: &str) {
            fs::remove_file(self.dir.join(path)).unwrap();
        }

        /// Commits everything to main.
        fn commit(&self, message: &str) {
            self.git(&["add", "-A"]);
            self.git(&["commit", "-q", "--no-verify", "-m", message]);
        }

        /// Whether the step passes with `change` made to the working tree, which is then undone.
        fn passes(&self, change: impl FnOnce(&Self)) -> bool {
            change(self);
            let ok = Command::new("bash")
                .args(["-e", "-c", &self.script])
                .env("BASE", "main")
                .current_dir(&self.dir)
                .output()
                .unwrap()
                .status
                .success();
            self.git(&["checkout", "-q", "--", "."]);
            self.git(&["clean", "-q", "-f", "-d"]);
            ok
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }

    /// The Quality Gate's step "The lab log only grows", against a main that holds a day of the
    /// log: it passes what adds to the log and fails what changes it.
    #[test]
    fn the_lab_log_step_passes_only_an_append() {
        let repo = Scratch::new("log", "The lab log only grows");
        let day = "lab/log/2026-09-30.jsonl";
        assert!(
            repo.passes(|r| r.write(day, "one\n")),
            "main has no log yet"
        );
        repo.write(day, "one\ntwo\n");
        repo.commit("main with a day of the log");
        assert!(repo.passes(|_| {}), "nothing changed");
        assert!(
            repo.passes(|r| r.write(day, "one\ntwo\nthree\n")),
            "an entry added"
        );
        assert!(
            repo.passes(|r| r.write("lab/log/2026-10-01.jsonl", "a\n")),
            "a new day"
        );
        assert!(
            !repo.passes(|r| r.write(day, "one!\ntwo\n")),
            "an entry changed"
        );
        assert!(!repo.passes(|r| r.write(day, "one\n")), "an entry cut");
        assert!(!repo.passes(|r| r.write(day, "one\ntwo")), "a newline cut");
        assert!(!repo.passes(|r| r.remove(day)), "a day removed");
    }

    /// The Quality Gate's step "A measurement record never changes", against a main that holds a
    /// record: it passes a new record and fails any change to one on main.
    #[test]
    fn the_record_step_passes_only_a_new_record() {
        let repo = Scratch::new("records", "A measurement record never changes");
        let record = "lab/records/2026-09-30-draw.toml";
        assert!(
            repo.passes(|r| r.write(record, "a = 1\n")),
            "main has no records yet"
        );
        repo.write(record, "a = 1\n");
        repo.commit("main with a record");
        assert!(repo.passes(|_| {}), "nothing changed");
        assert!(
            repo.passes(|r| r.write("lab/records/2026-10-01-draw.toml", "a = 2\n")),
            "a new record"
        );
        for (changed, why) in [
            ("a = 2\n", "a value changed"),
            ("a = 1\nb = 2\n", "a line added"),
            ("a = 1", "a newline cut"),
            ("a = 1\r\n", "a line ending changed"),
        ] {
            assert!(!repo.passes(|r| r.write(record, changed)), "{why}");
        }
        assert!(!repo.passes(|r| r.remove(record)), "a record removed");
        assert!(
            !repo.passes(|r| {
                r.remove(record);
                r.write("lab/records/2026-09-30-drew.toml", "a = 1\n");
            }),
            "a record renamed"
        );
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
        assert!(
            found.len() > 3
                && found
                    .iter()
                    .any(|(at, _)| at == "firmware/enforcer/Cargo.toml"),
            "found {:?}",
            found.iter().map(|(at, _)| at).collect::<Vec<_>>()
        );
        for (at, m) in found {
            for (name, spec) in dependencies(&m) {
                assert!(
                    pinned(&spec) != Some(false),
                    "{at}: {name} is not pinned to one version: {spec}"
                );
            }
        }
    }

    /// The Pico enforcer's build script reads lab/limits.toml through crates/lab, so the safety
    /// path must not pull in the database or the catalogue.
    #[test]
    fn the_lab_crate_needs_only_serde_and_toml() {
        assert_eq!(runtime_dependencies("crates/lab"), ["serde", "toml"]);
    }

    /// Reading the records is parsing and checking; the database comes after, in the warehouse. The
    /// lab log is JSON, chained by sha256.
    #[test]
    fn parsing_records_needs_no_database() {
        assert!(runtime_dependencies("crates/warehouse").contains(&"duckdb".to_string()));
        assert_eq!(
            runtime_dependencies("crates/records"),
            ["serde", "serde_json", "sha2", "tentzhen-lab", "toml"]
        );
    }

    /// The enforcer's logic goes into the Pico's firmware, so it depends on nothing at run time,
    /// and its build script reads the limits through crates/lab alone.
    #[test]
    fn the_enforcer_depends_on_nothing_at_run_time() {
        assert_eq!(
            runtime_dependencies("firmware/enforcer"),
            Vec::<String>::new()
        );
        assert_eq!(
            dependencies_of("firmware/enforcer", "build-dependencies"),
            ["sha2", "tentzhen-lab"]
        );
    }

    /// Every workflow in .github/workflows, by file name, with its text.
    fn workflows() -> Vec<(String, String)> {
        let mut out: Vec<(String, String)> = fs::read_dir(root().join(".github/workflows"))
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|x| x == "yml"))
            .map(|p| {
                let name = p.file_name().unwrap().to_string_lossy().into_owned();
                (name, fs::read_to_string(&p).unwrap())
            })
            .collect();
        out.sort();
        out
    }

    /// Every action a workflow uses is pinned to a commit, with the release it is in a comment
    /// (CLAUDE.md: "Pin every dependency version"): a tag can be moved, a commit cannot.
    #[test]
    fn every_action_is_pinned_by_commit() {
        let mut actions = 0;
        for (file, text) in workflows() {
            for line in text.lines() {
                let Some((_, used)) = line.split_once("uses: ") else {
                    continue;
                };
                actions += 1;
                let (action, rest) = used.split_once('@').unwrap_or((used, ""));
                let (sha, release) = rest.split_once(" # ").unwrap_or((rest, ""));
                let pinned = sha.len() == 40
                    && sha
                        .chars()
                        .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
                    && release.starts_with('v');
                assert!(pinned, "{file}: {action} is not pinned by commit: {line:?}");
            }
        }
        assert!(actions >= 7, "found {actions} actions");
    }

    /// A newer push to a pull request cancels the Quality Gate's older run on it; a run on main, or
    /// one started by hand, is a concurrency group of its own, so nothing cancels it or holds it
    /// back. Text only: that GitHub does so is seen on a pull request, not here.
    #[test]
    fn a_newer_push_cancels_only_a_pull_request_s_older_gate_run() {
        let (_, gate) = workflows()
            .into_iter()
            .find(|(f, _)| f == "quality-gate.yml")
            .unwrap();
        for needed in [
            "\nconcurrency:\n",
            "  group: quality-gate-${{ github.event.pull_request.number || github.run_id }}\n",
            "  cancel-in-progress: ${{ github.event_name == 'pull_request' }}\n",
        ] {
            assert!(gate.contains(needed), "the Quality Gate lacks {needed:?}");
        }
        assert_eq!(
            gate.matches("concurrency:").count(),
            1,
            "one concurrency block"
        );
        assert!(
            !gate.lines().any(|l| l.trim_start().starts_with("queue:")),
            "a queue would hold main's runs together"
        );
    }

    /// tentzhen.com is published from main only, after main's Quality Gate passed on that commit:
    /// the Pages workflow's guard names each condition. `branches: [main]` matches a branch's name
    /// alone, which a pull request from a fork can share.
    #[test]
    fn the_pages_workflow_publishes_only_a_commit_main_s_gate_passed() {
        let (_, gate) = workflows()
            .into_iter()
            .find(|(f, _)| f == "quality-gate.yml")
            .unwrap();
        assert!(gate.contains("\nname: Quality Gate\n"));
        let (_, pages) = workflows()
            .into_iter()
            .find(|(f, _)| f == "pages.yml")
            .unwrap();
        for needed in [
            "workflows: [Quality Gate]",
            "github.event.workflow_run.conclusion == 'success'",
            "github.event.workflow_run.event == 'push'",
            "github.event.workflow_run.head_branch == 'main'",
            "github.event.workflow_run.head_repository.full_name == github.repository",
            "(github.event_name == 'workflow_dispatch' && github.ref == 'refs/heads/main')",
            "ref: ${{ github.event.workflow_run.head_sha || github.sha }}",
            "path: site\n",
        ] {
            assert!(pages.contains(needed), "pages.yml lacks {needed:?}");
        }
    }

    /// The Kani harnesses and tests in the workspace, by package name.
    #[derive(Default)]
    struct Code {
        harnesses: BTreeMap<String, BTreeSet<String>>,
        tests: BTreeMap<String, BTreeSet<String>>,
    }

    /// Reads every member's sources for functions marked `#[kani::proof]` or `#[test]`, through
    /// any other attributes between the mark and the `fn`.
    fn code() -> Code {
        let mut code = Code::default();
        for m in members() {
            let package = manifest(&root().join(&m).join("Cargo.toml"))["package"]["name"]
                .as_str()
                .unwrap()
                .to_string();
            let mut stack = vec![root().join(&m).join("src")];
            while let Some(dir) = stack.pop() {
                for entry in fs::read_dir(dir).unwrap() {
                    let path = entry.unwrap().path();
                    if path.is_dir() {
                        stack.push(path);
                        continue;
                    }
                    if path.extension().is_none_or(|x| x != "rs") {
                        continue;
                    }
                    let mut pending: Option<bool> = None;
                    for line in fs::read_to_string(&path).unwrap().lines() {
                        let line = line.trim();
                        if line.starts_with("#[kani::proof") {
                            pending = Some(true);
                        } else if line == "#[test]" {
                            pending = Some(false);
                        } else if let (Some(kani), Some(rest)) = (pending, line.strip_prefix("fn "))
                        {
                            let name = rest.split(['(', '<']).next().unwrap().to_string();
                            let set = if kani {
                                &mut code.harnesses
                            } else {
                                &mut code.tests
                            };
                            set.entry(package.clone()).or_default().insert(name);
                            pending = None;
                        } else if !line.starts_with("#[") {
                            pending = None;
                        }
                    }
                }
            }
        }
        code
    }

    /// The Kani version the Quality Gate installs.
    fn kani_version() -> String {
        let workflow =
            fs::read_to_string(root().join(".github/workflows/quality-gate.yml")).unwrap();
        let (_, rest) = workflow
            .split_once("kani-verifier --version ")
            .expect("the Quality Gate installs Kani");
        rest.split_whitespace().next().unwrap().to_string()
    }

    /// What is wrong with a claim's referent of `kind` (`check`, `test` or `proof`), if anything.
    /// A check is `<package> <harness>; Kani <version>; <bound>`, naming a harness in that package
    /// and the version the Quality Gate runs. A test is `<package> <test>`. A proof is
    /// `<path>; <tool> <version>`, naming a file in the repo.
    fn referent_problem(kind: &str, referent: &str, code: &Code, kani: &str) -> Option<String> {
        let fields: Vec<&str> = referent.split("; ").collect();
        let named = |set: &BTreeMap<String, BTreeSet<String>>| match fields[0].split_once(' ') {
            Some((package, name)) if set.get(package).is_some_and(|s| s.contains(name)) => None,
            _ => Some(format!("{kind} {referent:?} names no {kind} that exists")),
        };
        match kind {
            "check" => {
                if fields.len() != 3 || fields[2].trim().is_empty() {
                    return Some(format!(
                        "check {referent:?} is not `<package> <harness>; Kani <version>; <bound>`"
                    ));
                }
                if fields[1] != format!("Kani {kani}") {
                    return Some(format!(
                        "check {referent:?} names {}, but the Quality Gate runs Kani {kani}",
                        fields[1]
                    ));
                }
                named(&code.harnesses)
            }
            "test" => named(&code.tests),
            "proof" => match fields.as_slice() {
                [path, tool] if !tool.is_empty() && root().join(path).is_file() => None,
                _ => Some(format!(
                    "proof {referent:?} is not `<path>; <tool> <version>` naming a file"
                )),
            },
            _ => unreachable!(),
        }
    }

    /// Every claim in parts/ and builds/, as its record and its table.
    fn claims() -> Vec<(String, Table)> {
        let mut files: Vec<PathBuf> = fs::read_dir(root().join("parts"))
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        for b in fs::read_dir(root().join("builds")).unwrap() {
            let b = b.unwrap().path();
            if b.is_dir() {
                files.extend(fs::read_dir(b).unwrap().map(|e| e.unwrap().path()));
            }
        }
        files.sort();
        let mut out = Vec::new();
        for f in files
            .iter()
            .filter(|f| f.extension().is_some_and(|x| x == "toml"))
        {
            let at = f.strip_prefix(root()).unwrap().display().to_string();
            for c in manifest(f)
                .get("claim")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                out.push((at.clone(), c.as_table().unwrap().clone()));
            }
        }
        out
    }

    /// A claim's `check`, `test` or `proof` names code that exists, so no record can take a grade
    /// by naming evidence that is not there (docs/architecture.md, "Rules").
    #[test]
    fn a_code_referent_names_code_that_exists() {
        let (code, kani) = (code(), kani_version());
        let mut checks = 0;
        for (at, claim) in claims() {
            for kind in ["check", "test", "proof"] {
                if let Some(r) = claim.get(kind).and_then(Value::as_str) {
                    checks += usize::from(kind == "check");
                    let problem = referent_problem(kind, r, &code, &kani);
                    assert!(problem.is_none(), "{at}: {}", problem.unwrap());
                }
            }
        }
        assert!(checks >= 5, "found {checks} check referents");
        // Each way a referent can be wrong is refused, and the right one is not.
        let good = format!(
            "tentzhen-enforcer a_refused_input_changes_nothing; Kani {kani}; one input from any state"
        );
        assert_eq!(referent_problem("check", &good, &code, &kani), None);
        for (kind, bad, why) in [
            (
                "check",
                good.replace("a_refused", "an_unwritten"),
                "names no check",
            ),
            (
                "check",
                good.replace(&kani, "0.1.0"),
                "the Quality Gate runs",
            ),
            (
                "check",
                good.replace("tentzhen-enforcer", "tentzhen-lab"),
                "names no check",
            ),
            (
                "check",
                good.replace("; one input from any state", ""),
                "is not",
            ),
            (
                "check",
                good.replace("one input from any state", " "),
                "is not",
            ),
            // A test is not a harness, nor a harness a test.
            (
                "check",
                good.replace(
                    "a_refused_input_changes_nothing",
                    "a_trip_keeps_its_first_cause",
                ),
                "names no check",
            ),
            (
                "test",
                "tentzhen-enforcer a_refused_input_changes_nothing".into(),
                "names no test",
            ),
            (
                "test",
                "tentzhen-enforcer no_such_test".into(),
                "names no test",
            ),
            (
                "proof",
                "gateware/none.sby; SymbiYosys 0.60".into(),
                "naming a file",
            ),
            ("proof", "Cargo.toml".into(), "naming a file"),
        ] {
            let problem = referent_problem(kind, &bad, &code, &kani).unwrap_or_default();
            assert!(problem.contains(why), "{bad}: {problem}");
        }
        assert_eq!(
            referent_problem(
                "test",
                "tentzhen-enforcer a_trip_keeps_its_first_cause",
                &code,
                &kani
            ),
            None
        );
        assert_eq!(
            referent_problem("proof", "Cargo.toml; a tool 1.0", &code, &kani),
            None
        );
    }
}
