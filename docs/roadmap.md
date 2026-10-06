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
  lab/         limits.toml, targets.toml (the lab-target list), log/, records/
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
  price. This produces the tier prices, and checks them against the target (`scope.md`, "Price").
- **Lab safety before hardware.** `lab/limits.toml`, the lab-target list (`lab/targets.toml`),
  and a lab log that is hash-chained and only grows on `main`. Its entries are not signed. Commits
  are, but an agent that can commit signs with the person's key (`architecture.md`), so where to
  keep a key an agent cannot use is decided with the Pico enforcer.
- **First cart.** A BOM and cart for tiers 1–2, reviewed by the user, who places the order.
  Nothing in it is made at JLCPCB: tier 2 is modules on jumper wires, wired to extension boards
  that one i9 moves between (`scope.md`, "Architecture"; Jon, 2026-10-06).

## Phase 1 — Bring-up

LiteX + VexRiscv on the Colorlight; Rust "hello" over UART, through the extension board's DAPLink
serial port.
Bitstream hash and toolchain versions recorded in the build artifact. All `measured`. Then, on the
i9 alone, the loopback test that settles whether it reads the tier-3 ADC's 1.8 V outputs
(`scope.md`, "Open questions").

## Phase 2 — First capture

One AD9226 into on-chip RAM with an edge trigger, read out and plotted from a Rust host tool.
The capture core's properties are `proven` or `checked`, and its cocotb tests pass, **before**
it touches hardware. On arrival every AD9226 module goes through incoming QA: its logic supply,
then the AD8138 VOCM check (`scope.md`), as `builds/digitizer/v1.toml`'s walkthrough has it.

## Phase 3 — Two channels + streaming

First the Ethernet adapter for P1: the extension board has no Ethernet jacks (`scope.md`), so a
small board of our own carries a jack with built-in transformers on P1, from KiCad files, made at
JLCPCB. It is the project's first board order (moved here from Phase 0 by Jon, 2026-10-06). Then
Etherbone reachable from the host, SDRAM buffering, continuous Ethernet streaming, samples landing
in DuckDB with full metadata.
Sustained sample rate over the headers' jumper wires is `measured`, not assumed from the ADC's
rating.

## Phase 4 — Instrument interface

SCPI server (pyvisa-compatible), ngscopeclient driver, and an MCP server so an AI agent can
drive the instrument within `lab/limits.toml`.

## Phase 5 — Signal generator

DAC module + DDS / arbitrary-waveform core, with formal properties on the core.

## Phase 6 — Front end v1

KiCad design, with the ADS4245 and the oscillator that clocks it on the board (`scope.md`, "The
tier-3 ADC"), ngspice-verified (bandwidth, noise, overload), fabricated and assembled at JLCPCB.
It weighs putting the AD9767 on a board of our own in place of the DAC module.
Prefer JLCPCB basic parts; the BOM tool's unique-extended-part count is reported with the design.

## Phase 7 — Characterization

Noise floor, ENOB, bandwidth and timebase accuracy. Publish a spec sheet where every number is
`measured` with a record id, and the trusted base is printed beside it.

## Phase 8 — First application

Ultrasonic sonar: transmit a chirp, capture the echo, compute range.

## Parallel tracks, same repo

- Programmable supply and electronic load (`vision.md`, "The bench"). Their limit enforcers
  have their limit logic `checked` with Kani before they drive anything.
- Logic analyzer, on the second extension board (`scope.md`, "Architecture"): the i9's header
  pins as inputs. No build yet.
- Research: a proven control core in the Lightbulb style (`verification.md`).
- Research: signing keys in a TROPIC01 secure element.
