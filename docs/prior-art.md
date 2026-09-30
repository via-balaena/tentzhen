# Prior art

What already exists for each part of Tentzhen, who built it, and what we take from it. We build
with existing building blocks where they are not the point, and make friends with the people who
made them (`vision.md`, principle 8). A proposal for a new piece starts here, and searches again.

## How this was searched

- **When and how:** 2026-09-30. Five searches in English, one per part of Tentzhen (instruments
  from cheap parts, grading claims by evidence, safety below an AI agent, lab records, projects
  that combine several), then three in Chinese (below).
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

- **Instruments from cheap parts** are well trodden. What is new is the evidence behind them.
- **Grading claims** has close relatives in test-equipment spec sheets and in metrology, where a
  maker or a lab asserts how a number is known. We found none where the grade is derived from
  evidence a tool checks.
- **Safety below the agent** has a name, runtime assurance, and a literature. Every system we
  found that lets an AI drive lab hardware enforces its limits in host software. One paper asks
  for verified firmware below the agent and has no code.
- **Lab records** each exist: hash-chained logs, flash gates for agents, measurement records. We
  found none that ties them to claims.

## Instruments from cheap parts

| project | what it is | shared with Tentzhen | different | source |
|---|---|---|---|---|
| AD9226 builds on GitHub | 27 repos on the cheap AD9226 module: hobby instruments, SDRs and student projects, one from China's national student electronics contest (全国大学生电子设计竞赛, `19-E-NUEDC`); `amin005/skywave_SDR` pairs it with an ECP5 | the same module, often with an FPGA | no evidence behind their specs seen in their descriptions | GitHub repository search for "AD9226", **re-read** |
| Red Pitaya STEMlab 125-14 | the class instrument #1 aims at | two channels, FPGA, SCPI; an MCP server for it exists | €649; schematics not open; no evidence on its specs; calibrated at the factory | redpitaya.readthedocs.io |
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
| Duke's LLM agent on ARTIQ, for a trapped-ion experiment | an MCP proxy on the host, "the single enforcement point" | no | arXiv 2606.27231, **re-read** |
| LabMCP | a connector on the host checks each request against limits the user sets; for bench supplies and other lab instruments | no | github.com/K-Dense-AI/lab-instrument-mcps, **re-read** |
| Anthropic's Model Hardware Standard | its driver writes a reference file of "what safety limits will be enforced"; where the driver runs is not said | not said | anthropic.com, 2026-08-27, **re-read**; a preview for applicants |
| Safe-SDL | asks for limits "on dedicated hardware with verified firmware" | proposed | arXiv 2602.15061, **re-read**; a framework paper with no code |
| LAP, an agent-to-instrument protocol | leaves the instrument's physical limits to its maker, below the protocol | no | arXiv 2606.03755, **re-read** |
| agentic-hil | a gate on the host for flashing and serial; one configuration kept "outside the repository, out of reach of the agent's own file tools" | no | github.com/agentic-hil/agentic-hil, **re-read** |
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

To be filled from the search in Chinese.

## What we take from it

Each is a candidate, to be proposed where it lands:

1. **Name the enforcer as runtime assurance,** and grade it against the four properties of a
   reference monitor (`architecture.md`; plan step 9).
2. **An agent clearing a trip departs from ISO 13849-1's manual reset.** It is Jon's choice
   (2026-09-30); `lab/limits.toml` should say it departs, and from what.
3. **Speak the spec-sheet language readers know.** Map the grades onto "meas.", "typ." and "nom.",
   and add what Tentzhen lacks: a claim that holds across many units, not one unit measured.
4. **Record a measurement as a calibration certificate does:** value, expanded uncertainty,
   coverage factor, and how the meter's calibration is traced.
5. **Anchor the lab log's head outside the repo** with an RFC 3161 timestamp, so a rewrite of the
   whole chain shows.
6. **Keep the list of what an agent may do out of the agent's reach,** as agentic-hil does:
   `lab/targets.toml` is a file an agent can edit.
7. **Check sigrok's driver for the DPS5005's stock firmware** before flashing OpenDPS, which
   changes the module's flash protection (plan step 10).
8. **A link from a claim to a record could store the record's sha256,** as Doorstop does, so an
   edited record flags its claims.
9. **Emit an RO-Crate Process Run Crate for each measurement record,** and name measurement fields
   as OpenHTF does, so other tools can read them.

## Building blocks we use

| project | for | where |
|---|---|---|
| LiteX, VexRiscv | the digitizer's SoC and soft core | `scope.md`, `CLAUDE.md` (planned) |
| Amaranth, SymbiYosys, Yosys, nextpnr, Project Trellis | gateware cores, their proofs, and the bitstream | `verification.md` (planned) |
| Tock | the enforcer's OS; upstream supports the RP2350, and Jon's `via-balaena/tock` branches add drivers | `verification.md`, "Tock" (planned) |
| Kani | the enforcer's checks | `firmware/enforcer` (PR #29) |
| DuckDB | the warehouse | `crates/warehouse` |
| OpenDPS | the DPS5005's firmware | `lab/limits.toml`, `parts/dps5005.toml` |
| Raspberry Pi debugprobe | the debug probe's firmware | `builds/debug-probe/v1.toml` |
| jlcparts | part data without an LCSC key | `roadmap.md` (planned) |

## People worth writing to

The authors of the closest work: agentic-hil and LabMCP (agents on benches), Duke's ARTIQ group
and Safe-SDL's authors (safety below the agent), the authors of Self-Verifying Measurement Records
(evidence on every number), and M-Labs (Sinara's evidence-footnoted datasheets). And the projects
we build on, upstream first when we find a bug.
