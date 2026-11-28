# The open fab, and the town around it

Project #4 (Jon, 2026-11-24) is "a fully open source ASML/TSMC engineering project". It is built
up to scrappily and in public, to show what is possible.

- **What the fab is for:** it builds computers that resolve conflict before it happens. They are
  the chips of project #3, which run simulations of trained agents more cheaply per game step
  than a GPU (`littlebit.md`, "What it is for" and "The goal").
- **The town:** a town grows around the fab, the way brownfield-compute's town
  (YoungMoolahBaby/brownfield-compute) grows around its data center, and a city like Shenzhen
  grows from there. Tentzhen as a whole is to become "its own simulated shenzhen like city"
  (Jon, 2026-11-25; `vision.md`).
- **Safety comes first:** "the towns safety is 100% the most important thing" (Jon, 2026-11-24).

Nothing here is built. Every step below is a proposal, and its claims are `unknown` until they
earn a grade. What already exists is in `prior-art.md`, "Open fabs, and chips made outside one".

## Safety first

Jon approved these rules on 2026-11-24, rule 3 as it is worded here. No step that handles
chemicals, heat, high voltage, vacuum or pressure starts outside them.

1. **No step starts without its safety plan.** The plan is written before anything is bought. It
   lists:
   - each hazard: chemical, heat, electrical, pressure, vacuum, light and waste;
   - the controls and the protective equipment for each;
   - what to do in a spill, a fire or an exposure;
   - who may do the step;
   - the codes it was checked against, fire, waste, zoning and exhaust, each with the date it
     was read, so that whether the step is legal where it runs can be checked (rule 3).

   Jon approves each plan in the session it is used (CLAUDE.md).
2. **Limits below the agent, as on the bench.** An AI may plan a run and log it. Heaters, pumps,
   high voltage and chemical dispensing sit behind interlocks it can't reconfigure: an
   over-temperature cutoff in hardware, an emergency stop, and fume detection that cuts power
   (`vision.md`, principle 5).
3. **Anything on mains is bought, listed and legal.** That covers hotplates, furnaces, pumps, and
   RF and sputter supplies. Each is listed by a recognised testing lab and used as sold, where it
   is legal to use there, and anything wired in is installed by a licensed electrician. The
   project designs, builds and wires no mains equipment, and writes no instructions for mains
   wiring. No agent switches mains power except through an interlock it can't override (rule 2).
   Hacker Fab's first goal is to "Make a DIY version of every nanofabrication tool". Here the
   tools on mains are bought instead (Jon, 2026-11-24: "id say its ok as long as its legal";
   CLAUDE.md).

   When a bought tool is what stops the next stage of closing a gap, it is raised with Jon:
   whether to engineer and make a version of our own, "a fraction of the cost and meets every
   safety mark as well" (Jon, 2026-11-26). Until Jon decides, this rule holds as written.
4. **Never here:**
   - toxic and pyrophoric gases: silane, phosphine, arsine, diborane and chlorine;
   - chlorosilanes;
   - developers made with TMAH, which is toxic through the skin. A sodium or potassium hydroxide
     developer is used instead.

   Dopants come as liquids spun onto the wafer. Hacker Fab's repositories include work on such
   spin-on dopants (`prior-art.md`).
5. **Hydrofluoric acid comes last, under a plan of its own.**
   - Hacker Fab's transistor process etches oxide with it, as buffered oxide etch.
   - Hacker Fab's procedure for that waste guards against "toxic fluoride exposure".
   - The first steps don't use it at all.
6. **The neighbours' safety too.** Chemical storage, wastewater and exhaust are written up the way
   brownfield-compute writes up water and air: against public limits, with the uncertainty stated.
7. **A town fab brings its own safety office.** All ten Hacker Fabs in its GitBook's list are at
   universities. Its waste procedure for buffered oxide etch names the "HackerFab Wet Chemistry
   Lab - ESC 232", and no university. A fab in a town has no university's safety office behind
   it, so it needs its own. How the Hacker Fabs share that work with their universities is
   `unknown` here.

## What it builds, and why fidelity matters

A simulation predicts a conflict only if its agents act as the trained ones do. That is why
project #3 judges a compressed policy by the behaviour bar, not by score (`littlebit.md`,
"Whether a student plays as the teacher does").

In Clean Up, zapping is the conflict, and the compressed students already get it wrong both
ways. These are `behaviour.py`'s figures, one training run each, and `unknown` (`littlebit.md`):

