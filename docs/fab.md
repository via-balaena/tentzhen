# The open fab, and the town around it

Project #4 (Jon, 2026-11-24) is "a fully open source ASML/TSMC engineering project". It is built
up to scrappily and in public, to show what is possible.

- **What the fab is for:** it builds computers that resolve conflict before it happens. They are
  the chips of project #3, which run simulations of trained agents more cheaply per game step
  than a GPU (`littlebit.md`, "What it is for" and "The goal").
- **The town:** a town grows around the fab, the way brownfield-compute's town grows around its
  data center, and a city like Shenzhen grows from there.
- **Safety comes first:** "the towns safety is 100% the most important thing" (Jon, 2026-11-24).

Nothing here is built. Every step below is a proposal, and its claims are `unknown` until they
earn a grade. What already exists is in `prior-art.md`, "Open fabs, and chips made outside one".

## Safety first

These rules are proposed, and they take effect when Jon approves them. Until then no step that
handles chemicals, heat over a hotplate's, high voltage, vacuum or pressure starts.

1. **No step starts without its safety plan.** The plan is written before anything is bought. It
   lists:
   - each hazard: chemical, heat, electrical, pressure, vacuum, light and waste;
   - the controls and the protective equipment for each;
   - what to do in a spill, a fire or an exposure;
   - who may do the step.

   Jon approves each plan in the session it is used (CLAUDE.md).
2. **Limits below the agent, as on the bench.** An AI may plan a run and log it. Heaters, pumps,
   high voltage and chemical dispensing sit behind interlocks it can't reconfigure: an
   over-temperature cutoff in hardware, an emergency stop, and fume detection that cuts power
   (`vision.md`, principle 5).
3. **Anything on mains is bought and certified.** That covers hotplates, furnaces, pumps, and RF and
   sputter supplies. Each is plugged in as sold, or installed by an electrician. The project
   designs, builds and wires no mains equipment. Hacker Fab's first goal is to "Make a DIY version
   of every nanofabrication tool". Here the tools on mains would be bought instead.

   Whether this fits CLAUDE.md's "mains voltage out of scope entirely" is Jon's to say (below).
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
   universities, and CMU's waste procedure names a wet chemistry lab, ESC 232. A fab in a town has no university's safety
   office behind it, so it needs its own. How the Hacker Fabs share that work with their
   universities is `unknown` here.

## What it builds, and why fidelity matters

A simulation predicts a conflict only if its agents act as the trained ones do. That is why
project #3 judges a compressed policy by the behaviour bar, not by score (`littlebit.md`,
"Whether a student plays as the teacher does").

In Clean Up, zapping is the conflict, and the compressed students already get it wrong both
ways. These are `behaviour.py`'s figures, one training run each:

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
| 4 | lithography: a maskless stepper on Hacker Fab's open design, patterning photoresist on bought wafers | resist and a hydroxide developer, under its safety plan; no acids or furnace | the smallest line it prints, measured |
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
  - Its TI evaluation module, $1,328.63 of the $3,012.13 build, was out of stock everywhere in
    May 2025.
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
   - Hacker Fab lists a $250 projector for this: the DMD projector of an Elegoo Mars 4 resin
     printer, with only a 405 nm LED. Its own board takes only resin-printer files.
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
   - It would replace GRBL on the Arduino, whose settings Hacker Fab ties to its backlash. The
     gateware would be checked as the bench's cores are, with SymbiYosys and cocotb.

**What stays on the PC:** the camera and its fiducial detection (RF-DETR), which does the
alignment, and the slicer.

It is the same board as the digitizer's and LittleBit's, with the same tools. Every bitstream
load still needs Jon's OK (CLAUDE.md). Nothing here is built or tested on an i9.

**The order proposed:**
1. The vibration model, from the open CAD, since it needs nothing bought.
2. A measurement of a real stepper.
3. The i9 as the stepper's controller: the light, then the resin-printer projector fed from the
   i9, then exposures timed to the measured vibration.

## The town

- **The centre is the fab, together with what made Shenzhen:** a parts market, PCB fabrication
  and assembly, a machine shop, and the bench's test gear. The fab alone is a lab of a handful of
  people, not a town's anchor; that is a judgment, `unknown`.
- **Its buildings use brownfield-compute's building tools, shared rather than copied.** The Homer
  City analysis stays in brownfield-compute, whose reports serve its townships.
  - Where a tool belongs in CortenForge, it goes there as a suggestion. Brownfield-compute's own
    `docs/CORTENFORGE.md` already lists several, from routing to IFC export.
  - Here CortenForge is used through its published crates, as an end user, with what's missing
    logged in `docs/CORTENFORGE.md` once code meets it.
- **Sites with a big grid connection suit the heaviest steps.** Brownfield-compute's retired
  coal plants are an example, and smelting is the step that would need one.

## Open decisions, Jon's

- The safety rules above, and their wording in CLAUDE.md.
- Mains: whether bought, certified equipment fits CLAUDE.md's rule, or the fab stops before its
  first furnace, at step 5.
- Brownfield-compute's building work: shared or moved, and in which language here, since
  CLAUDE.md keeps long-lived code in Rust.
- Upstream: whether via-balaena/tentzhen takes the wider scope.
- The training's compute (`littlebit.md`, "The training needs more compute").
