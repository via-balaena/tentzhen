# Prior art

What already exists for each part of Tentzhen, who built it, and what we take from it. We build
with existing building blocks where they are not the point, and make friends with the people who
made them (`vision.md`, principle 8). A proposal for a new piece starts here, and searches again.

## How this was searched

- **When and how:** 2026-09-30. Five searches in English, one per part of Tentzhen (instruments
  from cheap parts, grading claims by evidence, safety below an AI agent, lab records, projects
  that combine several), then three in Chinese (below). The web-search budget ran out during the
  Chinese searches, so a later search would reach further.
- **How the pages were read:** most through a tool that summarises a page before it is read. One
  such summary was contradicted by the paper's own text. A source marked **re-read** was read in
  its own text; the rest are as summarised, so re-read one before resting anything on it.
- **Not seen:** forums (EEVblog, volt-nuts), paywalled standards (ISO/IEC 17025, ISO 13849-1, ASTM
  F3269) and papers, private repos. The English searches ran on a US search engine.
- **What it can show:** what these searches found. It cannot show that nothing closer exists.

## The short answer

Nothing found combines Tentzhen's parts: instruments from cheap modules checked on arrival,
gateware and limit logic with proofs or bounded checks, a grade on every published number derived
from its evidence, an AI agent held to limits enforced below it, and the repo as the lab's system
of record. Each part exists on its own, and several were built in 2026.

- **Instruments from cheap parts** are well trodden. What we found no one doing is putting
  evidence behind their numbers.
- **Grading claims** has close relatives in test-equipment spec sheets and in metrology, where a
  maker or a lab asserts how a number is known. Tools that derive a grade from evidence and fail
  when it is missing exist for software; we found none for hardware specs.
- **Safety below the agent** has a name, runtime assurance, and a literature. Of the systems we
  found that let an AI drive lab hardware and say where their limits are enforced, each enforces
  them in software above the instrument, or in the instrument's own firmware, whose protection
  settings sit on the same link the commands use. One paper asks for verified firmware below the
  agent and has no code.
- **Lab records** each exist: hash-chained logs, flash gates for agents, measurement records. We
  found none that ties them to claims.
- **In Chinese,** there are many more open instruments, supplies and loads, several with one unit's
  accuracy measured and published, and agents on instruments from RIGOL and others. The pattern is
  the same: at most one unit's measurements behind some of the numbers, and every limit read in
  the software or firmware being commanded.

## Instruments from cheap parts

| project | what it is | shared with Tentzhen | different | source |
|---|---|---|---|---|
| AD9226 builds on GitHub | 27 repos on the cheap AD9226 module: hobby instruments, SDRs and student projects, one from China's national student electronics contest (全国大学生电子设计竞赛, `19-E-NUEDC`); `amin005/skywave_SDR` pairs it with an ECP5 | the same module, often with an FPGA | no evidence behind their specs seen in their descriptions | GitHub repository search for "AD9226", **re-read** |
| Red Pitaya STEMlab 125-14 | the class instrument #1 aims at | two channels, FPGA, SCPI; an MCP server for it exists | lab-instrument prices (€649 for the Gen2 PRO Z7010); schematics not open; no evidence on its specs; calibrated at the factory | redpitaya.readthedocs.io |
| PSLab (FOSSASIA) | a low-cost multi-instrument for education | the budget and teaching mission; corrects cheap parts in software against an ADS1115 | a microcontroller at 2 MS/s, no FPGA | pslab.io, FOSSASIA blog |
| ThunderScope | an open 4-channel FPGA oscilloscope | LiteX gateware; one spec says how it was set ("a current measured value with margin applied") | $1,099–1,299, custom boards | crowdsupply.com, github.com/EEVengers |
| Haasoscope Pro | an open USB oscilloscope | open, aimed at students | $999; numbers without method | crowdsupply.com |
| Sinara Zotino (M-Labs) | a lab-grade DAC card | **each number is footnoted to a datasheet or to the issue where it was measured**, with the setup | lab price, not cheap modules | m-labs.hk sinara datasheet 5432 |
| Glasgow revD | an interface multitool | its supply is "hardwired to turn off" on over- or undervoltage: a limit below software | not an instrument bench | crowdsupply.com |
| tinySA, NanoVNA | cheap RF instruments | a built-in self-test; a large community | clones of varying quality | tinysa.org |

