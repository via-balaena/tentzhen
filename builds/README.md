# Builds

One directory per build, one file per version: `builds/<name>/v1.toml`, `v2.toml`, … An optional
drawing sits beside it (`v1.svg`). `crates/site` turns these into the pages under `site/builds/`.

- **Versions.** `status = "draft"` may still change. `status = "published"` is frozen: the Quality
  Gate fails a PR that edits or deletes a published version or its drawing. Change it by adding
  the next version, with `changes` saying what changed. Every version keeps its page; the build's
  own page always sends you to the newest.
- **Parts by their lowest-level name** — `RP2350`, `AD9226`, `DPS5005` — the name more than one
  maker builds to. Each has a record in `parts/`. Commodities are named by their properties on
  the line itself (`commodity = "wire, 18 AWG, silicone"`). No prices, sellers or brands.
- **Builds from builds.** `[[uses]]` names another build and pins its version. Pages show
  "Uses" and "Used in" both ways.
- **Firmware** is pinned to a release, a file and its sha256, never copied into the repo.
- **Walkthrough.** Each `[[step]]` says what to `do`, optionally a command to `run`, what you
  should see (`expect`), and a rule for agents (`agent`) where one applies.
- **Claims.** Each `[[claim]]` has an `id`, says one thing, and carries its referents: `proof`,
  `check` or `test` (graded `proven`, `checked` or `tested`), and `record` (a lab measurement,
  graded `measured`) or `trusted` (an entry in the trusted base, graded `trusted`). No field types
  a grade; with no referent it is `unknown`. A part record carries claims in the same shape.

After editing, regenerate the pages:

```fish
cargo run -p tentzhen-site
```
