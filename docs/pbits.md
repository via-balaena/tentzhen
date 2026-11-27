# Project #6: p-bits

Research date for everything below: 2026-11-27, unless a line says otherwise.

Probabilistic bits, p-bits, wired together into a small thermodynamic computer: first as gateware
on the bench's Colorlight i9, then fed by a physical noise source, then as a chip. Jon,
2026-11-27: "given my interest in microcontrollers and previous work with thermodynamic computing
engineering/simulation in cortenforge, this is attractive to me." The plan came from Jon's Claude
session the same day ("Where this came from", below). Nothing is built or ordered, and what each
piece would do is `unknown` until it earns a grade.

## What a p-bit computes

CortenForge's Gibbs sampler (`cortenforge-sim-thermostat` 0.9.0, `src/gibbs.rs`) states it. Spins
σ ∈ {−1, +1} carry the energy

    H(σ) = −Σ J_ij σ_i σ_j − Σ h_i σ_i

and each update sets one spin to +1 with probability "sigmoid(2·local_field_i / kT)", where spin
i's local field is h_i plus the sum of J_ij σ_j over its neighbours (`sweep`, which updates the
spins in turn). A p-bit is hardware that makes that update. CortenForge's `exact_distribution`
(`src/ising.rs`) enumerates the distribution H gives, for up to 20 spins ("safety limit: `2^20`
= 1M configurations"), and CortenForge checks its sampler against it (`tests/gibbs_sampler.rs`,
"Validates the `GibbsSampler` against `exact_distribution`"). It is what a small network's
measured histogram is checked against here.

## Where it sits

- **CortenForge keeps the physics.** Its thermodynamic computing line
  (via-balaena/CortenForge at 2a5cc70b, `docs/thermo_computing/`) builds "mechanical
  thermodynamic computing devices", and says the computing element "is **not** a transistor",
  since "Extropic builds p-bits out of transistors because they have a fab. CortenForge has
  cf-design and a 3D printer." Its physical p-bit, "D4", a spring-steel cantilever on a shaker read
  by Hall sensors, stays there.
- **Tentzhen builds the electronics:** p-bits in logic on the i9, then in transistors, on an
  open-PDK shuttle or in our own process (`fab.md`, "The steps"). Tentzhen uses CortenForge as an
  end user, through its published crates (CLAUDE.md).
- **The i9 is on the bench already** (`lab/sourcing.toml`), so the first steps need nothing
  bought.

## What already exists

The sources are in `prior-art.md`, "p-bits and thermodynamic computing".

- **p-bits on FPGAs are well trodden, on Xilinx parts with Vivado.** The sparse Ising machine
  (Aadit et al., arXiv 2110.02481) updates its p-bits in graph-coloured parallel, makes each
  one's randomness with "32-bit linear-feedback shift registers", maps tanh with "a lookup table",
  and flips with "a comparator", reaching "143.8 flips/ns" at 4,793 p-bits. pc-COP puts 2,048
  fully connected p-bits on a ZCU104. Every FPGA p-bit design read that names its part is on a
  Xilinx or AMD FPGA, and quantinfo's open Verilog (MIT) needs "Vivado version: 2023.2
  Enterprise".
- **Invertible logic is the classic demonstration.** Camsari et al. (Phys. Rev. X 7, 031014,
  2017) show "a 4-bit multiplier which in inverted mode functions as a factorizer", and that "p-bits
  need not be magnet-based". Borders et al. (Nature 573, 390, 2019) factored integers with p-bits
  made from magnetic tunnel junctions.
- **In Rust,** ferrotherm (Apache-2.0) does thermodynamic computing in software, with an invertible
  multiplier its README calls "shipped, verified", and has layers of its own that emit FPGA
  bitstreams for the iCE40-HX8K and Artix-7. It has no ECP5 layer, and "Still not done: binding a
  p-bit fabric's LUTs to SLICE site pins".
- **On a shuttle,** WobblyBits (Apache-2.0, Tiny Tapeout, SKY130) is "6 p-bits ... driven by
  hardware randomness from a bunch of on-chip ring oscillators", taken from neoTRNG. Its results
  so far are "the RTL results".
- **Thermodynamic computing companies:** Extropic measured a transistor random-bit cell at about
  10 MHz, using about 350 aJ per bit; the rest of its paper was "done using simulations of
  thermodynamic hardware". Its code, thrml, is Apache-2.0.
- **True random numbers from ring oscillators** are open too: neoTRNG (BSD-3-Clause, implemented
  on an Artix-7, with no "online health monitoring"); a ring-oscillator TRNG for the ULX3S's ECP5,
  with no licence; and cryptoliteICE, whose RP2040 runs "SP 800-90B
  health tests" on an iCE40's ring oscillators.

**What nothing we found does:** p-bits on an ECP5, written in Amaranth and built with Yosys, nextpnr
and Trellis; a p-bit core checked bit for bit against an open Rust model; and p-bits fed by an
on-chip entropy source that runs SP 800-90B's health tests in the same gateware. That is the part
built here. A p-bit made in a process older than 180 nm, or in an open fab, also turned up nowhere.

## The steps

Each is a proposal, `unknown` until it earns a grade.

