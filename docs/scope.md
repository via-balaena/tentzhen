# Instrument #1: two-channel digitizer and signal generator

Research date for everything below: 2026-09-28, unless a line says otherwise.

## Architecture

- **FPGA board:** Colorlight i9 v7.2 + extension board. FPGA LFE5U-45F-6BG381C; SDRAM
  M12L64322A (8 MB, 32-bit); W25Q64 flash; 2× B50612D gigabit Ethernet PHYs; 25 MHz clock. The
  extension board adds a DAPLink (JTAG + USB-CDC), USB-C, HDMI and six 2×15 headers. P1 carries
  the Ethernet pairs, power and ground, and no FPGA I/O. P2–P6 carry 98 FPGA I/O, each header laid
  out as two PMOD-compatible sites plus four more I/O, a ground, and 5 V on pin 23.
  (github.com/wuxx/Colorlight-FPGA-Projects, `colorlight_i9_v7.x.md` and
  `schematic/i5-i9-extboard.pdf`; the headers read 2026-10-01)
- **Ethernet needs an adapter:** the extension board has no RJ45 jacks, and P1 carries the PHYs'
  bare pairs. Of the i5, which uses the same extension board, Tom Verbeure wrote that the module
  "only contains the transceivers, not the Ethernet transformers or RJ45 connectors" ("Getting
  Started with ECP5 FPGAs on the Colorlight i5 FPGA Development Board", tomverbeure.github.io,
  2021-01-22); that the i9 is the same is `unknown`, as its schematic is not public. Kazumoto
  Kojima's i5ether is an adapter for P1, in kazkojima/colorlight-i5-tips, with no license file, so
  we design our own (`roadmap.md`, Phase 0). Until it is built, the host reaches the board through
  its DAPLink's USB-CDC serial port.
- **SoC:** LiteX with a VexRiscv soft core, LiteEth (Ethernet streaming), LiteDRAM (SDRAM capture
  buffer). Custom cores — capture, trigger, DDS — in Amaranth so they can carry formal
  properties (`verification.md`).
- **Inputs, tier 2:** 2× AD9226 modules (12-bit, up to 65 MS/s), one per channel, on headers P2 and
  P3 by jumper wire. Each module has a 2×10 header: 14 signals, 5 V and ground. They are the cheap
  way in, and what the capture gateware is brought up on.
- **Inputs, tier 3:** one TI ADS4245 (two channels, 14-bit, up to 125 MS/s) on the front-end PCB,
  which plugs straight onto P2 and P3, with no wires. How the i9 reads it and what clocks it: "The
  tier-3 ADC", below. Why this chip, and what else exists: `prior-art.md`, "Faster ADCs".
- **Outputs (phase 5):** a dual AD9767 module in the AN9767's pinout (32 signals, 5 V and ground,
  on P4 and P5) with a waveform-generation core.
- Which header carries what, and why: `builds/digitizer/v1.toml`.
- **Analog front end:** custom PCB — input protection, switchable attenuation, AC/DC coupling,
  buffer amplifier, anti-alias filter, and from tier 3 the ADC and the oscillator that clocks it.
- **Host:** Rust services for ingest (DuckDB), a SCPI server and an MCP server. UI via an
  ngscopeclient driver, later possibly native egui.

The soft core does not run Linux; 8 MB of SDRAM is reserved for the capture buffer. It runs
bare-metal Rust or Tock.

## Against the reference

| | Red Pitaya STEMlab 125-14 | Tentzhen #1, tier 2 (modules) | Tentzhen #1, tier 3 (front-end PCB) |
|---|---|---|---|
| ADC | LTC2145-14 | 2× AD9226 | ADS4245 |
| channels in / out | 2 / 2 | 2 / 2 (outputs in phase 5) | 2 / 2 |
| sample rate | 125 MS/s | up to 65 MS/s (AD9226 rating) | up to 125 MS/s (ADS4245 rating) |
| resolution | 14-bit | 12-bit | 14-bit |
| the ADC's SNR, typical | 73.1 dBFS at 5 MHz | 69 dB at 31 MHz | 73.4 dBFS at 20 MHz |
| bandwidth | DC–60 MHz | 20–30 MHz usable — a target, `unknown` until measured | set by the front end, below 62.5 MHz (half the sample rate) |
| input range | ±1 V / ±20 V, 1 MΩ ∥ 10 pF | the module's | set by the front end |

Red Pitaya figures: redpitaya.readthedocs.io, `developerGuide/hardware/ORIG_GEN/125-14`; its ADC
and DAC: RedPitaya/Documentation at 5c39e718, the same page's "Components" (the DAC is the AD9767,
as ours is). SNR rows from each ADC's data sheet, read 2026-10-02: LTC2145-14 (21454314fa, typical
at 25 °C), AD9226 (Rev. B, its front page), ADS4245 (SBAS533E, typical at 25 °C, with the LVDS
interface).

