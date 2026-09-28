# Tentzhen

> An open-source lab instrument built from generic parts, for people who can't afford the real thing.
> Pronounced "TENT-zhen" (like "tension").

Tentzhen is a fully open, low-cost alternative to boards like the Red Pitaya: a two-channel
digitizer and signal generator built on a generic ECP5 FPGA board, cheap ADC/DAC modules, and a
custom analog front end. Target cost: roughly $150–200 in parts. The goal is to give anyone a
capable, verifiable instrument, and to document every step so others can build and improve it.

## Principles (apply to every decision)

1. **Open first.** Open hardware, open gateware, open toolchains. Prefer parts and tools anyone can buy and inspect.
2. **Verify everything.** No claim about hardware behavior is true until measured on the bench. Simulation first, measurement always.
3. **Cheap, but not at the cost of quality.** Commodity parts from AliExpress; quality-critical parts from LCSC or authorized distributors (see Sourcing).
4. **Everything scriptable.** Every instrument function must be controllable and loggable from code.
5. **Rust wherever possible.** See Language Policy.
6. **Document for peers.** Write as if someone with little money and a lot of curiosity will follow along.

## Architecture

- **FPGA board:** Colorlight i9 + extension board (Lattice ECP5 45F, 8 MB SDRAM, 2× Ethernet PHY, on-board DAPLink, 6× dual PMOD).
- **SoC:** LiteX with a VexRiscv RISC-V soft core, LiteEth (Ethernet streaming), LiteDRAM (SDRAM capture buffer).
- **Inputs:** 2× AD9226 modules (12-bit, up to 65 MS/s), one per channel, on PMOD headers.
- **Outputs (phase 5):** dual-DAC module (AD9767-class) with an FPGA waveform-generation core.
- **Analog front end:** custom PCB: input protection, switchable attenuation, AC/DC coupling, buffer amplifier, anti-alias filter.
- **Host:** Rust services for ingest (into DuckDB), a SCPI server, and an MCP server. Scope UI via an ngscopeclient driver, later possibly native egui.

Note: 8 MB SDRAM means the Colorlight is **not** Linux-capable. The soft core runs bare-metal Rust or a small RTOS.

## Language Policy

| Layer | Language / tools |
|---|---|
| Gateware (FPGA logic) | LiteX / Amaranth (Python). Rust HDLs are too immature. |
| Gateware tests | cocotb + Verilator |
| Soft-core firmware (VexRiscv) | Rust (`riscv32` target; check state of LiteX PAC/HAL crates) |
| Pico firmware (bridges, policy enforcement) | Rust on Tock OS |
| Host software (ingest, SCPI, MCP, CLI) | Rust (tokio, duckdb crate) |
| Analog simulation | ngspice |
| PCB | KiCad |
| Build reproducibility | Nix |

Python is allowed only for gateware, cocotb tests, and throwaway exploration. Anything long-lived gets rewritten in Rust.

## Toolchain (all open source)

Yosys (synthesis), nextpnr-ecp5 (place and route), Project Trellis (ECP5 bitstream), ecpdap / openFPGALoader (programming), Verilator + cocotb (simulation), ngspice, KiCad, Rust stable + riscv target, Nix.

## Sourcing Policy

**AliExpress (commodity, low risk):** Colorlight i9 + ext board (Muse Lab official store only), AD9226 modules (buy 3, keep best 2), DAC module, SMA/BNC connectors and cables, 10x probes, wire, headers, general-purpose passives.

**LCSC or authorized distributors (quality-critical):** front-end op amps, precision resistors/capacitors in the signal path, LM4040 voltage reference, low-jitter oscillators, signal relays, Pico boards, anything security-related.

**JLCPCB:** front-end PCB fabrication and SMD assembly (using LCSC parts), so the board is reproducible by anyone from published files.

**Incoming QA:** every received part is tested against genuine references and logged in DuckDB (part, seller, order, measurements, pass/fail). Publish seller quality data with the build.