| step | what | passes when |
|---|---|---|
| 1 | **A golden model,** in Rust: p-bits in integer arithmetic, with the random-number generator and the sigmoid's table or threshold that the gateware will use | on networks of up to 20 spins, its samples' histogram is within a distance of `exact_distribution` from `cortenforge-sim-thermostat` 0.9.0, the distance and the sample count set in the test before it first runs (`tested`) |
| 2 | **A p-bit core** in Amaranth | in simulation it draws exactly the golden model's samples, from the same seed (`tested`) |
| 3 | **p-bits on the i9** | a board run's histogram matches `exact_distribution` within the same distance, recorded as a measurement (`measured`) |
| 4 | **True randomness:** ring oscillators on the ECP5, with SP 800-90B's health tests in the gateware, feeding the p-bits | the health tests run on the board and the p-bits' histogram still matches (`measured`); what the source's entropy is stays `unknown` until it is estimated |
| 5 | **A p-bit chip,** on a shuttle and then in our own process | its histogram matches (`measured`); a shuttle costs money, which is Jon's decision (`fab.md`, "The steps") |

- **The first networks** are small enough to enumerate, as CortenForge's `exact_distribution`
  requires: a ferromagnetic ring, a max-cut on K3,3 as WobblyBits runs, and an invertible AND
  gate. A multiplier run backward is the showcase after those; ferrotherm's 4×4 multiplier takes
  176 spins, and whether that many fit the i9 beside the rest is `unknown`.
- **The random-number generator** for steps 1–3 is not chosen. LFSRs (the sparse Ising machine,
  pc-COP) and Xoshiro (Nikhar et al.) are what the FPGA designs read use; the golden model fixes
  whichever is chosen.
- **The noise source of step 4** is not claimed fit for keys. SP 800-90B asks that its health
  tests "shall include both continuous and start-up tests", run "before any conditioning is
  done", and that on a persistent failure "the entropy source shall not produce any outputs".
- **Loading the i9** follows the lab-target rules, as project #3's board runs do (`littlebit.md`).
  Steps 1 to 4 handle no chemicals, heat, high voltage, vacuum or pressure; step 5's own process
  is the fab's, under its safety rules (`fab.md`, "Safety first").
- **Later, a dev kit:** the i9, its extension board, the core and the golden model, published
  together, as the session recommends ("Where this came from").

## What it could run

| application | what exists | here |
|---|---|---|
| invertible logic: a multiplier run backward to factor | Camsari et al. 2017; Borders et al. 2019; ferrotherm's multiplier, in software | the showcase, after the small networks |
| max-cut and SAT | pc-COP and the sparse Ising machine, on Xilinx parts; WobblyBits' K3,3, in RTL | a first network |
| true random numbers | neoTRNG; cryptoliteICE's health tests | step 4 |
| Bayesian inference and energy-based sampling | Faria et al.'s autonomous Bayesian networks (titles read only); Extropic's denoising models, simulated | later, with CortenForge's `IsingLearner` for Boltzmann learning |
| Wave Function Collapse | nothing found on p-bits or Ising machines | later |

## Where this came from

Jon's Claude session of 2026-11-27, whose summary Jon brought the same day. It set out a
thermodynamic chip as the strongest candidate for a young fab, a path ("your sims as the
reference model, then p-bits on an FPGA, then fabbed devices using real physical noise"), its
applications, and a dev kit with a built-in RNG as the launch. Kept: the path, the applications
and the dev kit. Changed here:

- **The path gains a golden model and a noise step.** A Rust model bit-exact with the gateware
  stands between CortenForge's sampler and the i9. CortenForge's sampler draws from a ChaCha8
  generator in floating point (`src/gibbs.rs`), which the gateware is not planned to repeat, so
  the two are compared by their distributions. True randomness is its own step, with health tests.
- **"It suits a young fab especially well, because noisy, imperfect devices are a feature rather
  than a defect":** kept as a question, `unknown` until a device shows it. The oldest process a
  p-bit chip was found in is 180 nm, with RRAM.
- **"True random number generation (simplest and most sellable)":** kept as step 4; "most
  sellable" is left out, with no source.
- **The chips before it** (process test chips, a RISC-V microcontroller, hardwired classifiers, a
  neural physics surrogate, analog compute-in-memory) are not taken up here; `fab.md`'s own steps
  stand.
- **Left out, with no source found:** "Nvidia reportedly cutting gaming GPU supply over memory
  shortages". The physics card and the neural-physics plugin are CortenForge's, or a product's,
  not Tentzhen's. The resistor scanner fits project #5's intake as a parts sorter, once the stores
  exist.

## What was searched

On 2026-11-27, one search agent and my own reads. Searched: 156 queries in the agent's log (68
GitHub repository searches, 44 on arXiv, 10 GitHub code searches, 14 on Gitee, 14 on OSHWHub, 5 on
Bilibili and 1 on Crossref), 45 of them empty, and no web searches. Read: method sections of five
FPGA papers, of Extropic's paper and of Normal Computing's two (its stochastic processing unit and
CN101), about 30 READMEs, NIST SP 800-90B, and Tiny Tapeout's project index. The rows that
`prior-art.md` marks **re-read** were read again in their own text. Not seen: full Nature texts,
IEEE and ACM papers, any FPGA p-bit code that is not public, and Hacker Fab's sources, which were
not searched for p-bits. arXiv's search matches abstracts and GitHub's repository search matches
names, descriptions and topics, so this shows what was found, not that nothing closer exists.
