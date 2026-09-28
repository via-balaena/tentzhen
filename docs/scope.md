# Instrument #1: two-channel digitizer and signal generator

Research date for everything below: 2026-09-28, unless a line says otherwise.

## Architecture

- **FPGA board:** Colorlight i9 v7.2 + extension board. FPGA LFE5U-45F-6BG381C; SDRAM
  M12L64322A (8 MB, 32-bit); W25Q64 flash; 2× B50612D gigabit Ethernet PHYs; 25 MHz clock. The
  extension board adds a DAPLink (JTAG + USB-CDC), 6 dual-PMOD connectors and USB-C.
  (github.com/wuxx/Colorlight-FPGA-Projects, `colorlight_i9_v7.x.md`)
- **SoC:** LiteX with a VexRiscv soft core, LiteEth (Ethernet streaming), LiteDRAM (SDRAM capture
  buffer). Custom cores — capture, trigger, DDS — in Amaranth so they can carry formal
  properties (`verification.md`).
- **Inputs:** 2× AD9226 modules (12-bit, up to 65 MS/s), one per channel, on PMOD headers.
- **Outputs (phase 5):** dual-DAC module (AD9767-class) with a waveform-generation core.
- **Analog front end:** custom PCB — input protection, switchable attenuation, AC/DC coupling,
  buffer amplifier, anti-alias filter.
- **Host:** Rust services for ingest (DuckDB), a SCPI server and an MCP server. UI via an
  ngscopeclient driver, later possibly native egui.

The soft core does not run Linux; 8 MB of SDRAM is reserved for the capture buffer. It runs
bare-metal Rust or Tock.

## Against the reference

| | Red Pitaya STEMlab 125-14 | Tentzhen #1 target |
|---|---|---|
| channels in / out | 2 / 2 | 2 / 2 (outputs in phase 5) |
| sample rate | 125 MS/s | up to 65 MS/s (AD9226 rating) |
| resolution | 14-bit | 12-bit |
| bandwidth | DC–60 MHz | 20–30 MHz usable — a target, `unknown` until measured |
| input range | ±1 V / ±20 V, 1 MΩ ∥ 10 pF | set by the front end |

Red Pitaya figures: redpitaya.readthedocs.io, `developerGuide/hardware/ORIG_GEN/125-14`.

Expect more noise than a Red Pitaya on front end v1. That is fine. Publish real numbers.

## Known part issues

- **AD9226 modules driven by an AD8138 clip early.** On that variant the AD8138's VOCM pin is
  wired to the AD9226's VREF (2 V) instead of its common-mode level (2.5 V). Documented fix: cut
  the trace and add a jumper wire; gain is set by R2/R14.
  (github.com/machcnz/VHSDecode-HSDAOH-Capture-Hardware-Development,
  `initial_setup_and_modification/README.md`, rev 1.1, 2026-09-15.) Whether the modules we buy
  are that variant is `unknown` until they arrive; incoming QA checks it first.
- At least one module variant is specified for ±5 V (10 Vpp) input scaled to 1–3 V, with a 5 V
  supply and 3.3 V logic (manuals.plus/ae/1005005576645194). Input stages differ between
  sellers, so each unit is characterized on arrival.
- **Availability:** both Amazon listings for the Colorlight i9 read "Currently unavailable" on
  2026-09-28. AliExpress availability is `unknown`.

## Calibration references (no expensive gear required)

- DC accuracy: LM4040 precision reference (genuine, from LCSC or DigiKey).
- Timebase: crystal oscillator of known frequency.
- Edges and timing: Pico PWM output.
- Noise and distortion: clean audio-range sine from a sound card.

## Reuse before reinventing

- LiteX / LiteX-Boards: the i9 is `--board i9` on the `colorlight_i5` target
  (`litex_boards/targets/colorlight_i5.py`, read 2026-09-28).
- LiteScope for in-FPGA debugging.
- Red Pitaya's open FPGA code for capture/trigger structure (check the license before copying).
- ThunderScope's open design files as a front-end reference (check the license).
- ngscopeclient for the UI.
- Upstream Tock's `litex_vexriscv` chip, if the soft core runs Tock.

## Open questions

- Current state of Rust crates for LiteX-generated SoCs.
- Whether 65 MS/s survives PMOD headers and ribbon cable — signal integrity is `unknown`.
- Best available DAC module on AliExpress; confirm its specs by measurement.
- Front-end op-amp selection: bandwidth vs noise vs LCSC availability vs JLCPCB basic-part status.
