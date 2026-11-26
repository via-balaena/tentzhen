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
- **Checking parts from any source** has industry standards, closed databases and open tests for
  single chips. We found nothing that publishes measured results by seller.
- **Sorting parts by kind** has the class letters schematics label parts with (R, C, U), and a
  free procurement code, UNSPSC, that fits neither our boards nor publishing.
- **Faster ADCs** in the Red Pitaya's class (two channels, 14-bit, 125 MS/s) are catalogue chips
  from TI and Analog Devices, and most put out 1.8 V logic. The tricks around a cheap ADC,
  equalizing and averaging, are open in Red Pitaya's and Haasoscope Pro's code; calibrating two
  interleaved AD9226s is in nothing we found.
- **A cheaper FPGA board,** the EBAZ4205, a used miner's board with a Zynq-7010, already carries
  AD9226 builds, made with Vivado. A U-Boot fork for it sets up its DDR3 from a generated register
  file, copyright Xilinx; GenZ, the open generator of such files, does not cover DDR3 yet.
- **Hardwired neural networks** (project #3) are well trodden on Xilinx parts with vendor tools,
  and done on an ECP5 with open tools (pocket-bnn) and on Tiny Tapeout chips. Low-rank binary
  factors on an FPGA, and one network compared hardwired and in block RAM, are in nothing we
  found.
- **A chip's layout** for a hardwired network has been done on Tiny Tapeout: a ternary MLP for
  MNIST with its weights constant-folded into the logic, on a 2×2 tile, and a binary one on a 1×2.
  Printed-electronics papers build MLPs as circuits for one trained model and judge them by area.
  Low-rank binary factors laid out in silicon are in nothing we found. Tiny Tapeout's GitHub
  Action runs LibreLane in Docker on GitHub's computers, so a layout needs no LibreLane on the lab
  computer.
- **An open fab** (project #4) exists: by August 2026 there were ten Hacker Fabs, all in
  universities, on open tools. One team of eight students at CMU made NMOS chips of 952
  transistors with 5 µm gates (Hacker Fab's March 2026 update). The scarcest input,
  crucible-grade quartz, has a known substitute, synthetic silica, already used on crucibles'
  inner faces. No purity was found for making it from sand by the water-glass route.
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
| Scoped (wavius) | an oscilloscope over USB or UART: an AD9226 module on Muse Lab's iCESugar-Pro, an ECP5 board | the same module on an ECP5, wired with 20 cm jumpers, its clock at 25 MHz from the FPGA | one channel; its demo ran over UART at a 50 kHz acquisition rate, as USB kept dropping; no license file | github.com/wavius/Scoped, **re-read** |

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

## Checking parts from any source

Searched 2026-10-01 and 2026-10-02, for `vision.md`'s principle 3 (parts from anywhere, judged by
incoming QA): standards for parts of unknown origin, open tools that test parts on arrival, and
anything that publishes measured results by seller. GitHub searches for "aliexpress test results",
"seller quality database", "fake parts database", "incoming inspection electronic components",
"counterfeit detection electronic components" and "fake mosfet" found nothing, nor did Chinese
searches for 假芯片, 翻新芯片 and 元器件 检测 假货. GitHub's repository search matches names, descriptions and
topics, not READMEs. Not seen: the paywalled standards' texts, the closed databases' contents,
Reddit, YouTube, and Chinese forums.

| work | closest part | different | source |
|---|---|---|---|
| SAE AS6171A, with AS5553E | tests for parts whose chain of custody is unknown (AS6171A), and avoiding and detecting counterfeits (AS5553E) | for organisations, and paywalled. AS6171A's scope: "No amount of testing can confirm an item as authentic", and it "is not intended to be used to assess quality or reliability issues" | saemobilus.sae.org, AS6171A revised 2018-04-18, **re-read** (its scope) |
| IDEA-STD-1010-B; CCAP-101 | inspecting parts bought on the open market; certifying independent distributors | paid; CCAP-101 covers only new parts never installed, so not salvaged ones | idofea.org; ERAI's glossary, quoting CTI |
| ERAI; GIDEP | databases of counterfeit reports and suppliers | closed: ERAI to established businesses, GIDEP to US and Canadian government bodies and their contractors | erai.com, gidep.org |
| cpetrich/counterfeit_DS18B20 | over 1000 DS18B20 probes and chips from more than 70 vendors, measured and sorted into families | results by chip family, not by seller. "All of the probes bought on ebay and AliExpress contained counterfeit DS18B20 sensors", and "Neither origin nor price were indicators of sensor Family" | github.com/cpetrich/counterfeit_DS18B20 (Apache-2.0), **re-read** |
| The AVR transistor tester (Kübbeler's and Reschke's firmware) | identifies and measures two- and three-pin parts | one part at a time, no record of the source; EUPL-1.2 | github.com/kubi48/TransistorTester-source |
| flipper-fake-chip-detector | reads the factory ID registers of 82 I2C chips | ID registers only, and some chips have none | github.com/hleserg/flipper-fake-chip-detector (MIT) |
| Zeptobars | die shots of fake chips beside real ones | names a source only as "ebay" and the like; needs decapsulation | zeptobars.com (CC BY 3.0) |
| InvenTree | a test result per stock item, linked to its supplier | an inventory, with results kept private | github.com/inventree/InvenTree (MIT) |

We found nothing that publishes measured results by named seller. The closest, the DS18B20 study,
names its sources and reports by family; the databases that do report by supplier are closed.

## Sorting parts by kind

Searched 2026-10-02, for how the parts page groups what the bench needs: a classification an
electronics builder already reads, that the repo and the site may publish. Not seen: the standards
themselves (IEEE 315-1975, ASME Y14.44-2008, IEC 81346-2:2019), and ECLASS.

| work | closest part | different | source |
|---|---|---|---|
| Reference designator class letters (IEEE 315-1975, ASME Y14.44-2008) | the letters a schematic labels parts with: R resistor, C capacitor, U integrated circuit, A assembly, W wire | one level. The standards are unread; the list we read "does not necessarily comply with standards" | en.wikipedia.org, "Reference designator", revision 1367154339, **re-read** |
| UNSPSC | a four-level code (segment, family, class, commodity) for anything bought | made for purchasing: 315 of its 158,464 rows are electronic components. In our reading, the bench's 30 rows fall in four segments, its five boards and modules share one code (32101502, printed circuit assemblies), and 14 of the 30 fit only roughly. The last terms published at unspsc.org give use "for User's individual use", without the right to "redistribute, compile or create derivative works" or to "combine the Materials with any other coding system" without written permission; its FAQ says embedding the codes needs "an addendum to the terms & conditions". UNDP, its owner, has run it since 2025. UNDP's terms of use, for everything downloaded from its sites unless special terms say otherwise, give "the User’s personal, non-commercial use, without any right to resell or redistribute them or to compile or create derivative works therefrom". unspsc.org redirected to an unrelated site on 2026-10-02 | undp.org/unspsc and its codeset UNv26.0801 (xlsx, sha256 `10f889fde3bfb4f73d9eca76b69e24e0ba009aa5684aba2d28f5703ce532d752`), **re-read**; undp.org/copyright-terms-use, read 2026-10-02, **re-read**; the unspsc.org terms and FAQ as the Internet Archive kept them on 2025-06-10, **re-read**; Wikipedia's "UNSPSC", revision 1335491644, for who runs it |
| InvenTree; Part-DB | part categories in a tree | each user makes up the tree | inventree.org; docs.part-db.de |

We use the letters, one level, in our own words (`Class` in `crates/records`). A deeper tree waits
until the list is long enough to need one.

## Faster ADCs

Searched 2026-10-02, for a cheap ADC that brings instrument #1 nearer the Red Pitaya, and for what
gateware and calibration add to a cheap one: makers' data sheets and product pages, GitHub in
English and Chinese, arXiv, and EEVblog teardowns. The makers' 1,000-unit list prices were read on
their own pages and are not recorded here: prices come from the BOM tool, with a date
(`vision.md`). Not seen: shops' prices (CLAUDE.md, shop data), the Chinese makers other than
Ruimeng (芯炽, 成都华微, 中科芯 and others were not reached), ALINX's and other Chinese module
makers' catalogues, and paywalled papers.

| work | closest part | different | source |
|---|---|---|---|
| TI ADS424x and ADS422x | the ADS4245: two channels, 14-bit, 125 MS/s, SNR 73.4 dBFS at 20 MHz; 1.8 V parallel CMOS or DDR LVDS outputs. The 12-bit ADS4225 shares its data sheet | outputs at 1.8 V: Lattice's tables name 1.8 V inputs only at a VCCIO of 1.8 V, so the i9 reads them by an unnamed route or as LVDS (`scope.md`, "The tier-3 ADC") | TI SBAS533E, **re-read** |
| LTC2145-14 | the Red Pitaya STEMlab 125-14's ADC: two channels, 14-bit, 125 MS/s, SNR 73.1 dBFS at 5 MHz | — | RedPitaya/Documentation at 5c39e718, "Components", **re-read**; data sheet 21454314fa, **re-read** |
| ADI's AD9648 family | duals from 10 to 16 bits on one footprint (the AD9608, AD9628 and AD9648 among them), 1.8 V CMOS or LVDS; Digilent's Zmod Scope 1410-125 uses the AD9648, and its controller IP is MIT | — | AD9648 data sheet, Rev. C, **re-read** (pin compatibility); Digilent/vivado-library, `ip/Zmods/ZmodScopeController/src/ConfigADC.vhd`, and its MIT license, **re-read** |
| HMCAD1511, HMCAD1520 | 8-bit at 1 GS/s, or 12-bit at 2× 320 MS/s; ThunderScope's LiteX gateware receives them | serial LVDS; ThunderScope's receivers are built on a Xilinx primitive, IDELAYE2 | analog.com data sheets; EEVengers/thunderscope_litex (its BSD-2-Clause license **re-read**) |
| Ruimeng MS5134 | the one Chinese part found: two channels, 14-bit, 125 MS/s, serial LVDS at 1.8 V | its data sheet was not reached | relmon.com, a product card on its home page |
| Mass-market oscilloscopes | the Rigol DS1000Z uses an HMCAD1511; FNIRSI's 1013D and 1014D an AD9288 | 8-bit, so salvage gives no route to 14 bits | EEVblog teardown threads; pecostm32/FNIRSI-1013D-1014D-Hack (no license file) |
| qzkydz AD9226_2CH module | two AD9226s; a jumper picks two channels at 65 MS/s, or one at 130 MS/s, interleaved | its manual describes no calibration between the two chips | the vendor's manual in Scaxlibur/CycleScope at b6a2fbfb, **re-read** |
| CycleScope (Scaxlibur) | an AD9226 oscilloscope that weighed interleaving to 130 MS/s and did not adopt it: "需要校准通道增益、偏置和相位失配，复杂度和不确定性高" (it needs the channels' gain, offset and phase mismatch calibrated; complex and uncertain) | — | `CycleScope设计报告初稿.md`, section 2.1 (Unlicense), **re-read** |
| Haasoscope Pro | equalizes each unit with an FIR filter fit to a square wave, and stops at the anti-alias cutoff, since correcting past it "would just amplify noise"; has interleaved modes | GS/s, on its own boards | drandyhaas/HaasoscopePro at b7762c54 (MIT), `software/FIR_CALIBRATION_README.md`, **re-read** |
| Red Pitaya's gateware | `red_pitaya_dfilt1`, a "Filter to equalize input analog chain"; decimation that averages: "Each sample is the average of skipped samples if DEC > 1" | — | RedPitaya-FPGA at 728a4f37; RedPitaya/Documentation at 5c39e718, **re-read** |
| ngscopeclient's de-embed filter | removes a network's effect by applying its S-parameters in reverse, its gain capped by a "Max Gain" setting, 20 dB by default | on the host, after capture | ngscopeclient/scopehal at 8e3ebb11 (BSD-3-Clause), `DeEmbedFilter.cpp`, **re-read** |

## A cheaper FPGA board

Searched 2026-10-02, for the EBAZ4205 as instrument #1's FPGA board: a GitHub repository search for
"ebaz4205" (the 30 with the most stars), the board's community repository, LiteX's files for it, the
boot code of one Linux build for it, and the open tools for its chip. Why it was not taken:
`scope.md`, "Architecture". Not seen: shops' listings and prices (CLAUDE.md, shop data), forums,
Chinese-language sites, and the board's schematic: the pin count below comes from a project's pin
table.

| work | closest part | different | source |
|---|---|---|---|
| EBAZ4205 | the control board of Ebit's E9+ bitcoin miner, sold used: a Zynq-7010 (two ARM cores beside an FPGA), 256 MB of DDR3, and 10/100 Ethernet (an IP101GA) | three headers at 2.0 mm pitch, not 2.54 mm, with 42 FPGA I/O, 14 on each; they are its only power input (5–12 V) until a diode, D24, is fitted | xjtuecho/EBAZ4205 at 05cdb450 (MIT), README, **re-read**; the pin count from kushpet's table (next row) |
| kushpet/ebaz4205-sdr | a 0–30 MHz SDR receiver: one AD9226 module, and optionally one DAC904 module, on an EBAZ4205. The FPGA runs a downconverter at 60 MS/s, decimating to 1 MS/s I/Q, and the ARM side streams it over TCP | one input, and one output with the DAC; Vivado and Vitis; no license file | at 18ec936a, README, `docs/EBAZ4205-ADC-DAC.md` and `hardware/constraints/ebaz4205.xdc`, **re-read** |
| guido57/EBAZ4205_Spectrum | a 0–32 MHz spectrum viewer: an AD9226 on an EBAZ4205 | Vivado, Vitis and PetaLinux | at a7123471 (MIT), its description and README, **re-read** |
| LiteX's EBAZ4205 target | the board in LiteX: a clock, two LEDs and a UART on the FPGA side, with Vivado as the default toolchain | asking for the ARM side (`cpu_type` "zynq7000") raises `NotImplementedError` | litex-hub/litex-boards at 10debf14, `platforms/ebaz4205.py` and `targets/ebaz4205.py`, **re-read** |
| embed-me's U-Boot and Buildroot for the EBAZ4205 | U-Boot's first-stage loader calls `ps7_init()`, which sets up the board's DDR3 from `ps7_init_gpl.c`, a register file whose header says it is "automatically generated", copyright Xilinx, under the GPL. A Buildroot config builds that loader and Linux 4.19 | the bitstream comes "from the Vivado build" (its README) | embed-me/u-boot, branch v2019.07-ebaz4205 at ec30ee60, `board/embedme/ebaz4205/` (`ps7_init_gpl.c`, `Makefile`) and `arch/arm/mach-zynq/spl.c`; embed-me/ebaz4205_buildroot at c807c1fd (GPL-3.0), `configs/zynq_ebaz4205_defconfig` and README; all **re-read** |
| openXC7 and GenZ | open tools for the Zynq-7000: openXC7 places and routes Zynq-7000 FPGAs among others, and GenZ generates the ARM side's setup code. One openXC7 demo uses both, on a PYNQ-Z1's Zynq-7020 | GenZ's code notes "ddr: DDR3, unsupported yet", and that demo builds its boot loader with `-DNODDR` | openXC7/toolchain-installer at 0cb31563, README; openXC7/demo-projects at ff232be4, `ps7-blinky-digilent-pynqz1/`; regymm/GenZ at 842a8f64 (MIT), `zynq7000.py`; all **re-read** |

## Hardwired neural networks

Searched 2026-11-21, for project #3 (`littlebit.md`). A search agent ran 4 web searches, 9 arXiv
queries and about 50 GitHub repository searches, 3 of them in Chinese, and read the sources it
names; a row marked **re-read** was then read in its own text. GitHub's repository search matches
names, descriptions and topics, not READMEs: pocket-bnn turned up for "ulx3s bnn" and not for "ecp5
bnn", so more may exist. These found nothing: "littlebit fpga", "low-rank binary fpga", "hardwired
weights fpga", "colorlight neural", "colorlight mnist", "bnn ecp5", and arXiv abstracts with
low-rank, binary and FPGA together. Not seen: GitHub's code search, Gitee, IEEE and ACM papers,
Tiny Tapeout's chips beyond one run's list, and Chinese-language sites beyond those 3 queries.

| work | closest part | different | source |
|---|---|---|---|
| marph91/pocket-bnn | a binary network with its weights and thresholds as constants in the logic, on an ECP5-85F (the ULX3S) with the open toolchain: "Accuracy on Mnist: 75 %", 41 % of the chip's slices | binarized weights, no low-rank factors and no block-RAM arm; VHDL; last pushed 2021-07-16 | at 04e092f (MPL-2.0), README and Makefile (`nextpnr-ecp5 --85k`) **re-read**; "constants" in its source as the agent read it |
| skyline624/ecp5-inference | an int8 language model, `stories260K`, on a Colorlight i9 v7.2 (LFE5U-45F-6BG381C), with openFPGALoader, iverilog and cocotb | its weights are stored and read into block RAM, not hardwired; no license file, so it is read to learn and nothing is copied | at 7e79fa7, README (in French) **re-read**; how it stores weights as the agent read it |
| LogicNets, PolyLUT, NeuraLUT, LUTNet | weights become LUT truth tables, with MNIST among their tasks | Vivado (LUTNet: Vivado HLS 2016.3) | Apache-2.0 for the first three, BSD-2-Clause for LUTNet, as the agent read them |
| DWN and dwn2rtl | lookup-table networks trained in PyTorch (DWN), turned into Verilog and testbenches with no vendor tools (dwn2rtl) | lookup tables, not a factorized weight matrix; DWN's training needs CUDA | alanbacellar/DWN at 2b456ac (MIT), dwn2rtl/dwn2rtl at 3130d59 (MIT), licenses **re-read**; the rest as the agent read it |
| FINN, hls4ml | compilers from a trained network to FPGA logic | FINN needs Vivado or Vitis and targets PYNQ and Alveo boards; hls4ml has no Lattice backend | Xilinx/finn (BSD-3-Clause), fastmachinelearning/hls4ml (Apache-2.0), as the agent read them |
| rejunity/tt10-lgn-mnist | an MNIST classifier as a logic-gate network on Tiny Tapeout chips: "10K logic gates", "0.8mm x 0.4mm" die area, "97.60% test accuracy", on SKY130 and IHP's 130 nm | a learned network of gates, not weights | at 950a653 (Apache-2.0), README **re-read** |
| MNIST on small FPGAs, weights in memory | gigwegbe/bnn-goboard (an iCE40-HX1K, weights in a `.mem` ROM, MIT) and semaphoric775/bnn_mnist (a Gowin Tang Nano 9k, block RAM, no license) | stored weights | as the agent read them |
| Taalas HC1 | "hard-wired with Llama 3.1 8B": "17K tokens/sec per user", from "a custom 3-bit base data type" | a language model in custom silicon; AMD reached "a definitive agreement to acquire Taalas" on 2026-08-06 | taalas.com, "The path to ubiquitous AI" (2026-02-19), **re-read**; AMD's press release of 2026-08-06, **re-read** |
| LittleBit | the method: W ≈ diag(h) U_sign diag(ℓ) V_signᵀ diag(g), Dual-SVID initialization, Residual Compensation, down to 0.1 bits per weight on language models | written for language models, with FP16 scales, its speed measured on an NVIDIA A100; its code is CC BY-NC 4.0, for non-commercial use, so none of it is copied | arXiv 2506.13771v5 (NeurIPS 2025), sections 3.2 and 3.3 and Appendix D, **re-read**; SamsungLabs/LittleBit at 42d658b, LICENSE **re-read** |

Building blocks for project #3: Muse Lab's Amaranth platform for the i9
(wuxx/Colorlight-FPGA-Projects, Apache-2.0); SocialJax's environments and three checkpoints
(Apache-2.0); Concordia's local model backends (Apache-2.0); LibreLane (Apache-2.0) and Tiny
Tapeout for Phase 5.

## Simulations in hardware

Searched 2026-11-23, for project #3's goal of running multi-agent simulations more cheaply a game
step than a GPU (`littlebit.md`, "The goal: cheaper than a GPU per game step").

- **How it was searched.** A search agent ran about 30 web searches, and GitHub repository and code
  searches through `gh`. It read pages through a tool that summarises them.
- **Read in their own text,** through GitHub's API: rial, LoWA and KohakuTPU, and EnvPool's PR #433.
- **Not seen:**
  - ACM's page for FA3C, which refused the request, so FA3C's figures come from search snippets;
  - PufferLib 2.0's PDF;
  - AMD's own release on completing its purchase of Taalas.

Every speed-up and efficiency ratio below is its authors' own, each against a different baseline.

| work | closest part | different | source |
|---|---|---|---|
| FPGA-Gym (NeurIPS 2024, Open-World Agents workshop) | RL environments' steps written in Verilog on a PCIe FPGA board, grid worlds (FrozenLake, CliffWalking, Taxi) among its 8: "4.36 to 972.6×" the fastest software framework | single-agent; the agents stay on the host, behind a Gymnasium interface; Vivado 2018.2; no steps a second or board named on the pages read | neurips.cc/virtual/2024/100961 and Selinaee/FPGA_Gym (MIT), as the agent read them |
| Asseman et al., IBM's Neural Computer (arXiv 2005.04536, 2020) | the Atari console, the image preprocessing and the network all on FPGAs, "about 1 million frames per second" on "432 Xilinx FPGAs" | single-agent, for neuroevolution; no claim to match software bit for bit | as the agent read it |
| FA3C (ASPLOS 2019), FIXAR (DAC 2021), LearningGroup (FPT 2022) | RL on FPGAs: FIXAR's fixed-point training at 2,638.0 IPS/W, 15.4× a CPU and GPU; LearningGroup's multi-agent training on chip at 12.43× a Titan RTX's energy efficiency | all accelerate a training loop on Xilinx parts; none hardwires weights, and none puts the environment beside the policy | arXiv 2102.12103 and 2210.16624, as the agent read them; FA3C from search snippets |
| SocialJax (arXiv 2503.14576v2), its Table 1 | Clean Up with random moves on an A100 and 14 CPU cores: 9.9 × 10⁵ steps a second at 1,024 games, 2.1 × 10⁶ at 4,096 | no figure with a trained policy choosing the moves | as the agent read it |
| JaxMARL, EnvPool, CuLE, XLand-MiniGrid, PufferLib 2.0 | environments on GPUs and CPUs at 10⁶ to 10⁷ steps a second with random moves | the baselines to beat, each on its own hardware | as the agent read them; XLand-MiniGrid's A100 figure and PufferLib from search summaries only |
| craftax.cu (a gist by brycelelbach, 2026-04-28) | Craftax, a JAX environment, hand-written in CUDA: 54,177,909 steps a second at 4,096 environments on an RTX Pro 6000, and training at 3,007,291 against JAX's 107,000 on an RTX 3090 | one author, unreviewed; shows a CUDA port can beat JAX by tens of times, so a GPU's bar isn't fixed | as the agent read it |
| EnvPool's Craftax port (sail-sg/envpool PR #433, merged 2026-08-28) | Craftax's rules and its Threefry streams in C++, with a pinned JAX as the oracle: "State, rewards… and uint8 rendered frames must match exactly", but for "one-to-two-ULP differences caused by… fused multiply-add order in specific combined JAX reset/step graphs" | C++ on a CPU, not hardware; EnvPool's docs now say it uses "EnvPool's native random-number generator" instead | the PR, read in its own text by the agent; EnvPool's license not checked |
| Threefry and Philox in HDL: mdgrape/rial (RIKEN, Chisel, BSD-2-Clause), SDMuhsin/LoWA (Verilog), KohakuBlueleaf/KohakuTPU | rial's `Threefry4x32Generator`, 20 rounds; LoWA's `rn_tf.v`, Threefry-2x32 unrolled to a block a clock; KohakuTPU's Philox-4x32-10 | none claims to match JAX's streams; rial is 4 × 32; LoWA's is 13 rounds, and JAX's 20 (`jax/_src/prng.py`); LoWA has no license, so it is read, never copied; KohakuTPU's license is unstated | read in their own text by the agent |
| HNLPU, "Metal-Embedding" (arXiv 2508.16151v2, 2026-01-13) | weights encoded in the topology of a chip's metal wiring: 249,960 tokens a second and 36 tokens a joule | modelled, not said to be made; a language model | as the agent read it |
| Taalas HC1, checked by others | heise: "independent measurements by third parties are not yet available" (2026-02-21); the demo server's own figures, read by a gist's author, averaged about 14,200 tokens a second | no controlled benchmark by anyone but Taalas; AMD's purchase was reported complete on 2026-11-17, by a secondary site | heise.de/-11185112 and a gist by instax-dutta (2026-07-20), as the agent read them; AMD's release not read |
| QuaRL/ActorQ (arXiv 1910.01055), Deep Binary RL for Scalable Verification (arXiv 2203.05704), Low Precision Policy Distillation (arXiv 1809.09260) | RL policies at 8 bits, binarized, or distilled to low precision | none uses low-rank binary factors, and none measures a compressed policy's return in a multi-agent social dilemma | as the agent read them |

What this means for the plan:

- **Already done:**
  - environments in HDL, single-agent, with the policy on the host;
  - a whole game and its network on FPGAs, single-agent, on Xilinx parts;
  - FPGA trainers and actors;
  - Threefry in HDL;
  - a JAX environment rebuilt with its Threefry streams and matched exactly against a pinned JAX,
    in C++.
- **Not found:**
  - a multi-agent social-dilemma game on one chip with its policy;
  - hardware matched bit for bit with JAX;
  - any of it on open tools and a cheap ECP5;
  - low-rank binary factors in an RL policy, with its return measured;
  - a published cost per game step, policy included, against a GPU.
- **To reuse:**
  - EnvPool's method: a pinned oracle, each field compared exactly, and its warning about fused
    multiply-adds;
  - rial's Threefry structure, should the gateware need one (BSD-2-Clause);
  - Random123's known answers, which `crates/cleanup` checks.

## A chip's layout

Searched 2026-11-21, for Phase 5 of project #3 (`littlebit.md`): what already lays out a hardwired
network on an open process, and where LibreLane can run. Six web searches; about ten GitHub
repository searches; four GitHub code searches over the TinyTapeout organization, whose shuttle
repos hold every tile's datasheet, `info.yaml` and layout figures; and arXiv's API for the
printed-electronics papers. Every row was read in its own text, through GitHub's API or arXiv's,
except where it says otherwise. These found nothing: repository searches for "amaranth
tinytapeout", "amaranth tiny tapeout", "tinytapeout bnn", "tinytapeout binary neural", "tt_um
mnist" and "sky130 binary neural network". Not seen: Tiny Tapeout tiles whose datasheets use none
of the words searched for ("mnist", "neural network", "binary neural"), the papers behind the
printed-electronics abstracts, IEEE and ACM papers, and SiliconCompiler's and OpenROAD Flow
Scripts' own docs.

| work | closest part | different | source |
|---|---|---|---|
| kenchangh/mnist-character-recognition (`tt_um_kenchangh_mnist`, on TTSKY26c) | an MNIST MLP, 64→32→10, whose ternary weights "are baked into the netlist as Verilog constants, so synthesis constant-folds them into plain logic — no RAM, no ROM macros, no weight loading", on a 2×2 sky130A tile. Its integer model scores "80.3 %" on MNIST cut to 8×8 binary pixels. Synthesis gives 2,165 cells for its 2,368 weights (64×32 + 32×10), and its layout has no DRC or LVS errors | ternary weights, no low-rank factors; 64 input bits, not 784 pixels; Verilog; its accuracy is its model's, not a chip's | at 33c215e5 (Apache-2.0); its datasheet, `info.yaml`, `stats/synthesis-stats.txt` and `stats/metrics.csv` in TinyTapeout/tinytapeout-sky-26c, **re-read** |
| Petros-Emmanouilidis/ttnn (`tt_um_petros`, on TTSKY26b) | a binary network with its first layer's weights as constants in the SystemVerilog (`l0_weights`), combinational, on a 1×2 tile; its Python model scores "70%" on its test data | binary, 8 hidden units; scikit-learn's 8×8 digits, not MNIST | at 8b03b852 (Apache-2.0), `src/ttnn.sv` and its datasheet, **re-read** |
| Tiny Tapeout tiles that load their weights | binary or small networks on one tile: `tt_um_BNN` (TTSKY25a), `tt_um_vmm_bnn` and `tt_um_dean_foulds_ai_accelerator` (TTSKY26b), `tt_um_four_bit_nn_accel` (TTSKY26c), `tt_um_gregac_tiny_nn` (TT10) | the weights are loaded from outside and held in flops or shift registers, not hardwired | their datasheets in TinyTapeout's shuttle repos, **re-read** |
| Printed MLPs as bespoke circuits (Kokkinis, Armeniakos, Afentaki, Saglam, Zervakis, Tahoori and others) | MLPs built as circuits for one trained model, "Bespoke (fully-customized) design" (arXiv 2312.17612), judged by area, the measure of research question 5: "for up to 5% accuracy loss up to 8x area reduction" (arXiv 2301.11142) | printed electronics, not silicon standard cells; their full texts are unread, so their flows and per-weight figures are not known here. A web search's summary says 2301.11142's weights are constants in the circuit and that it synthesizes with Synopsys Design Compiler | arXiv 2301.11142, 2303.08255, 2312.17612 and 2412.06542, abstracts **re-read**; the summary as summarised |
| LibreLane | the flow Phase 5 names. Its install guide ranks Nix "(Best)", an AppImage "(Easiest)" on Linux or WSL, then Docker; on Windows it "*requires* WSL2". Its FAQ compares it with OpenROAD Flow Scripts, SiliconCompiler and OpenLane, and lists its open process kits as `sky130`, `gf180mcu` and `ihp-sg13g2` (beta) | the building block itself, not yet in `trusted-base.toml`. Latest releases 3.0.17 (2026-10-26) and 3.1.0.dev6 (2026-11-08) | librelane/librelane at 8494d4fe (Apache-2.0): `docs/source/installation/index.md`, `installation/wsl/_common.md`, `faq.md`, **re-read** |
| Tiny Tapeout's GDS action | hardens a tile on GitHub's `ubuntu-24.04` runners: pip-installs LibreLane, 3.0.14 by default for sky130A, and runs it in LibreLane's Docker image (`--dockerized`). The template's workflow adds Tiny Tapeout's precheck and a gate-level test | lays out a Tiny Tapeout project, with its pins and tile, on GitHub's computers; nothing is installed on the lab computer | TinyTapeout/tt-gds-action at d4783dbe, `action.yml`; tt-support-tools at 0922544e, `project.py` and `tt_tool.py`; ttsky-verilog-template at a42e6662, `.github/workflows/gds.yaml`; all Apache-2.0, **re-read** |
| Amaranth on Tiny Tapeout | virantha/tt10-enigma, `language: "Amaranth"`, hardened on TT10; htfab/tt10-py-checkers, one of whose layers is Amaranth | not networks. Tiny Tapeout's templates are Verilog, one for each shuttle, and one Chisel, none Amaranth | tt10-enigma at 68fa8233 and tt10-py-checkers at cd434374, both Apache-2.0, datasheets **re-read**; TinyTapeout/tinytapeout_www at ce1fff22, `content/hdl/templates/_index.md`, **re-read** |

**The tile.** A sky130A 1×1 tile is 161.00 × 111.52 µm and a 2×2 is 334.88 × 225.76 µm
(tt-support-tools at 0922544e, `tech/sky130A/tile_sizes.yaml`). The FAQ's "about 160x100 um" is
of TT04 to TT10.

## Chinese-language sources

Searched in Chinese on 2026-09-30: OSHWHub (立创开源硬件平台, JLC's open-hardware platform,
through its own API), Gitee, GitHub, and Bilibili video titles. CSDN, Zhihu, elecfans, EEWorld,
21ic and Baidu blocked the search, so reviews posted there were not seen.

**Instruments.** Nothing found pairs the AD9226 with a Colorlight board and LiteX. Searched again
on 2026-10-01, on GitHub (repositories: "AD9226" 27, with "ecp5" 1, with "colorlight", "pmod",
"litex" or "amaranth" 0; "AD9767" 2, with "colorlight" or "ecp5" 0, **re-read**) and Gitee: no
project connects an AD9226 or AD9767 module to a Colorlight board. On OSHWHub,
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

## Open fabs, and chips made outside one

Searched 2026-11-24, for project #4 (`fab.md`): what already makes chips in the open, and what
frees a fab from its scarcest inputs. Ten web searches in English, on a US search engine: open
fabs, Sam Zeloof, open process kits, upgraded silicon, high-purity quartz, float-zone growth,
quartz purification, synthetic silica for crucibles, printed classifiers and PlasticARM. Then
GitHub's API for Hacker Fab's repositories and GitBook, and arXiv's API for two abstracts. Rows
marked **re-read** were read in their own text; the rest are as a search summarised them.

Later the same day, for `fab.md`'s "Improving Hacker Fab's stepper":
- one web search, for levitated wafer stages;
- arXiv's API for FleXstage;
- Hacker Fab's stepper and nanopositioner pages, through GitHub's API;
- CortenForge's crates as published;
- cortenforge-cfsem and brownfield-compute, read in their own repositories;
- Hacker Fab's page of other projectors, and the OpenMLA controller it points to, through
  GitHub's API.

Not seen:
- Hacker Fab's Discord;
- the papers' full texts;
- Chinese-language sources;
- any source on purifying sodium silicate by ion exchange, the water-glass route `fab.md` names.

On 2026-11-25, for `fab.md`'s "Hacker Fab's other tools" and its gradiometer:
- Hacker Fab's GitBook at bd1eef57, still its head, and its organization's repository list,
  through GitHub's API;
- the repository descriptions of SputterOS and of the CMU sputtering control code;
- Klipper's docs at 7558e8c2;
- CortenForge's gradiometer study at 520da08f.

Not seen: the code of SputterOS, UW's mass-flow controller board, the atomic force microscope and
the tube furnace's controller; any source on the state of the art that Jon's "large gap" is
measured against. No web search was made.

| work | closest part | different | source |
|---|---|---|---|
| Hacker Fab | "the first open-source semiconductor fab", whose goals are to "Make a DIY version of every nanofabrication tool" and to "Get there with collaborative open source hardware". "As of August 2026, **ten** Hacker Fabs have been established". Its contribution guidelines' goals include "Low cost", "Create the simplest designs possible", "Minimal danger", "No cleanroom needed" and "Highly sourcable materials". The GitBook holds 35 pages of operating procedures. The March 2026 update reports a team of eight students, "CMU Fab", making over 1,500 NMOS transistors: "952 Transistors per Chip", "94% Yield", "1.26V Threshold Voltage", "5 um gate length" | it builds its own tools, the mains ones among them by its first goal. All ten fabs in its list are at universities, CMU, Case Western, Georgia Tech, IIT Bombay, Ohio State, UBC, UC Irvine, Illinois, Waterloo and Wrocław; its waste procedure for buffered oxide etch names the "HackerFab Wet Chemistry Lab - ESC 232", and no university | hacker-fab/gitbook at bd1eef57 (CERN-OHL-S-2.0): `README.md`, `guides/required-reading-todo.md`, `nmos-capabilities/current-fab-capabilities/march-2026-update/README.md`, `archive/chemicalwastedisposalforboesop.md` and the folder names under `individual-fab-information/`, **re-read**; the organization's repository list through GitHub's API |
| Hacker Fab's stepper (Gonzalez, Bourcart, Wirant and others, with Moneck) | a "$3000 maskless photolithography stepper" built on a DLP projector, with "an optical resolution of under 2 microns" and alignment "with sub-micron precision". It finds fiducials on the chip with RF-DETR, and makes "micron-scale devices such as NMOS transistors". Its software is hacker-fab/stepper (MIT), "Source code for photolithography stepper software" | the nearest thing to `fab.md`'s slicer. Whether it takes a chip layout's layers is `unknown` here | arXiv 2510.15082v2, abstract **re-read**; hacker-fab/stepper at 1640b32d, its description only |
| Sam Zeloof | the Z1 (2018), a PMOS dual differential amplifier, "the first homemade lithographically synthesised microchip"; then Atomic Semi, with Jim Keller, now renamed Fab2 and in Texas, building tools for small fabs | a company; nothing seen says its tools are open | Wikipedia and Tom's Hardware, as summarised |
| PlasticARM (Arm and PragmatIC, Nature 2021) | a 32-bit Arm processor of "18,334 NAND2 equivalent gates" in metal-oxide thin-film transistors on polyimide, at 20 to 29 kHz (the reports differ). Its simpler successor, FlexiCore, used 2,104 devices for a better yield | made in PragmatIC's fab, not printed at home; a processor, not a hardwired network | doi 10.1038/s41586-021-03625-w, The Register, AnandTech and IEEE Spectrum, as summarised |
| Printed Machine Learning Classifiers (Mubarik, Weller, Bleier, Tomei, Aghassi-Hagmann, Tahoori and Kumar, MICRO 2020) | classifiers made bespoke to one trained model in printed electronics. Its bespoke decision trees took about 48.9 times less area and 75.6 times less power on average | decision trees and SVMs, not low-rank networks. The printed MLP papers are in "A chip's layout" | the paper's abstract, as summarised |
| High-purity quartz | two US producers, near Spruce Pine, North Carolina (USGS). CRU says only Sibelco and The Quartz Corp, both there, make the premium grades "in an economically viable and scalable manner". USGS says no economic substitutes exist for most uses, and that cultured quartz is one but costs too much | the deposit sets the limit, not the process (next row) | USGS Mineral Commodity Summaries 2025 and 2026, CRU (2025), as summarised |
| Quartz purification | crushing, flotation, calcining and water quenching, hot acid leaching and chlorination roasting. On Pakistani vein quartz they left only the lattice's aluminium, titanium and lithium, which the authors found hard to remove by current methods. Hot HCl gas exchanges the lattice's alkali ions for hydrogen. One review's targets are under 50 ppm in all, with aluminium under 30 ppm and titanium under 10 | cleans the grains, not the lattice | Minerals (2024); US5037625A; Minerals 14(12) 1225, as summarised |
| Synthetic silica in crucibles | crucible makers' patents put synthetic powder, from silicon tetrachloride, from alkoxides by sol-gel or from sodium silicate, on the inner face and natural quartz outside. The synthetic powder has each metal at 0.1 ppm or less, against 1 ppm or more of aluminium and titanium in natural powder. Sol-gel powder keeps 50 to 100 ppm of silanol, 5 to 30 ppm after vacuum fusing | patents, so the applicants' claims. No purity is given for sodium silicate purified by ion exchange | US 9534318 and US 8844321, as summarised |
| Upgraded metallurgical silicon (Míguez Novoa and others, Ferrosolar) | silicon refined without the Siemens process. It is "a competitive alternative to polysilicon" for multicrystalline solar cells, its purification leaving "a moderately compensated material due to the presence of phosphorous and boron" | made for solar cells, not chips. Its impurity levels in ppm came from a search summary, not the abstract | arXiv 2604.04095v1, abstract **re-read**; the ppm figures as summarised |
| Float-zone growth | crucible-free: a molten zone, heated by RF at about 3 MHz through a one-turn coil in argon, is moved along a rod; repeated passes purify it | industrial; no open or home-built one was found | arXiv 1102.3800 and patents, as summarised |
| Hacker Fab's stepper V2.1 and nanopositioner | V2.1 costs $3,012.13, with an optical resolution of 1 µm, "Vibration susceptibility" of "1.2 µm (when built onto optical table)", 1.04 × 0.58 mm exposures and chips up to 1 cm. Its list of current issues names vibration, the UV-modified projector's stray UV and its pop-ups imaged onto the chip, and the TI evaluation module out of stock everywhere (May 2025). The closed-loop nanopositioner (October 2026, from the Open Micro-Manipulator) reached "~1um resolution but highly affected by vibration", against a goal of 100 nm, with "not enough scientifically rigorous testing" of why. Its encoders are MT6835 modules read over SPI, kept "less than 25 cm" from the control board. The V2.1 page says the camera "may be replaced with a cheaper camera with a smaller image size, by adding a reduction lens", "tested at UCI", and its Thorlabs list gives 15 part numbers with their quantities but no prices | what `fab.md`'s "Improving Hacker Fab's stepper" and its value engineering start from | hacker-fab/gitbook at bd1eef57: `fab-toolkit/lithography-stepper/lithography-stepper-v2.1+-build/README.md` and `current-issues-and-next-steps.md` beside it, `fab-toolkit/nanopositioner/README.md` and `micromanipulator.md` beside it, and `.gitbook/assets/v2.1-thorlabs.csv`, **re-read** |
| FleXstage (Wu and Zhou) | a lightweight magnetically levitated planar stage "towards High-Throughput IC Manufacturing". Its structure and control are designed together, because "the motion control bandwidth is limited by the first structural resonance frequency of the stage" | a research prototype, whose closed-loop results were still to come | arXiv 2309.11735v1, abstract **re-read** |
| Levitated wafer stages | exposure-tool patents describe a wafer stage driven by "a moving-magnet-type planar motor of the magnetic levitation method" | their assignee wasn't seen, so whose machines use it is not known here | US 10133194 and others, as summarised |
| Hacker Fab's other projectors | for $1,300, TI's DLP4710EVM-LC, adapted by a CMU student. For $250, a whole Elegoo Mars 4 DLP resin printer, which "contains a small 0.3" DMD projector": "There is only a 405 nm LED". Its own board takes resin-printer files (.goo), "Thus, instead of sending images via HDMI, the user will have to slice a 1-layer pattern", and they name "designing a custom projector controller board" as a fix | the projector `fab.md`'s i9 would feed | hacker-fab/gitbook at bd1eef57, `fab-toolkit/lithography-stepper/lithography-stepper-v2.1+-build/additional-projector-options.md`, **re-read** |
| OpenMLA's elegoo-mars-4-dlp-controller | "Raspberry pi hat for the elegoo mars 4 dlp UV projector. Implements SPI data transfer for the DLPC1438". It projects "arbitrary **2560x1440** images", "~0.75s to send a full 1440p image" at an SPI clock of about 125 MHz, and breaks out pins for exposure "based on some sensor readout". It is "part of an effort towards an open hardware photolithography system". It says "The easiest and cheapest way to source the projector is to buy a Elegoo Mars 4 DLP"; the projector alone can come from its maker, Anhua, but "expect to pay a premium". Its steps take the printer apart, and the board plugs into "the original elegoo power supply" at a barrel jack | a Raspberry Pi Zero 2 and Python, not an FPGA; the TI controller on the projector's board, the DLPC1438, still drives the micromirrors | openMLA/elegoo-mars-4-dlp-controller at 54562aab (hardware CERN-OHL-W-2.0, software MIT, in its `LICENSE`): `README.md` and `PCB/README.md`, **re-read** |
| CortenForge 0.9.0, and cortenforge-cfsem | sim-core ("MuJoCo pipeline"), sim-soft (finite elements, with "SDF-to-tet meshing"), sim-opt ("simulated annealing, parallel tempering"), and the sim crate as a "differentiable co-design toolkit". cortenforge-cfsem's planar conveyor is "A coil grid that levitates and routes several movers across a flat surface". brownfield-compute's findings list what CortenForge lacks for structures: beam elements, and orthotropic materials | no optics; no modal solver in the crates' descriptions | the crates' `Cargo.toml` descriptions (cortenforge 0.9.0, MIT OR Apache-2.0); YoungMoolahBaby/cortenforge-cfsem at c97cd685, `projects/planar-conveyor/README.md` (no license file, so linked, not copied); YoungMoolahBaby/brownfield-compute at 236796f3, `docs/CORTENFORGE.md`, **re-read** |
| Hacker Fab's sputter control station, and SputterOS | pressure control, "A finite state machine" and "A PI controller", moving from an Arduino Uno to a "Raspberry Pi Pico 2 / RP2350"; its board "Designed and ordered, but not assembled or fully tested". Its gauge's "RS485 path later failed". Later versions "may support ... safety interlocks". Its software is to "align the ACS with SputterOS", BlastermanR/SputterOS (Apache-2.0): "Deterministic C++17 microkernel framework for safety-critical nanofab system control ... multi-tier safety interlocks" | C++17, not Rust; what `fab.md` would compare a Rust version with. The earlier code, BlastermanR/CMU_HackerFab_Sputtering_Control, has no license file, so it is linked, not copied | hacker-fab/gitbook at bd1eef57, `fab-toolkit/rf-sputtering/add-ons-and-wip/sputtering-automation-control-system.md` (last changed 2026-10-18), **re-read**; SputterOS's repository description only |
| Hacker Fab's impedance matcher | a "Working prototype": a "Teensy 4.1" reads "the forward and reflected power signals from an off-the-shelf VSWR meter", averages "N = 300 samples", and tunes by "Coordinate Descent" with a finite-difference gradient. "Maximum Forward Power" 100 W, "~1.2 VSWR" stabilized, "~5 seconds" to match from startup, $636.65; a "maximum peak-to-peak voltage of 200 V". The RF comes from "a 100W Radio transceiver" at 14 MHz | magnitudes only, so it searches; `fab.md`'s version would measure phase too. The matcher's page says 13.56 MHz, the chamber's page 14 MHz | hacker-fab/gitbook at bd1eef57, `fab-toolkit/rf-sputtering/add-ons-and-wip/automated-impedance-matching.md` and `fab-toolkit/rf-sputtering/README.md`, **re-read**; hacker-fab/impedance-matcher's description |
| Hacker Fab's stepper V3 | "Stepper V3 - WIP": a "Piezo nanopositioner" and a "Laser interferometer", Michelson, "593 nm", its photodiode through "a transimpedance amplifier" so that it can "register on an Arduino Nano"; the projector swapped for TI's DLPDLCR3310EVM, at 1080p | last changed 2024-03-15, so older than the nanopositioner page | hacker-fab/gitbook at bd1eef57, `fab-toolkit/metrology-characterization/interferometer.md`, **re-read** |
| Hacker Fab's measurement | "SMU - Analog Discovery": three Analog Discovery 3s. A probe station on bought XYZ stages, against "~$10K to ~$100K" for one on the market, with "a piezo vibration sensor for probe touch detection (work in progress)". Spin-on glass thickness from a spectrometer: "Determine which shape your graph most closely resembles". Furnace oxide from a colour chart: "the color tends to oscillate based on the thickness of oxidation". A profilometer procedure | shape-matching, not a fitted model; `fab.md` proposes the fit, and instrument #1 where an Analog Discovery 3 is used | hacker-fab/gitbook at bd1eef57: `fab-toolkit/smu/smu-analog-discoveries.md`, `fab-toolkit/probe-station.md`, `standard-operating-procedures/spin-on-glass-sop/spin-on-glass-thickness-measurement.md`, `fab-toolkit/tube-furnace/oxidation-chart-and-results.md`, **re-read**; `standard-operating-procedures/profilometer-sop.md`, its name only |
| Hacker Fab's other controllers | repositories for a "Teensy 4.1" quartz-crystal monitor ("to measure thickness of deposition"), "STM32 and Pi code for Raman spectrometer", "Arduino code for piezoelectric nanopositioner", "Arduino and Pi code for OpenFlexure microscope", a GRBL fork, a "UW MFC Controller PCB", "RS-485 code for advanced configuration of the DB-4824 PID controller" for a tube furnace, and "HW design & firmware for a low-cost Atomic Force Microscope" | a controller of its own for each of these, and SputterOS (above) meant to become a shared base; `fab.md`'s one interlock design | the organization's repository list through GitHub's API, descriptions only; `fab-toolkit/rf-sputtering/add-ons-and-wip/qcm-for-rf-sputtering.md`, its first lines |
| Klipper's input shaping (Klipper3d/klipper, GPL-3.0) | "Input Shaping - a technique that can be used to reduce ringing", where "Ringing is caused by mechanical vibrations in the printer due to quick changes of the printing direction"; resonances measured with "ADXL345, MPU-9250, LIS2DW and LIS3DH compatible accelerometers" | for 3D printers in motion, not a stepper that settles before each exposure. GPL-3.0, so its method is taken, not its code | Klipper3d/klipper at 7558e8c2, `docs/Resonance_Compensation.md` and `docs/Measuring_Resonances.md`, their first lines, **re-read** |
| CortenForge's gradiometer study | a "room-temperature MEMS gravity gradiometer": "Conditional GO for the large-anomaly / void niche", "No code written yet". "The bet": "Matching by construction (sub-ppm) + self-gravity from CAD". Out of scope: "etch-process physics, circuit synthesis, DAQ firmware" | a study, not a device; Jon's goal for later (`fab.md`) | via-balaena/CortenForge at 520da08f, `docs/studies/gradiometer/src/00-context.md`, its status table, **re-read** |

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
11. **Say what incoming QA shows, and no more:** a part that passes is fit for what the tests
    checked, never "genuine", as AS6171A's scope says of its own tests (`sourcing.md`, "Incoming
    QA").
12. **Publish incoming-QA results by seller,** which we found no one doing in the open, and offer
    them to the DS18B20 study's author, whose data is sorted by family.
13. **Lay out Phase 5 on GitHub's computers,** as Tiny Tapeout's action does: LibreLane in its
    Docker image, the version and the action pinned. Nix in WSL stays an option for working
    through a failed run by hand, and needs WSL's disk moved off C: first.
14. **Set Phase 5 beside kenchangh's tile:** the same process kit and tile, with its cells per
    weight as the nearest figure research question 5 has.
15. **Read the printed-MLP papers in full** before research question 5 settles how area per
    weight bit is counted.
16. **Build project #4's stepper and NMOS process on Hacker Fab's open designs and procedures,**
    and send back what we add: safety plans, and measurements with their grades.
17. **Read hacker-fab/stepper before writing the slicer** (`fab.md`, "Software").
18. **Ask a Hacker Fab how it shares safety with its university** before the town fab's safety
    plans are written (`fab.md`, "Safety first").
19. **Compare a Rust version of the sputter station with SputterOS** on the same simulated
    chamber, on measures named first (`fab.md`, "Hacker Fab's other tools").
20. **Take input shaping from Klipper's docs,** not its GPL-3.0 code, for the stepper's moves.

## Building blocks we use

| project | for | where |
|---|---|---|
| LiteX, VexRiscv | the digitizer's SoC and soft core | `scope.md`, `CLAUDE.md` (planned) |
| Amaranth, SymbiYosys, Yosys, nextpnr, Project Trellis | gateware cores, their proofs, and the bitstream | `verification.md` (planned) |
| Tock | the enforcer's OS; upstream supports the RP2350, and Jon's `via-balaena/tock` branches add drivers | `verification.md`, "Tock" (planned) |
| Kani | the enforcer's checks | `firmware/enforcer` |
| DuckDB | the warehouse | `crates/warehouse` |
| Muse Lab's Colorlight pinouts and extension-board schematic | the digitizer's FPGA board and its headers | `scope.md`, `builds/digitizer/v1.toml` |
| machcnz's AD9226 module notes | the module's clipping defect and its fix | `scope.md` |
| Kazumoto Kojima's i5ether | an Ethernet adapter for the extension board's P1; no license file, so we link to it and do not copy it | `scope.md` (planned) |
| OpenDPS | the DPS5005's firmware | `lab/limits.toml`, `parts/dps5005.toml` |
| Raspberry Pi debugprobe | the debug probe's firmware, on a Pico 2 (v1) or on Raspberry Pi's Debug Probe (v2) | `builds/debug-probe/v1.toml`, `builds/debug-probe/v2.toml` |
| jlcparts | part data without an LCSC key | `roadmap.md` (planned) |
| Red Pitaya's equalizer and decimation, Haasoscope Pro's FIR calibration, ngscopeclient's de-embed filter | equalizing and averaging each unit | `scope.md` (planned) |

## People worth writing to

The authors of the closest work: agentic-hil and LabMCP (agents on benches), Duke's ARTIQ group and
Safe-SDL's authors (safety below the agent), the authors of Self-Verifying Measurement Records
(evidence on every number), M-Labs (Sinara's evidence-footnoted datasheets), and cpetrich
(counterfeit DS18B20s, measured). In China: Muse Lab (our FPGA board), greentor (LogicPi's analog
front end), the ESP32 load's author (our load's topology), the S-ELO and XS1 authors (calibration,
published), RIGOL's UVerse team (agents on instruments, with confirmation on the host), the authors
of instrumentControl and WaveBench, rIC3's authors, the 一生一芯 course, and the reviewers at 数码之家.
For project #3: kenchangh and Petros Emmanouilidis, whose hardwired networks are on Tiny Tapeout
tiles; FPGA-Gym's authors, whose environments run in Verilog; and the authors of EnvPool's Craftax
port, who matched a JAX environment exactly outside JAX.
For project #4: Hacker Fab's contributors, the authors of its stepper paper, SputterOS's author,
and the impedance matcher's authors.
Kazumoto Kojima, whose i5ether carries no license, so we could build on it only once it has one. And
the projects we build on, upstream first when we find a bug.
