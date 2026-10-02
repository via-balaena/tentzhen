# Instrument #1: two-channel digitizer and signal generator

Research date for everything below: 2026-09-28, unless a line says otherwise.

## Architecture

- **FPGA board:** Colorlight i9 v7.2 + extension board. FPGA LFE5U-45F-6BG381C; SDRAM
  M12L64322A (8 MB, 32-bit); W25Q64 flash; 2× B50612D gigabit Ethernet PHYs; 25 MHz clock. The
  extension board adds a DAPLink (JTAG + USB-CDC), USB-C, HDMI and six 2×15 headers. P1 carries
  only the Ethernet pairs. P2–P6 carry 98 FPGA I/O, each header laid out as two PMOD-compatible
  sites and four more pins, with 5 V on pin 23. (github.com/wuxx/Colorlight-FPGA-Projects,
  `colorlight_i9_v7.x.md` and `schematic/i5-i9-extboard.pdf`; the headers read 2026-10-01)
- **Ethernet needs an adapter:** the extension board has no RJ45 jacks, and P1 carries the PHYs'
  bare pairs. Of the i5, which uses the same extension board, Tom Verbeure wrote that the module
  "only contains the transceivers, not the Ethernet transformers or RJ45 connectors" ("The
  Colorlight i5 as FPGA development board", tomverbeure.github.io, 2021-01-22); that the i9 is the
  same is `unknown`, as its schematic is not public. Kazumoto Kojima's i5ether is an adapter for
  P1, in kazkojima/colorlight-i5-tips, with no license file. Until one is built, the host reaches
  the board through its DAPLink's USB-CDC serial port.
- **SoC:** LiteX with a VexRiscv soft core, LiteEth (Ethernet streaming), LiteDRAM (SDRAM capture
  buffer). Custom cores — capture, trigger, DDS — in Amaranth so they can carry formal
  properties (`verification.md`).
- **Inputs:** 2× AD9226 modules (12-bit, up to 65 MS/s), one per channel, on headers P2 and P3 by
  jumper wire. Each module has a 2×10 header: 14 signals and 5 V.
- **Outputs (phase 5):** a dual AD9767 module in the AN9767's pinout (32 signals and 5 V, on P4 and
  P5) with a waveform-generation core.
- Which header carries what, and why: `builds/digitizer/v1.toml`.
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
  `initial_setup_and_modification/README.md`, rev 1.1, 2026-09-15.) The module's V1.2 schematic
  (2020-02-21, `docs/AD9226-V1.2-board.pdf` in kushpet/ebaz4205-sdr) shows that wiring, read
  2026-10-01. Why it clips early is `unknown`. Whether the modules we buy are that variant is
  `unknown` until they arrive; incoming QA checks it (`builds/digitizer/v1.toml`).
- **Bit order differs by seller.** The AD9226's BIT1 is its MSB (`parts/ad9226.toml`). 凌智电子's
  manual (V2.3, 2021-06-03) labels BIT1 as D0; on the V1.2 board, D0 is header pin 5, which its
  schematic wires to BIT12, the LSB (traced by eye, 2026-10-01). Incoming QA reads a ramp to tell.
- **The extension board's USB-C port has no CC resistors** (the schematic marks CC1 and CC2 not
  connected), so a USB-C to USB-C cable may give it no power. Power it from a USB-A port.
- At least one module variant is specified for ±5 V (10 Vpp) input scaled to 1–3 V, with a 5 V
  supply and 3.3 V logic (manuals.plus/ae/1005005576645194). Input stages differ between
  sellers, so each unit is characterized on arrival.
- **Availability:** both Amazon listings for the Colorlight i9 read "Currently unavailable" on
  2026-09-28. AliExpress availability is `unknown`: its pages answered automated fetches with a
  captcha on 2026-10-01, so a person checks.

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
- Whether 65 MS/s survives 10 cm jumper wires — signal integrity is `unknown`. Scoped
  (github.com/wavius/Scoped) ran an AD9226 module at a 25 MHz clock on 20 cm ones.
- Whether one USB port powers the i9 and three modules: the extension board's fuse rating is not
  printed, and the draw is not measured.
- The Ethernet adapter on P1: none is in a build yet.
- The DAC module's specs: confirm them by measurement.
- Front-end op-amp selection: bandwidth vs noise vs LCSC availability vs JLCPCB basic-part status.