## Phases and acceptance criteria

1. **Bring-up.** LiteX + VexRiscv on the Colorlight; Rust "hello" over UART; Etherbone reachable from the host.
2. **First capture.** One AD9226 into on-chip RAM with an edge trigger; read out and plotted from a Rust host tool. Capture core passes cocotb tests before hardware.
3. **Two channels + streaming.** SDRAM buffering, continuous Ethernet streaming, samples landing in DuckDB with full metadata.
4. **Instrument interface.** SCPI server (pyvisa-compatible) and an ngscopeclient driver.
5. **Signal generator.** DAC module + DDS/arbitrary waveform core.
6. **Front end v1.** KiCad design, ngspice-verified (bandwidth, noise, overload), fabricated and assembled at JLCPCB.
7. **Characterization.** Measure noise floor, ENOB, bandwidth, and timebase accuracy. Publish an honest spec sheet.
8. **First application.** Ultrasonic sonar (transmit chirp, capture echo, compute range).

Realistic expectations: ~20–30 MHz usable bandwidth and more noise than a Red Pitaya on front-end v1. That is fine. Publish real numbers.

## Calibration references (no expensive gear required)

- DC accuracy: LM4040 precision reference (genuine, from LCSC/DigiKey).
- Timebase: crystal oscillator of known frequency.
- Edges and timing: Pico PWM output.
- Noise and distortion: clean audio-range sine from a sound card.

## Safety and Permission Rules (Claude must follow these)

Never do the following without explicit approval from the user in the current session:

- **Place orders or spend money** (LCSC, AliExpress, JLCPCB, anything). Building carts and BOMs is fine.
- **Write irreversible settings:** OTP memory, eFuses, secure-boot keys, flash protection bits on any device.
- **Command a power supply or load** beyond the limits in `lab/limits.toml` (create it before any power automation). Default ceilings: 3.6 V and 200 mA for Pico/3.3 V circuits.
- **Flash firmware to a device** that is not listed as a lab target.
- **Expose any service** outside the isolated lab network, or add internet access for instruments.

Always:

- Keep mains voltage out of scope entirely. No designs or instructions involving mains wiring.
- Treat model-generated code as unverified until tested in simulation and then on hardware.
- Log every hardware action (what, when, parameters, result) to the lab's append-only log.

## Security and Verifiability

- Signed git commits (SSH signing).
- Reproducible builds via Nix for gateware, firmware, and host tools.
- Pin all dependency versions; record toolchain versions in each build artifact.
- Hash-chained, signed logs for measurements and hardware actions (keys eventually in a TROPIC01 secure element).
- Instruments live on an isolated lab network with no internet access.

## Reuse before reinventing

- LiteX / LiteX-Boards (Colorlight i9 support exists).
- LiteScope for in-FPGA debugging.
- Red Pitaya's open FPGA code for capture/trigger structure (check license before copying).
- ThunderScope's open design files as a reference for the analog front end (check license).
- ngscopeclient for the UI.

## Related lab infrastructure (separate repos, same principles)

- Programmable supply: Kungber PSU → DPS5005 running OpenDPS → Tock-based Pico bridge enforcing limits.
- DIY electronic load: MOSFET + op amp + MCP4725 DAC + ADS1115, Pico-controlled with thermal cutoff.
- Parts/sensor database: TOML source files → SQLite/DuckDB, with LCSC part numbers.

## Conventions

- Measurements: SI units in data; store raw ADC codes alongside converted values.
- Every experiment gets a record: hardware revision, gateware hash, firmware hash, setup notes, results.
- Docs live in `docs/`, written for someone following along on a budget.

## Open questions

- Current state of Rust crates for LiteX-generated SoCs.
- AD9226 module variants: logic levels, input stage, clock requirements. Characterize on arrival.
- Best-available DAC module on AliExpress; confirm specs.
- Front-end op amp selection (bandwidth vs. noise vs. LCSC availability).