None of seven open instrument repos has a SymbiYosys file (`.sby`) on its default branch: Glasgow,
ThunderScope and its LiteX gateware, Haasoscope Pro, Red Pitaya, red-pitaya-notes and LibreVNA.
The same code search found them in ZipCPU's `wbscope`, so it could see them. Other formal flows
were not searched for.

## Grading claims by evidence

| framework | what it grades | how the grade is set | source |
|---|---|---|---|
| Rohde & Schwarz spec sheets | each number: "meas." (measured on samples), "typ.", "nom." (not tested in production), "n. trc." (tested, not traceable) | asserted by the maker; an unmarked number is warranted, the strongest | FSWP spec sheet |
| Keysight spec guidelines | warranted specs, with guardbands, against characteristics "not necessarily verified on all units" | asserted by the maker | Keysight 5991-1732 |
| Gene Ontology evidence codes | each annotation must carry a reference and an evidence code; ND means no data | chosen by a curator | geneontology.org |
| OpenFastTrace; SLSA verification summaries | coverage of each requirement; an artifact's supply-chain level | **derived from the evidence present, and a tool fails when it is missing** | github.com/itsallcode/openfasttrace, slsa.dev |
| PTB Digital Calibration Certificate | each calibration result: value, expanded uncertainty, coverage factor, unit | issued by an accredited lab, signed | ptb.de |
| Self-Verifying Measurement Records | every number in a paper, bound by its content hash to the observation and check behind it, in an append-only log a verifier audits offline | derived; for GPU benchmarks, not instruments | arXiv 2606.27934, **re-read** |

The spec-sheet labels are the same idea on the same surface, asserted rather than derived. The
tools that derive a grade from evidence are for software. Self-Verifying Measurement Records is the
nearest in spirit.

## Safety below an AI agent

**The concept has a name.** Runtime assurance, or the Simplex architecture (Sha, 2001; ASTM
F3269): an untrusted complex controller, a trusted safety monitor that watches it, and a recovery
function the monitor switches to. The enforcer is the monitor and turning the output off is the
recovery. It is also a reference monitor (Anderson, 1972), which must be impossible to bypass,
impossible to tamper with, always invoked, and small enough to verify. Industrial safety standards
separate a safety system from the control system it guards (IEC 61511), and ISO 13849-1 asks that
a safety function be reset only by a deliberate manual action (clause 5.2.2, read in a secondary
source).

| system | where its limits are enforced | verified? | source |
|---|---|---|---|
| Duke's LLM agent on ARTIQ, for a trapped-ion experiment | an MCP proxy, "the single enforcement point" | none described | arXiv 2606.27231, **re-read** |
| LabMCP | a connector on the host checks each request against limits the user sets; for bench supplies and other lab instruments. Each connector is labelled "simulated" until someone confirms it on a real instrument | "not certified safety systems" | github.com/K-Dense-AI/lab-instrument-mcps, **re-read** |
| Anthropic's Model Hardware Standard | its driver writes a reference file of "what safety limits will be enforced"; where the driver runs is not said | not said | anthropic.com, 2026-08-27, **re-read**; a preview for applicants |
| Safe-SDL | asks for limits "on dedicated hardware with verified firmware" | proposed | arXiv 2602.15061, **re-read**; a framework paper with no code |
| LAP, an agent-to-instrument protocol | leaves the instrument's physical limits to its maker, below the protocol | no | arXiv 2606.03755, **re-read** |
| agentic-hil | a gate on the host for flashing and serial; one configuration kept "outside the repository, out of reach of the agent's own file tools" | none described | github.com/agentic-hil/agentic-hil, **re-read** |
| RoboGuard, Safe-ROS | a planner layer; a ROS node checked with Dafny | Safe-ROS: yes | GitHub, arXiv 2511.14433 |

