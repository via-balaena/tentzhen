# The open fab, and the town around it

Project #4 (Jon, 2026-11-24) is "a fully open source ASML/TSMC engineering project". It is built
up to scrappily and in public, to show what is possible.

- **What the fab is for:** it builds computers that resolve conflict before it happens. They are
  the chips of project #3, which run simulations of trained agents more cheaply per game step
  than a GPU (`littlebit.md`, "What it is for" and "The goal").
- **The town:** a town grows around the fab, the way brownfield-compute's town
  (YoungMoolahBaby/brownfield-compute) grows around its data center, and a city like Shenzhen
  grows from there.
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
