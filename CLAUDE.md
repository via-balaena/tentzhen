# Tentzhen

A bench of open, verifiable electronics instruments built from cheap generic parts, for people
with more curiosity than money. Instrument #1 is a Red Pitaya-class two-channel digitizer and
signal generator. Pronounced "TENT-zhen" (like "tension").

Read before planning anything: `docs/vision.md` (why and for whom), `docs/roadmap.md` (phases and
acceptance criteria), `docs/verification.md` (how every claim is graded), `docs/sourcing.md`
(LCSC, JLCPCB, incoming QA), `docs/scope.md` (instrument #1).

## Every claim carries its grade

No statement about this project's behaviour — in docs, commits, code comments or replies — is
made without saying how it is known, and pointing at the thing that shows it:

| grade | meaning | referent |
|---|---|---|
| `proven` | an unbounded proof was accepted (SBY `prove`, Verus) | proof file + tool version |
| `checked` | a bounded check passed (SBY `bmc`, Kani) — true only up to the stated bound | harness + bound |
| `tested` | tests passed in simulation or on the host | test name |
| `measured` | observed on the bench | measurement record id |
| `trusted` | assumed, and listed in `docs/verification.md`'s trusted base | the list entry |
| `unknown` | not established | — |

An explanation with no referent is written as `unknown`, not as "probably because X".
Model-generated code, including yours, is `unknown` until it earns a grade.

## Safety and permission rules

Never do the following without explicit approval from the user in the current session:

- **Place orders or spend money** (LCSC, AliExpress, JLCPCB, anything). Building carts and BOMs is
  fine. The LCSC client must not implement the API's order endpoints.
- **Write irreversible settings:** OTP memory, eFuses, secure-boot keys, flash protection bits on
  any device.
- **Command a power supply or load** beyond the limits in `lab/limits.toml` (create it before any
  power automation). Default ceilings: 3.6 V and 200 mA for Pico/3.3 V circuits.
- **Flash firmware to a device** that is not listed as a lab target.
- **Expose any service** outside the isolated lab network, or add internet access for instruments.

Always:

- Keep mains voltage out of scope entirely. No designs or instructions involving mains wiring.
- Safety limits are enforced BELOW the layer an AI agent controls — in the Pico enforcer, not
  only in host software — so no agent can talk its way past them.
- Log every hardware action (what, when, parameters, result) to the lab's append-only log.

## LCSC and JLCPCB data

LCSC's API terms forbid bulk capture and forbid hosting or providing retrieved material
(including datasheets and images) to any third party (`docs/sourcing.md`, read 2026-09-28). So:

- **Never commit LCSC- or JLCPCB-API-derived data** (prices, stock, datasheets, images, specs).
  Commit only identifiers (`C` numbers, MPNs) and data we measured ourselves.
- API keys live in the macOS Keychain or the environment — never in the repo, a file, or a log.
- Each user brings their own key. Caches are local, under the user's data dir, never in the tree.

## Language policy

| Layer | Language / tools |
|---|---|
| Custom gateware cores (capture, trigger, DDS) | Amaranth — it has `Assert`/`Assume`/`Cover` for SBY; Migen has none |
| SoC integration | LiteX (Migen), importing the Amaranth cores as Verilog |
| Gateware tests and proofs | cocotb + Verilator; SymbiYosys |
| Soft-core firmware (VexRiscv, `riscv32`) | Rust |
| Pico firmware (bridges, limit enforcement) | Rust on Tock OS |
| Host software (ingest, SCPI, MCP, CLI, LCSC client) | Rust (tokio, duckdb) |
| Analog simulation / PCB | ngspice / KiCad |
| Build reproducibility | Nix |

Python only for gateware, cocotb tests, and throwaway exploration. Anything long-lived gets
rewritten in Rust.

## Conventions

- One repo for the whole bench. Planned layout in `docs/roadmap.md`, Phase 0.
- Measurements: SI units in data; raw ADC codes stored alongside converted values.
- Every experiment gets a record: hardware revision, gateware hash, firmware hash, toolchain
  versions, setup notes, results.
- KiCad symbols carry an `LCSC Part #` field (the field JLCPCB fabrication plugins read).
- `main` takes changes only through a PR that passes the Quality Gate
  (`.github/workflows/quality-gate.yml`), with signed commits and linear history; the
  `main-protection` ruleset enforces it. Pin every dependency version.
- Docs are written for someone following along on a budget.