## Lab records

| tool | closest part | different | source |
|---|---|---|---|
| agentic-hil | an agent flashes boards only through a gate; a SHA-256 audit chain | no claims or measurements | github.com/agentic-hil/agentic-hil, **re-read** |
| OpenHTF (Google) | a measurement with units, a validator and pass or fail, per device and station | per run, not per action; no chain; no meter accuracy | github.com/google/openhtf |
| Doorstop | a link stores the fingerprint of what it points at, and turns suspect when that changes | software requirements | doorstop.readthedocs.io |
| eLabFTW; Mnemosyne | lab notebooks anchored by RFC 3161 timestamps; Mnemosyne also chains its audit log | no instruments or claims | doc.elabftw.net, github.com/ArturRuppel/electronic_labbook |
| RO-Crate Process Run Crate; W3C PROV | a standard shape for "this instrument, this input, this result, this agent, this time" | no tamper evidence, no grades | researchobject.org, w3.org |
| labgrid | drives boards, power and flashing | keeps no history, so a log would wrap it; Python | labgrid.readthedocs.io |

## Chinese-language sources

Searched in Chinese on 2026-09-30: OSHWHub (立创开源硬件平台, JLC's open-hardware platform,
through its own API), Gitee, GitHub, and Bilibili video titles. CSDN, Zhihu, elecfans, EEWorld,
21ic and Baidu blocked the search, so reviews posted there were not seen.

**Instruments.** Nothing found pairs the AD9226 with a Colorlight board and LiteX. On OSHWHub,
"AD9226" returns one project, a sensor board, and "ECP5 示波器", "LiteX" and "Colorlight" return
none.

| project | what it is | shared with Tentzhen | different | source |
|---|---|---|---|---|
| LogicPi dual-channel FPGA oscilloscope (逻辑派双通道数字示波器, greentor) | a Gowin FPGA on JLC's own board, an 8-bit AD9288 at 100 MS/s, a DAC output | FPGA capture, a generator channel, calibration per range | its calibration table is set by hand for each unit; a standalone screen, not a networked host; LGPL-3.0 | oshwhub.com/greentor |
| Muse Lab (wuxx) | the Colorlight i5, i9 and i9plus pinouts, LiteX demos; sells the boards | **our FPGA board** | – | github.com/wuxx/Colorlight-FPGA-Projects, Apache-2.0, **re-read** |
| Sipeed SLogic | a logic analyser; its driver for ngscopeclient was being written in September 2026 | host software we might share | – | github.com/sipeed |
| DreamSourceLab DSView | open software for their analysers and scopes | – | last commit 2024-11-05 | github.com/DreamSourceLab/DSView |

**Supplies and loads.** Several open projects publish measured accuracy, each from one unit.
None found publishes variation across units or checks cheap modules on arrival, and every safety
limit read lives in the same firmware that is being commanded.

| project | shared with Tentzhen | different | source |
|---|---|---|---|
| ESP32 electronic load 2.0 (ESP32电子负载仪2.0) | **the same MOSFET, op amp and MCP4725 control as our load**; notes that the MCP4725 cannot output 0 V | measures with an INA226; GPL-3.0 | oshwhub.com/FJ956391150 |
| S-ELO electronic load (climbsnail) | publishes its accuracy before and after calibration, with a calibration helper board | firmware limits; GPL-3.0 | oshwhub.com/climbsnail |
| XS1 mini load (flyn) | measured its DAC's drift with supply voltage and corrected it | CC BY-NC-SA 3.0 | oshwhub.com/flyn |
| RT300-MKV supply (XACT) | one unit's ripple, setting error and trip time published; Modbus "for automated testing" | limits in the module's own firmware; CC BY-NC-SA 4.0 | oshwhub.com/XACT |
| Power Ultra supply | one unit checked against a calibrated 5½-digit meter | CC BY-NC-SA 4.0 | oshwhub.com/eda_jfdyucwo |

Many of the most-viewed supplies and loads are licensed CC BY-NC-SA, which forbids commercial use:
read the licence before reusing a design.