Expect more noise than a Red Pitaya on front end v1. That is fine. Publish real numbers.

## The tier-3 ADC

Chosen 2026-10-02, from sources read that day. Its figures are the makers' and our arithmetic on
them; none is measured.

- **Its outputs.** In parallel CMOS mode the ADS4245 puts out 28 data lines and a clock (`DA[13:0]`,
  `DB[13:0]`, `CLKOUT`) at 1.8 V: its output supply, DRVDD, is 1.7 V to 1.9 V. Four more lines set
  it up (`RESET`, `SCLK`, `SDATA`, `SEN`). That is 33 of the 38 I/O on P2 and P3, which leaves P4
  and P5 to the DAC, as `builds/digitizer/v1.toml` wires it. (TI SBAS533E, revised February 2023:
  pin tables and recommended operating conditions.)
- **1.8 V into the i9.** In Lattice's mixed-voltage table a bank accepts 1.8 V inputs only when its
  VCCIO is 1.8 V, and accepts 1.2 V inputs at every VCCIO (FPGA-TN-02032 1.4, Table 4.2). The
  i9's VCCIO is neither published nor measured. So the gateware reads the ADC as LVCMOS12, which
  "can also be set as fixed threshold inputs independent of VCCIO" (FPGA-DS-02012 3.4, section
  2.14.1). Table 3.12 gives LVCMOS12's thresholds as 0.35 and 0.65 of VCCIO and accepts up to
  3.465 V; its note 4 says that with mixed voltages "VIL/VIH follows the I/O signaling standard",
  so we read the thresholds as 0.42 V and 0.78 V. No Lattice text names this use, and none gives
  that receiver's speed: it is `unknown` until the loopback test passes (Open questions).
- **If it fails,** the ADS4245's DDR LVDS mode is the documented route. The ECP5 takes LVDS inputs
  on its left and right banks only, with VCCIO at 2.5 V or 3.3 V (FPGA-DS-02012 3.4, Tables 3.11,
  note 4, and 3.13). It needs 15 pairs, seven per channel and the clock. P2 to P5 carry 24 true
  pairs on those banks, only 8 of them within P2 and P3 (Lattice's ECP5U-45 pinout CSV, rev. 3.0,
  against the extension board's pinout), so the ADC would spread over all four headers and the DAC's
  pins would be planned again.
- **The sampling clock.** Clock jitter alone limits SNR to −20·log10(2π·f_in·t_jitter) (SBAS533E,
  section 9.2.2.4, Equation 2), so holding 73 dB at a 30 MHz input allows 1.2 ps rms. The ECP5's
  PLL is specified only peak to peak: up to 100 ps of period jitter at outputs of 100 MHz and
  above, measured "with clean reference clock with no additional I/O toggling" (FPGA-DS-02012 3.4,
  Table 3.23). Its rms jitter on the i9 is not measured. So the ADC runs from a crystal oscillator
  on the front-end board, and the FPGA captures on the ADC's `CLKOUT`, brought to a clock input:
  P2 pin 16 (J20, PCLKT2_0) or pin 13 (L20, PCLKT3_1).
- **Capture depth.** Two channels at 125 MS/s carry about 440 MB/s packed at 14 bits, 500 MB/s in
  16-bit words. LiteX's i9 target runs the SDRAM on its system clock, 60 MHz by default
  (`--sys-clk-freq`, `--sdram-rate 1:1`; litex-boards `colorlight_i5.py` target at 87307deb), which
  peaks at 240 MB/s on 32 bits. Whether LiteDRAM runs this SDRAM twice as fast is `unknown`, so
  until it does, a capture fits in block RAM. The LFE5U-45 has 108 blocks of 18 kbit (FPGA-DS-02012
  3.4, Table 1.1): at one sample per 18-bit word, about 55,000 samples per channel, less what the
  SoC takes. The Red Pitaya's standard buffer is 16,384 samples (RedPitaya/Documentation,
  acquisition commands, at 5c39e718). Gigabit Ethernet carries at most 125 MB/s, so a capture is
  triggered, held, then sent.

## Closer to the reference without parts

Gateware and host work, no extra parts (sources read 2026-10-02):

- **Equalization, per unit.** Each unit's own measured response sets a correction filter that
  flattens the front end up to the anti-alias cutoff and no further: past it, correcting "would
  just amplify noise" (Haasoscope Pro, `software/FIR_CALIBRATION_README.md`). The Red Pitaya has
  such a filter in its gateware, `red_pitaya_dfilt1`, a "Filter to equalize input analog chain".
