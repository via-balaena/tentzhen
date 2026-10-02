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
- **Class.** Each line gives the letter a schematic labels its thing with: `A` a board or module,
  `C` capacitor, `F` fuse, `J` connector, `MP` mechanical part, `PS` power supply, `Q` transistor,
  `R` resistor, `S` switch, `U` integrated circuit, `W` wire or cable, `XF` fuse holder. One thing,
  a part and its form or a commodity and its form, has one class in every build version that writes
  it, old ones too: once a published version writes a thing, its class is settled. The parts page
  groups by it. A new letter goes in `crates/records` (`Class`) and in `silver.line`'s check.
- **Builds from builds.** `[[uses]]` names another build and pins its version. Pages show
  "Uses" and "Used in" both ways.
- **Firmware** is pinned to a release, a file and its sha256, never copied into the repo.
- **Walkthrough.** Each `[[step]]` says what to `do`, optionally a command to `run`, what you
  should see (`expect`), and a rule for agents (`agent`) where one applies.
- **Claims.** Each `[[claim]]` has an `id` and says one thing. It carries any of these referents:
  `proof`, `check` and `test` grade it `proven`, `checked` or `tested` in simulation; `record` (a
  lab measurement) and `trusted` (the id of an entry in `trusted-base.toml`) grade it `measured`
  or `trusted` on the bench. Each axis shows the strongest it has, and `unknown` with none; no
  field types a grade. Numbers it states go in `values`, each key ending in its unit
  (`min_volts`), and `says` names them in backticks; the pages write the number in. A part record
  carries claims in the same shape.
- **Code referents** name code that exists, or the Quality Gate fails
  (`a_code_referent_names_code_that_exists`): a `check` is `<package> <harness>; Kani <version>;
  <bound>`, at the version the gate installs; a `test` is `<package> <test>`; a `proof` is
  `<path>; <tool> <version>`.
- **Never a lab target.** `never_a_lab_target = "<why>"` marks a build no agent may flash without
  a person's approval, such as the Pico enforcer, which holds the limits. `lab/targets.toml`
  refuses a target listed as it or as any build that uses it.

After editing, regenerate the pages:

```fish
cargo run -p tentzhen-site
```
