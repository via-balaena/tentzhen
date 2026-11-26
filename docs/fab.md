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
