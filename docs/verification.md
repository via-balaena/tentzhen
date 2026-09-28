# Verification

"Verified down to the atom" is the aim. This file says how close each layer can get **today**,
with sources, and lists what must still be trusted. Research date for everything below:
2026-09-28, unless a line says otherwise.

## Grades

Every claim about Tentzhen carries one of these, plus a referent (see `CLAUDE.md`):

- `proven` — an unbounded proof was accepted (SymbiYosys `prove` mode via k-induction or PDR;
  Verus). Only as strong as the stated properties.
- `checked` — a bounded check passed (SymbiYosys `bmc`, Kani). True up to the stated bound and
  no further.
- `tested` — tests passed in simulation or on the host.
- `measured` — observed on the bench, with a measurement record.
- `trusted` — assumed, and listed in the trusted base below.
- `unknown` — not established.

The published spec sheet will carry a grade and a referent on every number.

## The stack, layer by layer

| layer | best grade reachable today | how |
|---|---|---|
| Custom gateware cores (capture, trigger, DDS) | `proven` / `checked` | Amaranth `Assert`/`Assume`/`Cover` + SymbiYosys |
| LiteX SoC glue (Migen) | `tested` | Migen has no assertion statements; LiteX has no SBY flow (0 hits) |
| Synthesis output | `checked`, per build, per design | eqy (RTL vs netlist equivalence); eqy itself is unverified |
| Bitstream reproducibility | `unknown` → `measured` in Phase 0 | build twice from a pinned Nix toolchain, compare hashes |
| VexRiscv soft core | `trusted` | riscv-formal covers only the `FormalSimple` config (rv32i, no CSR/MMU/caches), bounded to 20–30 cycles; LiteX's variants are not covered |
| Soft-core firmware (Rust) | `checked` on pure logic | Kani (no inline asm support) |
| Pico limit enforcer | `checked` limit logic + `trusted` kernel isolation | Kani on the limit arithmetic; see Tock below |
| Host tools (Rust) | `tested`, `checked` where it pays | unit tests, Kani |
| Analog front end | `measured` | ngspice first, then the bench |
| ADC/DAC modules | `measured` | incoming QA on every unit |
| Silicon (ECP5, RP2040/RP2350), toolchains (Yosys, nextpnr, Trellis, rustc/LLVM) | `trusted` | — |

## Where seL4 and Pancake fit — and don't, on this hardware

- **seL4** has verified configurations for RV64 and ARM, not RV32; RV32 runs unverified, on
  QEMU only. It effectively requires an MMU. No VexRiscv or LiteX port was found.
  (docs.sel4.systems/projects/sel4/verified-configurations.html, docs.sel4.systems/Hardware/)
  Whether its RV64 proof reaches binary code is `unknown`: a 2021 post says yes, the current
  configurations page lists binary correctness only for AArch32.
- **Pancake** has a verified compiler through the CakeML backend, and a verified Ethernet driver
  on ARMv8 (arxiv.org/abs/2501.08249). The sDDF build accepts only aarch64, riscv64 and x86_64,
  so there is no RV32 route. Pancake serial drivers merged into sDDF on 2026-09-21
  (au-ts/sddf#771), off by default, with no claim of verification in the PR.
- **CakeML** has no RV32 backend.

So neither applies to the VexRiscv soft core or the Pico. They become relevant if a host or
instrument controller ever moves to an RV64 or ARMv8 part with an MMU. That is not planned.

## What does apply: Lightbulb, on the same FPGA family

Erbsen et al., PLDI 2021 (adam.chlipala.net/papers/LightbulbPLDI21/LightbulbPLDI21.pdf): one Coq
theorem covers an RV32I Kami processor, the bedrock2 compiler, SPI and Ethernet-chip drivers and
the application, against an I/O-trace specification. **It ran on a Lattice ECP5-85k**, built with
Yosys and nextpnr. Its trusted base included a ~200-line Verilog wrapper, Kami→Bluespec
extraction, the Bluespec compiler, Yosys, nextpnr and Coq. It proves no liveness or timing, and
ran about 10× slower than an unverified core (5.5 ms vs 0.5 ms for the same job).

**Research track, not a plan:** in Tentzhen the sample path is gateware, and the CPU only
configures and moves buffers. Whether that makes a slow, proven control core acceptable is
`unknown`, and so is whether a Kami core fits the Colorlight's ECP5-45F next to the capture
logic (Lightbulb used an 85k). Both are measurable once Phase 2 exists.

## Tock

TickTock (SOSP 2025, ranjitjhala.github.io/static/sosp25-ticktock.pdf) is a fork of Tock that
verifies process isolation with Flux on all ARMv7-M platforms and three RV32 platforms (which
three is `unknown`). **Neither Pico is in its stated scope**: upstream Tock runs the RP2040 as
ARMv6-M (`cortex-m0p`) and the RP2350 as ARMv8-M (`cortex-m33`). So the enforcer's kernel
isolation is `trusted`. Upstream Tock also has a `litex_vexriscv` chip and a LiteX Arty board;
whether the soft core should run Tock rather than bare metal is an open question.

## Toolchain trust

- Yosys has had real mis-synthesis bugs: Verismith found four (FPGA 2020), and issues #5941,
  #5942 and #5946 (June 2026) reported behaviour-changing passes, all closed as completed.
  Verified synthesis exists only as Lutsig (HOL4), which targets Xilinx 7-series, not ECP5.
  This is why eqy per build is in the table above.
- Bit-for-bit reproducibility of ECP5 bitstreams is `unknown` — nextpnr's README says nothing
  about determinism. Phase 0 measures it.

## Trusted base

Everything here is assumed, not shown. Shrinking this list is the long-term work.

- ECP5 and RP2040/RP2350 silicon, including their hard blocks and MPUs.
- Yosys, nextpnr-ecp5, Project Trellis.
- rustc and LLVM.
- The VexRiscv soft core.
- Tock's kernel isolation on the Pico.
- Every AliExpress module, until its incoming-QA record exists.
- The measuring references themselves (LM4040, crystal) — from authorized distributors, and
  cross-checked against each other.