- **Fewer zaps:** w8-dense-r4 at 300 epochs and w8-all-r4 after one step of DAgger zap 5 to 9
  fewer times a game than the teacher (`littlebit.md`, "Longer training, and one step of
  DAgger"). Hardwired as they are, they would show a more peaceful society than the trained
  agents make.
- **More zaps:** the 20-epoch w8-all-r4 zaps 7.5 more (`littlebit.md`, "A recipe that makes a
  faithful student"). It would show a more violent one.

So the fab builds two kinds of chip, for two purposes:

- **Silicon, for the cost goal.** It starts with open-PDK shuttles, and later uses our own process.
- **Printed, to show what is possible.** These are thin-film transistors on glass or plastic, a
  home-made chip with a trained network built in. They run at kilohertz, so they aren't the cost
  goal.

A p-bit chip, project #6's last step, would be made on the silicon track too (`pbits.md`).

## The steps

### Silicon

| step | what | needs | passes when |
|---|---|---|---|
| 1 | a network hardwired on the i9 | the bench's i9 | now (`littlebit.md`) |
| 2 | our design made on a shuttle, on an open PDK | a shuttle slot, which costs money: Jon's decision | the chip computes what the golden model does |
| 3 | test gear: probe station, microscope, curve tracer | project #1's bench | it measures a bought transistor's curves |
| 4 | lithography: a maskless stepper on Hacker Fab's open design, patterning photoresist on bought wafers | resist and a hydroxide developer, under its safety plan; no acids or furnace | the smallest line it prints, and its overlay error between two layers, measured on a calibration target |
| 5 | NMOS transistors on bought wafers | a furnace, spin-on dopant and hydrofluoric acid, each under its safety plan | measured threshold, leakage and yield |
| 6 | a small chip: a hardwired network of a few hundred to a thousand transistors | step 5's process | it computes what the golden model does |

Hacker Fab's lithography stepper costs about $3,000 and resolves under 2 µm (arXiv 2510.15082).
Its March 2026 update reports NMOS chips of 952 transistors each, with 5 µm gates, a 1.26 V
threshold and 94 % yield, from over 1,500 transistors made by eight students. Both are their
figures, read in their own text (`prior-art.md`).

### Printed

| step | what | passes when |
|---|---|---|
| P1 | thin-film transistors of metal oxide or organic semiconductor, made at a few hundred °C on glass or plastic | measured transistor curves |
| P2 | a tiny classifier hardwired into printed logic, as the printed-ML papers do | it computes what the golden model does |

PlasticARM, a 32-bit Arm processor in metal-oxide thin-film transistors on plastic, shows that
thin-film logic can reach 18,334 gates. It was made in PragmatIC's fab, not printed at home
(`prior-art.md`). Each step's safety plan comes first, as above.

### Materials

These come after the silicon track's step 5, once transistors work on bought wafers:

- **M1, our own silicon.** Float-zone refining of bought metallurgical or upgraded metallurgical
  silicon. It needs no crucible, and the same devices are compared on it and on bought wafers.
- **M2, our own silica.** High-purity silica made from ordinary sand by the water-glass route,
  where the metals are removed while the silica is still a liquid. Its purity is measured by
  ICP-MS at an outside lab. This is the way around the crucible-grade quartz that, by CRU's
  account, only two producers at Spruce Pine make at scale.
- **M3, smelting.** Quartz to metallurgical silicon in an arc furnace, the heaviest step, last.

### Software

A "slicer" turns each layer of a chip's layout into the stepper's exposures, the way a slicer
turns a model into a 3D printer's paths. It would be written in Rust. Hacker Fab's stepper software
(hacker-fab/stepper, MIT) comes first, read before anything is written.

## Improving Hacker Fab's stepper

Jon asked (2026-11-24) whether CortenForge could make an improved version of Hacker Fab's
stepper. What follows is a proposal: nothing is modelled or built yet. Its sources are in
`prior-art.md`, "Open fabs, and chips made outside one".

**What limits it, by Hacker Fab's own pages:**
- **Vibration.**
  - V2.1 lists a "Vibration susceptibility" of "1.2 µm (when built onto optical table)".
  - The closed-loop nanopositioner it now recommends reached "~1um resolution but highly
    affected by vibration", against a goal of 100 nm.
  - Tennis balls helped "but it was not enough". They have "many theories" but "not enough
    scientifically rigorous testing", and they measure the vibration from camera video, so they
    are "limited by the fps of the recording camera".
- **The projector.**
  - It is a colour projector modified for UV, so it emits "a small but noticeable amount of UV
    even when not actively imaging".
  - Pop-ups from the computer's operating system get imaged onto the chip.
  - Its TI evaluation module, the largest item in the $3,012.13 build, was out of stock everywhere
    in May 2025.
- **A small field.** An exposure is 1.04 × 0.58 mm, and chips up to 1 cm are stitched from many
  exposures. V2's figure for that stitching is 5 µm, and they mark it unverified.
- **Firmware.** An axis sometimes stops responding, and homing sometimes fails.

**Where CortenForge fits:**
1. **Vibration first.** The model is V2.1, built from its open CAD (hacker-fab/stepper_cad, in
   FreeCAD):
   - **The physics:** rigid bodies in sim-core, and the parts that flex as finite elements in
     sim-soft. The model is shaken by the floor and by the motors' steps.
   - **What it gives:** the one figure that blurs a pattern, the objective's motion relative to
     the chip.
   - **The tuning:** sim-opt's optimizers adjust where the posts go, how thick the plates are,
     and the mass and damping. cf-design and the mesh crates then make the parts.

   CortenForge calls its sim crate a "differentiable co-design toolkit". FleXstage, a levitated
   stage built for chip manufacturing, designs structure and control together because bandwidth
   "is limited by the first structural resonance frequency of the stage".

   Its likely gaps come from brownfield-compute's findings: no beam elements, and isotropic
   materials only. No modal solver shows in the crates' descriptions, so resonances would come
   from stepping the model and taking its spectrum. Each gap would go in `docs/CORTENFORGE.md`,
   as brownfield-compute logs its own.
2. **The stage's control loop,** tuned in sim-core before a part is made. The firmware's failures
   are not CortenForge's to fix. They would be a Rust state machine checked with Kani, as the
   bench's limit enforcer is.
3. **Later, a magnetically levitated stage,** which has no backlash and no stick-slip.
   - cortenforge-cfsem's planar conveyor already levitates movers on a coil grid. cfsem computes
     the fields and forces, and CortenForge the dynamics and control.
   - Exposure-tool patents describe wafer stages driven by "a moving-magnet-type planar motor of
     the magnetic levitation method".
   - The hard part is sensing the stage's position to well under a micron.

**Measured, so the model can earn a grade.** The bench's digitizer could read an accelerometer
far faster than a camera takes frames. A real stepper's vibration, measured that way, would check
the model. Until then the model's figures are `unknown`. Measuring needs a stepper, either built
here (about $3,000, Jon's decision) or measured with a Hacker Fab.

**Judged by overlay.** Each change to the stepper is judged by its overlay error between two
layers, measured on a calibration target before and after the change (step 4).

**Where it doesn't fit: the projector.** CortenForge has no optics. The projector's problems, and
the stage's timing, point at the i9 instead.

**Where the i9 comes in.** It would be the stepper's real-time controller, doing three jobs that
a PC, an Arduino and the projector's own board each do today:

1. **The light.**
   - The i9 switches the UV LED on only for an exposure, timed to its clock, so nothing leaks
     between exposures.
   - It can only ask for light. The interlock that cuts the LED's power, when the enclosure opens
     for one, stays in hardware the i9 can't override (rule 2).
2. **The image, with no operating system in its path.**
   - The slicer's frames go from the PC into the i9's memory, and the i9 shows the projector one
     frame for each exposure. No pop-up can reach the chip.
   - Hacker Fab's $250 option for this is a whole Elegoo Mars 4 DLP resin printer, taken apart
     for its DMD projector, which has only a 405 nm LED. The printer's own board takes only
     resin-printer files.
   - OpenMLA calls buying the printer "The easiest and cheapest way to source the projector".
     The projector alone can come from its maker, Anhua, at "a premium".
   - OpenMLA's controller, a hat for a Raspberry Pi Zero 2, sends images over SPI to the TI
     controller on the projector's own board, the DLPC1438.
   - So the micromirror chip needn't be driven without TI's controller: the projector has one, and
     it can be fed directly. The i9 would do the hat's job.
   - This would also replace the TI evaluation module that is out of stock.
3. **Motion, and exposures timed to it.**
   - The i9 runs the stage's loop and sequences each exposure: step, settle, expose.
   - With an accelerometer on the digitizer's ADC, it waits to expose until the measured
     vibration has died down. That is a cheap fix for the stepper's largest problem, and it needs
     no mechanical redesign first. OpenMLA's board breaks out pins for exposing "based on some
     sensor readout".
   - The same measurement could shape each move so the frame rings less. Klipper does this for
     3D printers, calling it input shaping, from an accelerometer's measurement of the printer's
     resonances (Klipper3d/klipper, `docs/Resonance_Compensation.md`). Klipper is GPL-3.0 and
     this repo is MIT or Apache-2.0, so the method is taken from its docs, not its code. Whether
     it shortens the stepper's settling is `unknown`.
   - It would replace GRBL on the Arduino, whose settings Hacker Fab ties to its backlash. The
     gateware would be checked as the bench's cores are, with SymbiYosys and cocotb.

**What stays on the PC:** the camera and its fiducial detection (RF-DETR), which does the
alignment, and the slicer.

It is the same board as the digitizer's and LittleBit's, with the same tools. Every bitstream
load still needs Jon's OK (CLAUDE.md). Nothing here is built or tested on an i9.

### Value engineering

The aim is to spend less where it doesn't buy quality, and more where it does. The figures are
Hacker Fab's own: the total its V2.1 page states, and the prices on its page of other projectors.
Shops' per-item prices are left out (CLAUDE.md). Nothing has been priced or bought here.

**Where to save:**

| item | what it does | instead | saves | what it does to quality |
|---|---|---|---|---|
| the TI evaluation module, the largest item | the image | a whole resin printer, taken apart for its projector, fed by the i9 | about $1,050: Hacker Fab prices the TI alternative at $1,300 and the resin printer at $250 | **better:** no stray UV, no pop-ups, in stock. **Worse:** its 0.3" chip, against 0.47", gives each exposure about 40 % of the area, if the optics are the same (the ratio squared). So there would be 2.4 times as many stitched exposures. Its resolution through V2.1's optics is `unknown` |
| the camera | alignment and inspection | a cheaper camera with a smaller sensor, behind a reduction lens of about $50, which Hacker Fab says was tested at UC Irvine | depends on the camera | **a risk:** alignment rests on it. Decided after measuring what the alignment needs |
| the Arduino and its CNC shield | the stage, through GRBL | the i9, already here | a little | **better:** no GRBL settings to tune. The stepper-motor drivers stay |
| the Thorlabs tubes and adapters | the optical path | generic tubes and adapters with the same SM1 thread | `unknown`: Hacker Fab's Thorlabs list gives part numbers, not prices | **neutral**, if the beamsplitter, the cube and the post stay Thorlabs. They set alignment and stiffness |
| the 10× objective | sets the resolution | keep it, or upgrade it (below) | — | — |

**Where to spend, in order:**
1. **An enclosure with a UV interlock.** Safety comes first (rule 2), so it isn't optional. It
   also keeps out room light, dust and air currents, and steadies the temperature.
2. **A calibration target,** chrome on glass, with resolution and grid patterns. Without one,
   resolution, distortion and stitching can't be measured. Hacker Fab marks its stitching and
   overlay figures, 5 µm, unverified. The smaller projector makes stitching matter 2.4 times as
   much.
3. **Measuring vibration:** an accelerometer read by the bench's digitizer. It is what exposures
   would be timed to.
4. **Vibration fixes,** in this order:
   - isolation feet, or a used optical breadboard;
   - a direct mount for the projector, as Hacker Fab suggests;
   - a stiffer frame, from the CortenForge model.
5. **The closed-loop nanopositioner** Hacker Fab now recommends.
   - Its magnetic encoders (MT6835) are read over SPI, which Hacker Fab says needs wires under 25
     cm.
   - The i9 could read them from close by.
6. **A better objective,** with a higher numerical aperture: sharper, and less distorted at the
   edge of each exposure, so it stitches better. Its price is `unknown`.
7. **An inspection light that doesn't expose the resist,** red or amber. The resin-printer
   projector has only its 405 nm LED.

**What to do, in order (the recommendation):**
1. **Now, with nothing bought:**
   - Ask Hacker Fab whether anyone has measured resolution through the resin-printer projector.
     That is the swap's largest unknown.
   - The vibration model, from the open CAD.
   - The i9's SPI link to the projector's TI controller (DLPC1438), in simulation. It would be
     checked with cocotb against what TI's documentation and OpenMLA's MIT-licensed code say the
     controller expects.
2. **The first purchase, Jon's decision: a whole Elegoo Mars 4 DLP resin printer, about $250,
   taken apart for its projector.**
   - The $250 buys the whole printer, not a projector alone. Hacker Fab's option and OpenMLA's
     controller both take the projector out of the printer, and OpenMLA gives the steps.
   - Its safety plan comes first, since 405 nm light is an eye hazard (rule 1).
     - OpenMLA's board takes the printer's own 12 V supply at a barrel jack, so the work on the
       projector is at 12 V. The plan checks that the supply is a separate adapter, so that taking
       the printer apart opens nothing on mains (rule 3).
     - No resin is used, which keeps the resin's chemical hazard out.
   - It comes with an inspection light.
   - The i9 drives the projector on the bench. The checks: the image arrives whole, and the LED
     lights only during an exposure.
3. **Then the stepper,** with the savings spent on the list above, the enclosure and the
   calibration target first.

This order puts the largest saving, and the largest unknown, in the cheapest test. Nothing more is
bought until the free work says it is worth it.

## Hacker Fab's other tools

Jon, 2026-11-25: "even though these are already at hackerfab, its still a good idea to apply our
engineering to them if we see something that can be both better and more economical ... theres a
large gap between what they have and what the state of the art is."

So a tool Hacker Fab already has is a candidate, not a reason to leave it out. Each one below:
- is a proposal, `unknown` until it earns a grade;
- is compared with Hacker Fab's own version, on a measure named before the work starts;
- is judged by Hacker Fab's own goals too, among them "Low cost" and "Highly sourcable
  materials", so that any Hacker Fab can rebuild it.

The safety rules still decide which tools can have a version of ours. RF and sputter supplies,
furnaces, hotplates and pumps stay bought (rule 3). A tool on vacuum, gas, RF or high voltage
runs only under its safety plan (rule 1).

The sources are in `prior-art.md`, "Open fabs, and chips made outside one" (searched
2026-11-25).

**The gap, row by row,** is `fab/stack.toml`, and tentzhen.com's fab page shows it (`site/fab/`).
Each row sets Hacker Fab's figure beside the industry's, quoting each source, with the gap and
Tentzhen's route across it. Where both sides have a number, the warehouse works the gap out
(`gold.page_fab`).

**1. The sputter tool's control station, in Rust.**
- **Hacker Fab's:**
  - Pressure control, a state machine and a PI loop, is moving from an Arduino Uno to a Pico 2
    (RP2350). Its board is ordered, not yet tested.
  - "safety interlocks" are on its list for later versions.
  - Its software is to follow SputterOS (Apache-2.0), a "Deterministic C++17 microkernel
    framework for safety-critical nanofab system control" with "multi-tier safety interlocks".
- **Ours:** Jon: "if we wanted to build a tockOS/embassy version of the sputterOS pico2w impl of
  theirs, and see which was better, that's fine with me." Hacker Fab's page names the Pico 2; the
  Pico 2 W has the same RP2350. Since the chip is the same, the comparison is not about price.
- **Judged by,** with both versions driving the same simulated chamber before either touches a
  real one:
  - whether each holds the same pressure set points;
  - what each does when a device stops answering, as their pressure gauge's RS-485 link did;
  - what is checked: Kani on our interlock logic, as on the bench's enforcer, against SputterOS's
    own tests;
  - whether a fault in the process logic can reach the interlocks.
- **Tock, then Embassy** (Jon, 2026-11-26: "Tock then Embassy if Tock doesnt fit the application
  well"). The station is written on Tock first. If Tock doesn't fit it, the reason is written
  here and the station moves to Embassy (CLAUDE.md, "Language policy"). UMSH's own firmware,
  which the mesh uses until a Tock port runs, is built on Embassy (`mesh.md`).

**2. One interlock design, for every tool.**
- **Hacker Fab's:** its tools run on different controllers:
  - an Arduino Uno, then a Pico 2, for the sputter tool's pressure;
  - a Teensy 4.1 for the impedance matcher, and another for the quartz-crystal thickness monitor;
  - an Arduino Nano on V3's interferometer;
  - GRBL on an Arduino for the stepper's stage;
  - a Raspberry Pi hat for the resin printer's projector;
  - an STM32 and a Pi for the Raman spectrometer.
- **Ours:** one design across the tools, in the bench's pattern: limits held below the agent, and
  the last word in hardware (rule 2). The sputter station above would be its first tool.
- **Judged by:** whether a Hacker Fab can rebuild it from common parts.

**3. Reading the stage's interferometer.**
- **Hacker Fab's:** "Stepper V3 - WIP", last edited 2024-03-15, measures the stage with a 593 nm
  Michelson interferometer. Its photodiode goes through a transimpedance amplifier to "an
  Arduino Nano".
- **Ours:** the digitizer's ADC reads the photodiode, on the same i9 that runs the stage's loop
  ("Where the i9 comes in", above).
- **Judged by:** the smallest stage motion each resolves, on the same stage.

**4. The impedance matcher.**
- **Hacker Fab's:**
  - It is a "Working prototype". A Teensy 4.1 reads forward and reflected power from an
    off-the-shelf VSWR meter and averages 300 samples.
  - It finds the match by coordinate descent: it steps a capacitor, measures the change and
    steps again.
  - Rated for 100 W of forward power, it holds about 1.2 VSWR once stable and matches in about
    5 seconds from startup. It costs $636.65.
- **Ours:**
  - The digitizer samples a directional coupler's two outputs before they are rectified. That
    gives the reflection's phase as well as its size.
  - Phase and size together give the impedance the matcher sees, so the capacitors could be set
    from it instead of found by stepping.
  - The sputter tool's radio supplies 14 MHz, by its page; the matcher's page says 13.56 MHz.
    Whether instrument #1 samples either well is `unknown` until its bandwidth is measured
    (`scope.md`).
- **Judged by:** time to match, and the VSWR held, on the same chamber, under the sputter tool's
  safety plan. Hacker Fab's page puts the network's maximum at 200 V peak to peak.

**5. Film thickness, from a fitted model.**
- **Hacker Fab's:**
  - Spin-on glass is measured by matching a spectrometer's curve to calibration graphs:
    "Determine which shape your graph most closely resembles".
  - The furnace's oxide is read from a colour chart.
- **Ours:** fit the same spectrum to the physics of thin-film interference, written as a golden
  model in Rust with its tests. It needs no new hardware and carries no hazard.
- **Judged by:** the fit's thickness against another method's on the same film. Hacker Fab has a
  profilometer procedure.

**6. The source-meter and the probe station.**
- **Hacker Fab's:**
  - Three Analog Discovery 3s serve as source-meters.
  - The probe station is built on bought XYZ stages. It senses the probe's touch with a piezo
    sensor, still in progress.
- **Ours:** instrument #1 where an Analog Discovery 3 is used, and its ADC on the touch sensor.
  This is step 3's test gear ("The steps", above).
- **Judged by:** the same transistor's curves, taken on both. Instrument #1's price is `unknown`
  until it is built.

**Not read yet,** beyond their repositories' descriptions:
- UW's board for mass-flow controllers;
- the low-cost atomic force microscope (hacker-fab/afm);
- the tube furnace's controller code;
- SputterOS's own code.

## Running the fab: lots, recipes and tools

Read 2026-11-26. Jon's Claude session of 2026-11-26 listed the records an industrial fab keeps
beside its tools, to be checked against what Hacker Fab already has (`makerspace.md`, "Where this
came from"). Its route: read Hacker Fab's GitBook, join its Discord, map what exists against the
list, and lead with one small working tool rather than a critique. Hacker Fab's README says "We
communicate over Discord"; joining is a step on Jon's own account.

**Hacker Fab's process database comes first.** Its GitBook (`fab-toolkit/database-build/README.md`
at bd1eef57) describes db.hackerfab.org, a Django site hosted on AWS. Its code is
hacker-fab/data-management (MPL-2.0, last commit 2025-05-04, ae5436a3).
- **Built, as the page tells it:** "chip pages" that "show every process done to a chip", and
  input forms with "prefilled default values for standardized parameters in our primary NMOS
  process".
- **Planned, in the page's future tense:** a "chip profile" that the chip creation page "will
  also have". A job queue on AWS, from which a Raspberry Pi beside each tool, the spin coater
  first, "will pull jobs" (`machine-integration.md`).
- **In its `models.py`:** a chip is a number, with an owner, a university, a creation time, a
  starting material, notes and its I-V curves. Each process is a table of its parameters and the
  step's time. No field is named for a lot, a batch, a recipe, a version, a yield or a wafer (a
  case-insensitive search of the file).

| practice | Hacker Fab, as read | Tentzhen's records today |
|---|---|---|
| a run card: every step a chip went through, with its parameters | its chip pages, and a table per process | a measurement record names the hardware revision, the gateware and firmware hashes and the toolchain versions (`lab/records/`) |
| lots: chips that move through the steps together | no lot field | none |
| recipes under version control | NMOS defaults in its input forms | the repo is in git; no fab step has a recipe file yet |
| statistical process control | not read | none |
| test structures and yield | 94 % yield in its March 2026 update ("The steps", above); how yield is tracked, not read | none |
| the same process at every site | ten Hacker Fabs share one GitBook; whether their processes are matched, not read | none |
| tool qualification and maintenance logs | a tube-furnace malfunction log, dated by hand (`fab-toolkit/tube-furnace/malfunction-log.md`) | the append-only lab log of hardware actions (`lab/log/`) |
| root-cause analysis | not read | none |
| chemicals and consumables by lot | a page per material (`fab-toolkit/chemicals-materials/`); whether lots are kept, not read | none |

"Not read" leaves Hacker Fab's side `unknown`, not absent. The small working tool is the
makerspace's stores (`makerspace.md`): they keep parts by bin first, then chemicals by lot, then
wafers by lot. Each is a proposal, `unknown` until it earns a grade. Wafer lots would be built
where Hacker Fab's database can take them, written to it or beside it, once its code has been
read.

## Security: checking the fab's computers are as they were left

Jon, 2026-11-27: "a persistent/trusted/tamperproof environment would be extemely important for a
fab like this. you dont want anyone messing with important work, it could cost money, be
dangerous". And: "we dont need to immediately buy anything, we are just speccing this out. for
the fab. we can iteratively improve this over time until its sound enough to buy and test irl."

So nothing here is bought, and every claim about the design is `unknown` until it earns a grade.
It starts from a plan Jon drafted with Claude the same day ("Where this came from", below). What
already exists is in `prior-art.md`, "Attesting a fab's computers".

**What it protects, and what it doesn't.** It protects the work: designs, recipes, records, and
the time and money in them. It doesn't make the fab safe. That is the interlocks' job (rule 2): a
tampered computer must be unable to make a tool dangerous, whatever this check says.

### The goal

Leave the fab, come back, and show in about a minute, for each of its computers, that:
- it didn't restart while no one was there;
- its firmware, bootloader, kernel and initramfs are the ones our build made;
- nothing ran on it that the image and its containers don't hold;
- no case, cabinet or door was opened (seals and sensors, "The room", below).

### Trust only the device in your pocket

- **Each computer's TPM** signs a report, a quote, of what it booted and how often it has
  restarted, with a key that never leaves the TPM.
- **A verifier you carry,** a Raspberry Pi Pico 2 on Tock, sends each computer a fresh random
  challenge and holds each one's public key. A hub in the fab only carries the messages. It can't
  forge a quote or pass off an old one; it can only withhold one, which shows red.
- **The verifier holds no secrets,** only public keys, reference values and the last snapshot.
  What matters is that no one can change them.

| threat | what shows it |
|---|---|
| someone changes the firmware, bootloader or disk, then puts things back | a restart, or a boot measurement that isn't ours |
| a look-alike computer replaces a real one | it can't sign with the enrolled key |
| a compromised system reports "all clear" | the TPM signs the quote, not the system |
| an old clean answer, sent again | every challenge is new |
| the hub is compromised | the verifier makes the challenges and judges the answers |

Out of scope: attacks on the chips themselves, such as fault injection or decapping, given long
unattended access. The seals and cameras are there to make those hard to do unseen. Not seen by
any check here: an attack on a running computer that neither restarts it nor runs a file the
image and its containers don't hold, such as one that uses only programs already there. What IMA
measures is set by its policy, which is still to write.

### Enrolling a computer

Once, when the computer is known to be clean:
1. The computer makes an attestation key in its TPM.
2. The verifier proves that key is in that TPM, by credential activation against the TPM's
   endorsement key. It makes the challenge itself (MakeCredential). Checked against the TPM
   maker's certificate, the endorsement key puts the verifier's trust in the TPM, not in the
   system enrolling it. An endorsement key only recorded at enrollment trusts that system, at
   that one clean moment. Whether the verifier can check the maker's certificate itself is
   `unknown`.
3. The computer shows the key's fingerprint on its own screen and the verifier on its own, and a
   person compares at least 80 bits of it before pressing accept.

### Each check, leaving and coming back

The verifier sends a challenge to every computer it enrolled, from its own list, not the hub's, so a
computer the hub leaves out shows red. Each challenge is 32 bytes from the RP2350's random-number
generator, hashed with SHA-256, new for each computer and each check. The computer answers with a
quote of PCRs 0 to 10, its boot event log, and the IMA entries added since the last check. Each
message is a CBOR object in a frame with its length and a CRC32.

The verifier checks that:
1. the quote is signed by the enrolled key;
2. the TPM made it, as a quote (`TPM_GENERATED_VALUE`, `TPM_ST_ATTEST_QUOTE`), which means
   something because enrollment showed the key is the TPM's own;
3. it carries the challenge just sent;
4. the PCR values the hub passed on hash to the digest inside the signed quote;
5. the event log replays to PCRs 0 to 9, and its entries match the reference values;
6. the new IMA entries replay from the last check's PCR 10 to this one, and each is a file the
   image or its containers hold.

It reads the reset and restart counts, the clock and the TPM's firmware version from the signed
quote.

| finding | result |
|---|---|
| the reset or restart count changed | red: restarted |
| a PCR from 0 to 9, or an event log entry, isn't the reference | red: boot chain isn't ours |
| a new IMA entry isn't a file in the image | red: something new ran |
| the TPM's firmware version changed | red: TPM firmware changed |
| no valid answer | red: missing |
| the clock went backwards | amber: look into it |
| everything matches | green |

**Why PCRs 0 to 10.** GRUB puts the kernel command line in PCR 8, and every file it reads, the
kernel and initramfs included, in PCR 9 (GRUB 2.16's manual, "Measured Boot", read 2026-11-27).
IMA measures into PCR 10 by default (`IMA_MEASURE_PCR_IDX`, "default 10", in Linux's
`security/integrity/ima/Kconfig` at f4585067). Whether Fedora's GRUB loads the module that
measures is `unknown`.

**Reference values, not only the last snapshot.** Comparing with the last check alone would pass a
computer that was already wrong when it was left. So:
- **Firmware** is ours: built from Dasharo's public source, or a release whose sha256 Dasharo
  signs. Its reference values are what a board reports on its first boot after that firmware is
  flashed. Dasharo's tests on the ODROID-H4 include "Identical configuration results in identical
  measurements" (MBO006.001, passed on v0.9.1), on one board. Whether two identical boards report
  identical PCRs 0 to 7 is `unknown` until two are measured; if they do, each computer is also
  checked against its twins, the other boards like it.
- **Boot software,** the shim, GRUB, the kernel and initramfs: the image's build computes the
  digests the event log will hold. The kernel command line, in PCR 8, is written on each computer
  when an update is deployed; whether the build can compute it is `unknown`.
- **What runs:** the build publishes one Merkle root over the hashes of every file in the image
  and in the container images it pins.
  The hub sends a proof with each new IMA entry, and the verifier, holding only the root, checks
  it. How many entries a quiet computer adds in a day is `unknown` until measured.

Keylime, on the hub, keeps attesting every computer while no one is there, and raises an alarm on
a failure. The hub is not trusted, so that alarm is a help, not the judgement. The judgement is
the verifier's, at the next check.

### The verifier

- **Hardware:** a Pico 2 (not the W, which has a radio), a small SPI display, two buttons
  (accept, cancel), and a USB-to-UART adapter on the hub. Two verifiers, enrolled together and
  kept apart. GrapheneOS's Auditor is not a backup: its verifier is "another Android device
  running the app in the Auditor mode", or its attestation.app service (attestation.app/about,
  read 2026-11-27).
- **UART, not USB:** over UART the hub can only send bytes to a small parser. Over USB it would
  drive a USB device stack.
- **Tock's drivers:** upstream's `chips/rp2350` (tock/tock at b89939e0, 2026-11-23) has UART,
  SPI, GPIO and timers, and Jon's `via-balaena/tock` adds `trng.rs` on its `main`. Drivers for the
  SHA-256 accelerator and the OTP are still to write; SHA-256 in software is the fallback.
- **Its randomness** is the RP2350's own TRNG, an Arm block on its own ring oscillator, which
  "Produces approximately 7.5 kb/s of entropy when the core runs at 150 MHz" (RP2350 datasheet,
  build-date 2026-11-19, section 12.12). The bootrom "streams raw TRNG ROSC samples (the TRNG
  random source) directly into the hardware SHA-256 accelerator", and so "bypasses all internal
  checking and conditioning in the TRNG". The verifier would do the same, after running SP
  800-90B's health tests on the raw samples: the tests of project #6's step 4 (`pbits.md`), from
  the Rust model its gateware is checked against. A failed test stops the check. Jon's `trng.rs`
  (at d343c1e5) reads the conditioned `EHR_DATA` registers, so reading the raw samples is new
  driver work.
- **Processes:** the parser, the judge and the display run as separate Tock processes, so a bug
  in the part the hub talks to can't reach the keys or the snapshots.
- **Signing its firmware:** secure boot with our key's hash in OTP, and the debug port locked.
  Both are irreversible, so each needs a person's approval in the session, on a board approved
  for flashing (CLAUDE.md). Practise on a Pico 2 W first: it has the same RP2350.
- **Its stored state** lives in the Pico 2's external flash. Secure boot doesn't cover it, so anyone
  holding the board can rewrite it. Keeping the verifier on you covers that. A MAC over the state,
  with a key in OTP, would go further, and would make that key the verifier's one secret.
- How long the RP2350 takes to check a P-256 signature is `unknown` until measured.

### The fab's computers

- **One board for every computer the verifier checks:** the hub, the stores' server, each tool's
  station, and a spare. One board means one firmware build, one set of reference values, twins to
  compare, and a spare that replaces any of them.
- **The candidate is the ODROID-H5** (Hardkernel, May 2026): an i3-N300, "fTPM enabled" and
  10 GbE, $230 for the board alone (hardkernel.com, read 2026-11-27). The H4, H4+ and H4 Ultra
  are marked "(DISCONTINUED)" there. A used H4+ is the surplus route: our own build replaces what
  its BIOS flash holds, and the board goes through incoming QA like any part (`sourcing.md`).
- **Its firmware is Dasharo:**
  - For the H5, only a release candidate (`hardkernel_odroid_h5_v0.9.0-rc2`, Dasharo/coreboot,
    2026-10-15).
  - For the H4, v0.9.1 (2025-09-03). It passed all 35 of the TPM and measured-boot tests Dasharo
    ran on it, among them "Check if event log PCRs match actual values", and two more were not
    run (Dasharo/osfv-results, `v0.9.1_results.csv`).
  - The H4's built binaries come with Dasharo's paid Pro Package. Its source is tagged in public
    and its docs give a building manual, so we build it.
  - **Not TrustRoot.** It checks firmware "against a specific private key managed by the device
    vendor", so "neither adversaries nor the device owners themselves can change the firmware" to
    one that key didn't sign (Dasharo's glossary). That rules out our own builds, and measured
    boot already turns a firmware change red.
  - Dasharo lists a known issue on the H4: "TPM is cleared after HAP disabling and then
    reenabling ME". Whether the fTPM works with Intel's ME disabled is `unknown`.
- **Not fully open:** Dasharo carries Intel's binary firmware, and its openness charts measure how
  much. Dasharo also supports a POWER9 board, Raptor's Talos II; how open its firmware is, and
  what it costs, weren't read here.
- **Software:** one bootc image, built from Fedora, on every computer the verifier checks, so one
  Keylime runtime policy covers them all; Keylime (7.14.3, 2026-07-06); Ansible for every
  configuration and update; and Podman for the services.
- **Disks** are unlocked by the TPM and a PIN (`systemd-cryptenroll`). PCR 7 is the "Secure Boot
  state", which "changes when UEFI SecureBoot mode is enabled/disabled, or firmware certificates
  (PK, KEK, db, dbx, …) changes" (`man/systemd-cryptenroll.xml`, systemd at 39c2bf5f). Bound to PCR
  7 alone, the unlock follows the Secure Boot keys, not what booted, so a swapped initramfs still
  unseals the disk and catches the PIN. A signed policy on PCR 11 follows the kernel and initramfs,
  once the image boots unified kernel images (systemd's `docs/TPM2_PCR_MEASUREMENTS.md`: "on systems
  that are not booted via a UKI, PCR 11 does not cover the kernel and initrd").

### The network

| zone | holds | reaches |
|---|---|---|
| management | the hub | each computer's Keylime agent, until the agents push (below) |
| fab computers | the tools' stations, the stores' server | each other and the hub |
| tool controllers | the controllers we bought | their own station only; no internet |
| cameras | local cameras | their recorder only; no internet |

- **Keylime's push model:** keylime/rust-keylime has a push-model agent
  (`keylime-push-model-agent`), with which computers connect to the hub instead of the hub to
  them. Then the hub needs no way into the other zones. How ready it is is `unknown`.
- **The router** is the OpenWrt project's own board, the OpenWrt One: an MT7981, 1 GB of RAM, NAND
  with NOR for recovery, and 2.5 GbE and 1 GbE ports (openwrt.org's table of hardware, read
  2026-11-27). The verifier can't check it, so it goes in the locked cabinet.
- **Alerts while no one is there** need a way out of the lab network, which CLAUDE.md keeps
  closed without approval ("Open decisions", below).

### The room

- A locked cabinet for the router, the hub, the servers and the UPS.
- A UPS that is bought and listed, used as sold (rule 3), and read over USB by NUT. Without it, a
  power cut would restart the computers and show red; its log says whether the power went.
- Chassis intrusion switches, where a board has the header. Whether the H5 has one is `unknown`.
- Tamper seals on cases, the cabinet and unused ports, photographed when leaving and compared on
  coming back.
- Local cameras recording to Frigate, and door, smoke, water and temperature sensors: fire and
  water matter here as much as intrusion. Their alerts come through the hub, so, like Keylime's,
  they are a help, not the judgement.

### The tools' controllers

- **Bought controllers** (GRBL, Teensys and Arduinos, as Hacker Fab's tools use: "Hacker Fab's
  other tools", above) can't be attested. They get their own zone with no internet. Their
  firmware versions are recorded and their settings kept in the repo.
- **Our own,** the sputter station first, could carry an identity measured from their firmware:
  a key derived inside the chip from a secret and the firmware's hash, as DICE does, for the same
  verifier to check. `unknown`.
- **Designs and recipes** are kept in git with signed commits, as `main` already requires, so a
  changed file shows up as a difference from a signed commit.

### The routine

- **Before leaving, if updates are due:** run the update playbook, restart, and take a check.
  Nothing is updated while no one is there.
- **Leaving:** plug the verifier into the hub, wait for "snapshot saved" for every computer, and
  photograph the seals.
- **Coming back:** plug it in again and read each computer's result on the verifier's own screen,
  never the hub's; then the sensors' events. On any red, type no password on that computer until
  it has been looked at.

### Where this came from

Jon's plan, drafted with Claude on 2026-11-27. Changed here:
1. The hub and server board: the H4 family is discontinued, so the H5 is the candidate, and
   Dasharo is still a release candidate for it.
2. What ran is judged by the verifier from PCR 10, not by Keylime on a hub the plan itself calls
   untrusted.
3. PCRs 0 to 7 became 0 to 10, to cover the kernel command line, the initramfs and what runs.
4. Reference values and twins, beside the last snapshot.
5. Enrollment proves the key is the TPM's own, and compares 80 bits, not a short fingerprint.
6. TrustRoot is dropped.
7. UART is chosen to keep the hub away from a USB stack, not as a fallback.
8. The verifier's flash: what needs protecting is that no one changes it, not that no one reads
   it.
9. The backup verifier is a second Pico, not a phone app that verifies phones.
10. The hub's reach into every zone ends once the computers push to it, if Keylime's push model
    is ready.
11. The verifier challenges the computers it enrolled, not the list the hub sends.

Left out: the plan's prices, which were estimates, and its computers that aren't the fab's.

### Before anything is bought

- Dasharo for the H5 is released, with its TPM and measured-boot tests passed.
- Fedora's GRUB measures into PCRs 8 and 9.
- How many IMA entries a quiet computer adds in a day.
- Whether two identical boards report identical PCRs 0 to 7.
- How ready Keylime's push model is.
- How long the RP2350 takes to check a P-256 signature.
- Whether the H5 has a chassis intrusion header.
- Prices for everything but the board.

### Getting to v1.0

Jon, 2026-11-27: "great idea", to auditing the design by making it run before anything is
bought. The review that found the hub could leave a computer out did it by reading; a test of
that row would have failed.

**v1.0 is reached when:**
1. every threat in "Trust only the device in your pocket" and every row of the findings table has
   a named test, and taking out the check it covers makes that test fail (`tested`);
2. the parser that the hub's messages reach is checked with Kani, up to a stated bound
   (`checked`);
3. one whole leave-and-return runs on virtual machines, with each attack staged;
4. each item of "Before anything is bought" is answered, or marked as needing the board.

Then one H5 is worth buying, Jon's decision, and a second answers whether twins report identical
PCRs.

| step | what | needs |
|---|---|---|
| 1a | **The judge,** in Rust, built for the host and for the RP2350 alike (`no_std`): the quote's signature, challenge, PCR digest and counts; the findings table; and the verifier's own list of computers. Its test inputs are real quotes from a software TPM, committed with the script that makes them, so the Quality Gate needs no TPM. | nothing bought |
| 1b | **The boot event log,** replayed to PCRs 0 to 9 and checked against reference values | nothing bought |
| 1c | **The IMA entries** since the last check, replayed to PCR 10, each with a proof against the image's Merkle root | nothing bought |
| 2 | **Kani** on the parser and the findings table | nothing bought |
| 3 | **Virtual machines:** Fedora's bootc image under QEMU, with a software TPM and UEFI measured boot, and Keylime's push model between them | nothing bought; whether this Mac runs it, or the x86 desktop is needed, is `unknown` |
| 4 | **A Pico 2 W already on hand:** how long a P-256 check takes; the raw ring oscillator with the health tests | Jon's approval to flash it, since it isn't a lab target; no OTP step; waits until Jon can reach the board (2026-11-27) |
| 5 | **An attack read by someone else:** a reviewer given only this section and Sperrle's security analysis, not the reviews before | nothing |

- **Step 3 answers,** in a virtual machine, whether Fedora's GRUB measures into PCRs 8 and 9, how
  many IMA entries an idle computer adds in a day, and how ready Keylime's push model is. Step 4
  answers how long the RP2350 takes to check a P-256 signature. The H5's Dasharo release, the
  twins, its chassis header and the prices need the board or a later read.
- **Before step 1a's parser is written,** an existing one is looked for (CLAUDE.md,
  "Conventions"). tpm2-tools' `tpm2_checkquote` already makes checks 1 to 4 on a host: its man
  page names the signature, the qualifying data and the PCR values, and its source checks the
  magic and the type (`tools/misc/tpm2_checkquote.c`, tpm2-software/tpm2-tools at 915443b3).
  A Rust one exists that runs on a microcontroller: tpm2-protocol 1.2.4 (MIT OR Apache-2.0,
  jarkkojs/tpm2-library at c77e90fa), "A unipolar `no_std` TPM 2.0 implementation that does not
  require heap allocator and has zero dependencies", with `TpmsAttest`, `TpmsQuoteInfo` and
  `TpmsClockInfo` among its types. Step 1a builds on it. Searched on crates.io, 2026-11-27, for
  "tpm2", "tpm quote", "tpms_attest", "tpm attestation" and "tpm2-protocol".
- **The software TPM:** swtpm 0.10.2 is in Homebrew; tpm2-tools is not (searched 2026-11-27). So
  step 1a's test inputs come from swtpm, driven by a small Rust program that speaks the TPM's
  commands through tpm2-protocol.

## Later: a gradiometer, engineered with CortenForge

Jon, 2026-11-25: "the gradiometer is from cortenforge. its a goal of mine to use cortenforge to
engineer one in the future."

CortenForge's study (via-balaena/CortenForge, `docs/studies/gradiometer`) is of a
"room-temperature MEMS gravity gradiometer".
- **Status:** "Conditional GO" for "the large-anomaly / void niche", with "No code written yet".
- **Scope:** it leaves "etch-process physics, circuit synthesis, DAQ firmware" out of CortenForge.
  Etching is the fab's kind of work, and reading out an instrument is the bench's.

Whether our process could make its parts is `unknown`.

## The town

- **The centre is the fab, together with what made Shenzhen:** a parts market, PCB fabrication
  and assembly, a machine shop, and the bench's test gear. The fab alone is a lab of a handful of
  people, not a town's anchor; that is a judgment, `unknown`.
- **Its buildings use brownfield-compute's building tools, shared rather than copied.** The Homer
  City analysis stays in brownfield-compute, whose reports serve its townships.
  - Where a tool belongs in CortenForge, it goes there as a suggestion. Brownfield-compute's own
    `docs/CORTENFORGE.md` already lists several, from routing to IFC export.
  - Here CortenForge is used through its published crates, as an end user, with what's missing
    logged in a `docs/CORTENFORGE.md` of this repo's own, not written yet, once code meets it.
- **Sites with a big grid connection suit the heaviest steps.** Brownfield-compute's retired
  coal plants are an example, and smelting is the step that would need one.

## Open decisions, Jon's

- Brownfield-compute's building work: shared or moved, and in which language here, since
  CLAUDE.md keeps long-lived code in Rust.
- Upstream: whether via-balaena/tentzhen takes the wider scope.
- The training's compute (`littlebit.md`, "The training needs more compute").
- Whether the hub may send alerts out of the lab network while no one is there ("Security",
  above). CLAUDE.md keeps the lab network closed without approval.