- **Averaging for bits.** Averaging samples trades rate for resolution where the input carries
  enough noise to dither it. The Red Pitaya's decimation does it: "Each sample is the average of
  skipped samples if DEC > 1". What it gains on our units is `unknown` until measured.
- **Equivalent-time sampling of our own stimulus.** Where the DAC drives the input (a Bode plot, a
  step response), shifting the stimulus's phase against the sampling clock between repeats samples
  the waveform at steps finer than one clock period. It does nothing for a single-shot signal.
- **sin(x)/x interpolation** on the host, for display near half the sample rate.

Not pursued: two AD9226s interleaved per channel for 130 MS/s. It needs the two chips' gain,
offset and timing matched in calibration, and neither the dual module that offers it nor any open
AD9226 project we found publishes one. (Haasoscope Pro measures and corrects the timing between
its own interleaved boards, `software/calibration.py`.) CycleScope weighed it and did not adopt it
(`prior-art.md`, "Faster ADCs").

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
  connected), and a USB-C source drives no VBUS until it detects a sink's pull-down on CC1 or CC2
  (USB Type-C spec 1.3, sections 4.5.2.2.7 and 4.5.2.2.8.2, as quoted in Zephyr's
  `subsys/usb/usb_c/usbc_tc_src_states.c`), so a USB-C to USB-C cable may give it no power. Power
  it from a USB-A port.
- At least one module variant is specified for ±5 V (10 Vpp) input scaled to 1–3 V, with a 5 V
  supply and 3.3 V logic (manuals.plus/ae/1005005576645194). Input stages differ between
  sellers, so each unit is characterized on arrival.
- **Availability:** both Amazon listings for the Colorlight i9 read "Currently unavailable" on
  2026-09-28. AliExpress availability is `unknown`: its pages answered automated fetches with a
  captcha on 2026-10-01, so a person checks.

## Calibration references (no expensive gear required)

- DC accuracy: LM4040 precision reference, from an authorized distributor (LCSC or DigiKey).
- Timebase: crystal oscillator of known frequency.
- Edges and timing: Pico PWM output.
- Noise and distortion: clean audio-range sine from a sound card.

## Reuse before reinventing

- LiteX / LiteX-Boards: the i9 is `--board i9` on the `colorlight_i5` target
  (`litex_boards/targets/colorlight_i5.py`, read 2026-09-28).
- LiteScope for in-FPGA debugging.
- Red Pitaya's open FPGA code for capture/trigger structure and its input equalizer,
  `red_pitaya_dfilt1` (RedPitaya-FPGA at 728a4f37, read 2026-10-02). Its LICENSE is BSD 3-clause
  text, but the directories it says it covers (`Applications`, `fpga` and others) are none of this
  repository's, so ask Red Pitaya before copying.
- Haasoscope Pro's FIR calibration (MIT) and ngscopeclient's de-embed filter (scopehal, BSD-3) for
  equalizing each unit (read 2026-10-02).
- ThunderScope's open design files as a front-end reference (check the license).
- ngscopeclient for the UI.
- Upstream Tock's `litex_vexriscv` chip, if the soft core runs Tock.

## Open questions

- Current state of Rust crates for LiteX-generated SoCs.
- Whether the i9 reads 1.8 V CMOS through LVCMOS12 inputs at 125 MHz ("The tier-3 ADC"):
  `unknown`. Settled on the i9 alone, before the front end is designed: one header pin drives a
  pseudo-random pattern through a resistor divider down to 1.8 V into an LVCMOS12 input on
  another, and the gateware counts errors at 125 Mb/s (`roadmap.md`, Phase 1).
- Whether LiteDRAM runs the i9's SDRAM fast enough for deep captures at 125 MS/s, and which speed
  grade of M12L64322A the i9 carries.
- Whether LCSC stocks the ADS4245 for JLCPCB's assembly: a person checks. Whether the 12-bit
  ADS4225 fits the same footprint: one data sheet and family table cover both, but pin for pin is
  not checked.
- Whether 65 MS/s survives 10 cm jumper wires (tier 2) — signal integrity is `unknown`. Scoped
  (github.com/wavius/Scoped) clocks an AD9226 module at 25 MHz over 20 cm ones, and its README
  shows a demo over UART at a 50 kHz acquisition rate, as USB kept dropping.
- Whether one USB port powers the i9 and three modules: the extension board's fuse rating is not
  printed, and the draw is not measured.
- The Ethernet adapter on P1: designed in Phase 0 (`roadmap.md`), not yet in a build.
- The DAC module's specs: confirm them by measurement.
- Front-end op-amp selection: bandwidth vs noise vs LCSC availability vs JLCPCB basic-part status.
