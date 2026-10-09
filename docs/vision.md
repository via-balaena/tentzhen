# Vision

Tentzhen is a site for economical electronics projects that take advantage of the electronics
surplus, to help people survive, and eventually thrive, in the AI economic transition happening
now (Jon, 2026-10-08). The bench below is project #1; the mesh (`mesh.md`) is project #2.

## Who this is for

Someone with a laptop, a lot of curiosity, and not much money.

Three things have become cheap at the same time. Shenzhen's electronics surplus puts FPGA
boards, ADCs and passives within reach of a small budget. Open toolchains — Yosys, nextpnr,
KiCad, ngspice, Rust — mean nothing in the chain has to be bought or taken on faith. And an AI
assistant can now play the role of the experienced engineer most people could never hire:
explaining a datasheet, reviewing a schematic, writing a first test.

What is still expensive is the **bench**: the oscilloscope, signal generator, supply and load
that turn "I built a circuit" into "I know what my circuit does". Red Pitaya's shop listed its
kits from €415 (Wayback snapshot 2026-09-18; whether that includes VAT is unknown). Project #1
is an attempt to build that bench from generic parts, and to publish every step so others can
follow.

The name is tent + Shenzhen: a bench you can set up anywhere, stocked from the world's
electronics surplus. In Chinese it is written 腾振 (téng zhèn): 腾, to soar; 振, to vibrate or
rouse — the 振 of 振荡 (oscillation) and 振幅 (amplitude).

Tentzhen is a Via Balaena project. Its site is https://tentzhen.com.

## What "rich" means here

Capability, not profit. The goal is that someone who starts with a computer and a small parts
budget ends up able to design, build and *verify* hardware — and to teach the next person.

## Principles

1. **Open first.** Open hardware, gateware, firmware and toolchains. Prefer parts and tools
   anyone can buy and inspect.
2. **Every claim says how it is known.** Proven, checked, tested, measured, trusted, or unknown —
   see `verification.md`. A spec sheet that hides its trusted base is marketing.
3. **Cheap, and measured.** A part comes from wherever it is cheap: the maker's own store, a
   reseller, a used sale, a thrift store, a junked device, a drawer at home. Until it is measured,
   where it came from sets how far its claims are trusted (`trusted-base.toml` keeps
   `aliexpress-modules` apart from `maker-datasheets`). Incoming QA on the bench, not the source,
   decides whether it is good, and the parts page shows each result beside the source it came from.
   A part comes from an authorized seller only where no bench test on arrival can show it does its
   job (`sourcing.md`). Cheap modules ship with real defects (`scope.md` records one on the AD9226
   module), which is what the measuring is for.
4. **Everything scriptable.** Every instrument function is controllable and loggable from code,
   including by an AI agent over MCP.
5. **Safety below the agent.** An AI can drive the bench, so hard limits live in hardware it
   cannot reconfigure.
6. **Rust wherever possible.**
7. **Document for peers.** Write for someone following along on a budget.
8. **Build with what exists, and make friends.** Before building a piece, look for who has built
   it already (`prior-art.md`). Use it where it is not the point, credit it, and send fixes
   upstream.

## Entry tiers

Nobody should need to buy anything to start contributing.

| tier | you need | you can do |
|---|---|---|
| 0 | a computer | simulate gateware (Verilator, cocotb), prove gateware properties (SymbiYosys), simulate analog (ngspice), build the bitstream, run host tools against a simulated instrument |
| 1 | + an FPGA board and a Pico | bring-up, digital loopback, Pico as a test-signal source |
| 2 | + ADC/DAC modules | a working two-channel scope and generator on module front ends |
| 3 | + the front-end PCB (JLCPCB) | the real instrument, characterized |

Tier prices are not written here. They come from the BOM tool (`roadmap.md`, Phase 0) with a
date attached, because a price typed into a doc carries no date and cannot be re-checked.

## The bench

Instrument #1 is the digitizer and signal generator (`scope.md`). The rest of the bench lives in
the same repo and follows the same rules:

- **Programmable supply:** Kungber PSU → DPS5005 running OpenDPS → Tock-based Pico bridge
  enforcing limits.
- **Electronic load:** MOSFET + op amp + MCP4725 DAC + ADS1115, Pico-controlled with thermal
  cutoff.
- **Parts database:** our own TOML records (identifiers and our measurements) → DuckDB, enriched
  at runtime from each user's own LCSC API key (`sourcing.md`).

## The mesh

Project #2 is a LoRa mesh of handhelds, repeaters and a phone, running UMSH on nRF52840 boards,
with a port to Tock as the project's own work (`mesh.md`).