**The DPS5005's own protection sits on the link the commands use.** A community Modbus library for
the DPS5005 reads and writes its over-voltage and over-current settings (registers `0x52` and
`0x53`), and a register map of Riden's RD6006 lists the same settings (registers 82 and 83). If the
module accepts those writes, whatever holds the link can raise them, so they are no limit below the
agent; that has not been tried here. Sources: github.com/lambcutlet/DPS5005_pyGUI and
github.com/Baldanos/rd6006, both **re-read**.

**Agents on instruments.**

| system | where its limits are enforced | source |
|---|---|---|
| RIGOL UVerse and its agent, Tron (普源精电, launched 2026-07-30, in beta in mainland China) | the host: reads and setting changes run directly; high-voltage output, reset and firmware updates ask first, and the user may answer "永久同意", permanent consent for that workspace | docs.openuverse.com, **re-read** |
| UNI-T 优利德 "小优" | a voice assistant inside the oscilloscope's firmware; its limits are not documented | uni-trend.com.cn |
| alwaysmy/instrumentControl | an MCP server for eight instruments, including a Beijing Dahua DH1766 supply: a blocklist of commands, confirmation gates and a JSONL audit log on the host; its tools can set the supply's over-voltage limit; its own audit is candid about what was not tried on hardware; no licence | github.com/alwaysmy/instrumentControl |
| WaveBench | host gates: an operation contract, read-only mode, and a preflight check against limits in `wavebench.toml`; MIT | github.com/Scaxlibur/WaveBench |
| ChemAgents (中国科大, USTC's robotic chemist) | a prompt of expert rules before the robot's code runs; no layer of limits visible | github.com/pic-ai-robotic-chemistry/ChemAgents |
| Uni-Lab-OS (深势科技 and Peking University) | host drivers; GPL-3.0 | github.com/deepmodeling/Uni-Lab-OS |

**Evidence and verification.**

| item | what it shows | source |
|---|---|---|
| Siglent (鼎阳) datasheets | every number is 技术指标 (guaranteed, measurement uncertainty included), 典型值 (typical: 80% of units at about 25 °C, not guaranteed) or 标称值 (nominal: the design value) | siglent.com datasheets |
| RIGOL (普源) performance-verification guide | for each spec, a limit, a procedure, the reference instrument, and a record table with a pass column, filled in by hand | supportcn.rigol.com, DS1000Z-E guide |
| GB/T 6592-2010 | China's adoption of IEC 60359, the standard for expressing the performance of measuring equipment, which that vocabulary comes from; only its catalogue entry was readable | std.samr.gov.cn |
| rIC3 (Institute of Software, Chinese Academy of Sciences) | a hardware model checker in Rust; upstream SymbiYosys lists it as an engine for both `bmc` and `prove` | github.com/gipsyh/rIC3, BSD-3, and YosysHQ/sby #367, **re-read** |
| 一生一芯 (ysyx), a national chip-design course | teaches SymbiYosys, bounded against unbounded checks, and injecting a bug to show a check can fail; its prose also calls a passed bounded check a proof, the difference our grades keep | ysyx.oscc.cc |
| XiangShan (香山) Deterload | Nix, so two builds of a workload come out identical | github.com/OpenXiangShan/Deterload |
| Asterinas vostd | Verus proofs in CI, with proofs an LLM helped write labelled `AI-assist` | github.com/asterinas/vostd |
| 数码之家 (mydigit) reviews | cheap meters and supplies measured against their claims, often without naming the reference instrument | mydigit.cn |
| OSHWHub's CW32 voltmeter course | a two-point calibration against a TL431, with the error table from before calibration published | oshwhub.com/geng_yan_wang |
| PanGucheng/ISE_prj | a diagnostics plan for ADS1115 and MCP4725 boards whose rules are close to ours: every measured value is entered by a person, and a changing ADC code is not accuracy unless the input and the reference were measured | github.com/PanGucheng/ISE_prj |

**Standards.** A draft national standard on lab safety monitoring from the Ministry of Emergency
Management (《实验室安全监测与智能管控通用要求》, comments closed 2026-08-07) sets a boundary
between people and AI, cuts power through smart breakers on serious events, and hashes each batch
of data with SHA-256. The Cyberspace Administration's opinion on AI agents (May 2026) asks for
"规则内嵌、行为围栏", rules built in and fences around behaviour. Neither was found to ask for
limits held below the agent in verified firmware (read through summaries).

## What we take from it

Each is a candidate, to be proposed where it lands:

1. **Name the enforcer as runtime assurance,** and grade it against the four properties of a
   reference monitor (`architecture.md`; plan step 9).
2. **An agent clearing a trip departs from ISO 13849-1's manual reset.** It is Jon's choice
   (2026-09-30); `lab/limits.toml` should say it departs, and from what.
3. **Speak the spec-sheet language readers know.** Map the grades onto IEC 60359's vocabulary
   (GB/T 6592 in China), which makers use: guaranteed, typical, nominal, and Rohde & Schwarz's
   "meas.". Add what Tentzhen lacks: a claim that holds across many units, not one unit measured.
   A verification procedure per spec, as RIGOL publishes, is what a measurement record per spec
   could look like.
4. **Record a measurement as a calibration certificate does:** value, expanded uncertainty,
   coverage factor, and how the meter's calibration is traced.
5. **Anchor the lab log's head outside the repo** with an RFC 3161 timestamp, so a rewrite of the
   whole chain shows.
6. **Keep the list of what an agent may do out of the agent's reach,** as agentic-hil does:
   `lab/targets.toml` is a file an agent can edit.
7. **Check sigrok's driver for the DPS5005's stock firmware** before flashing OpenDPS, which
   changes the module's flash protection (plan step 10). Either way the stock firmware's protection
   settings sit on the same link as the commands, so the enforcer stays.
8. **A link from a claim to a record could store the record's sha256,** as Doorstop does, so an
   edited record flags its claims.
9. **Emit an RO-Crate Process Run Crate for each measurement record,** and name measurement fields
   as OpenHTF does, so other tools can read them.
10. **Try rIC3 as the `prove` engine** for the first formal core (`roadmap.md`, Phase 0), pinned to
    one version, as `proven` requires.

## Building blocks we use

| project | for | where |
|---|---|---|
| LiteX, VexRiscv | the digitizer's SoC and soft core | `scope.md`, `CLAUDE.md` (planned) |
| Amaranth, SymbiYosys, Yosys, nextpnr, Project Trellis | gateware cores, their proofs, and the bitstream | `verification.md` (planned) |
| Tock | the enforcer's OS; upstream supports the RP2350, and Jon's `via-balaena/tock` branches add drivers | `verification.md`, "Tock" (planned) |
| Kani | the enforcer's checks | `firmware/enforcer` |
| DuckDB | the warehouse | `crates/warehouse` |
| Muse Lab's Colorlight pinouts | the digitizer's FPGA board | `scope.md` (planned) |
| machcnz's AD9226 module notes | the module's clipping defect and its fix | `scope.md` |
| OpenDPS | the DPS5005's firmware | `lab/limits.toml`, `parts/dps5005.toml` |
| Raspberry Pi debugprobe | the debug probe's firmware | `builds/debug-probe/v1.toml` |
| jlcparts | part data without an LCSC key | `roadmap.md` (planned) |

## People worth writing to

The authors of the closest work: agentic-hil and LabMCP (agents on benches), Duke's ARTIQ group and
Safe-SDL's authors (safety below the agent), the authors of Self-Verifying Measurement Records
(evidence on every number), and M-Labs (Sinara's evidence-footnoted datasheets). In China: Muse Lab
(our FPGA board), greentor (LogicPi's analog front end), the ESP32 load's author (our load's
topology), the S-ELO and XS1 authors (calibration, published), RIGOL's UVerse team (agents on
instruments, with confirmation on the host), the authors of instrumentControl and WaveBench, rIC3's
authors, the 一生一芯 course, and the reviewers at 数码之家. And the projects we build on, upstream first
when we find a bug.
