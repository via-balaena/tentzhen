# Roadmap

Each phase ends on acceptance criteria that can be checked, each with the grade it must reach
(`verification.md`). A phase is done when its criteria are met, not when its code is written.

## Phase 0 — Foundations (no hardware, $0)

Everything here runs at tier 0, and all of it must exist before the first order is placed.

- **Layout.** One repo for the whole bench:
  ```
  crates/      Rust: host tools, lcsc client, parts DB, lab log
  gateware/    Amaranth cores + LiteX SoC
  firmware/    soft-core (riscv32) and Pico firmware
  hardware/    KiCad projects
  sim/         ngspice
  parts/       TOML part records — identifiers and our own measurements only
  lab/         limits.toml, lab-target list, lab-log format
  docs/
  ```
  Directories are created when their first real file arrives, not before.
- **Pinned toolchain.** A Nix flake pinning Yosys, nextpnr-ecp5, Trellis, Verilator, SymbiYosys,
  Amaranth, LiteX, ngspice and Rust.
- **Bitstream reproducibility, `measured`.** Build the LiteX Colorlight i9 target twice from the
  flake and compare hashes. No board needed.
- **First formal core, `proven` or `checked`.** An Amaranth edge-trigger core with SymbiYosys
  properties and a cocotb test. This also establishes whether the Amaranth → SBY flow works,
  which is `unknown` today.
- **`crates/lcsc`, `tested`.** Typed client for the LCSC API: request signing, both rate limits,
  local cache, Keychain credentials, no order endpoints. Tested against recorded fixtures we
  generate ourselves, since real responses may not be committed.
- **LCSC API application.** Submitted 2026-09-28 as Via Balaena; not yet approved. The tools
  work without it (jlcparts for data, LCSC's Upload a BOM page for carts).
- **Parts records + BOM tool, `tested`.** TOML records → DuckDB; `bom` prices a BOM from the
  user's LCSC key or jlcparts, reports unique JLCPCB extended parts, and prints the date of every
  price. This produces the tier prices.
- **Lab safety before hardware.** `lab/limits.toml`, the lab-target list, and a hash-chained,
  signed, append-only lab-log format.
- **First cart.** A BOM and cart for tiers 1–2, reviewed by the user, who places the order.

## Phase 1 — Bring-up

LiteX + VexRiscv on the Colorlight; Rust "hello" over UART; Etherbone reachable from the host.
Bitstream hash and toolchain versions recorded in the build artifact. All `measured`.

## Phase 2 — First capture

One AD9226 into on-chip RAM with an edge trigger, read out and plotted from a Rust host tool.
The capture core's properties are `proven` or `checked`, and its cocotb tests pass, **before**
it touches hardware. On arrival every AD9226 module goes through incoming QA, starting with the
AD8138 VOCM check (`scope.md`).

## Phase 3 — Two channels + streaming

SDRAM buffering, continuous Ethernet streaming, samples landing in DuckDB with full metadata.
Sustained sample rate over PMOD is `measured`, not assumed from the ADC's rating.

## Phase 4 — Instrument interface

SCPI server (pyvisa-compatible), ngscopeclient driver, and an MCP server so an AI agent can
drive the instrument within `lab/limits.toml`.

## Phase 5 — Signal generator

DAC module + DDS / arbitrary-waveform core, with formal properties on the core.

## Phase 6 — Front end v1

KiCad design, ngspice-verified (bandwidth, noise, overload), fabricated and assembled at
JLCPCB. Prefer JLCPCB basic parts; the BOM tool's unique-extended-part count is reported with
the design.

## Phase 7 — Characterization

Noise floor, ENOB, bandwidth and timebase accuracy. Publish a spec sheet where every number is
`measured` with a record id, and the trusted base is printed beside it.

## Phase 8 — First application

Ultrasonic sonar: transmit a chirp, capture the echo, compute range.

## Parallel tracks, same repo

- Programmable supply and electronic load (`vision.md`, "The bench"). Their limit enforcers
  have their limit logic `checked` with Kani before they drive anything.
- Research: a proven control core in the Lightbulb style (`verification.md`).
- Research: signing keys in a TROPIC01 secure element.
