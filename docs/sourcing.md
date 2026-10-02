# Sourcing

Research date for everything below: 2026-09-28, unless a line says otherwise.

## Where parts come from

Anywhere they are cheap, if incoming QA can check what the build relies on (`vision.md`, principle
3). In this order:

- **The maker's own store**, when it sells the part, even at a higher price: the i9 and its
  extension board are to come from the listing that Muse Lab's README (wuxx/Colorlight-FPGA-Projects
  at 5042201f, read 2026-10-01) links under "our aliexpress store".
- **Anyone else:** a reseller, a used sale, a thrift store, a junked device, a drawer at home.
  `lab/sourcing.toml` records each as a shop's listing, a `store` with no listing, `used`,
  `salvaged_from` a device, or `arrived = true` with no source. Until incoming QA measures a part,
  its claims rest on a `trusted-base.toml` entry that says where that trust comes from:
  `maker-datasheets` for a part from an authorized distributor, `resold-parts` for one from anyone
  else, and `aliexpress-modules` for the AD9226's and the AD9767's, which come soldered to modules
  bought on AliExpress. A sourcing line for a part whose claims rest on `maker-datasheets` says
  where the maker lists its seller (`authorized`), or `crates/records` refuses it.
- **An authorized seller only**, for a part whose job no bench test on arrival can check, including
  one whose spec is finer than the bench can measure: the parts marked `authorized_only` in
  `parts/`.

A part may come out of a junked mains-powered device, but the repo carries no instructions for
taking one apart: `CLAUDE.md` keeps mains voltage out of scope.

**What the digitizer needs** (`builds/digitizer/v1.toml`): the Colorlight i9 and its extension
board, AD9226 modules (buy 3, keep the best 2), a DAC module, SMA/BNC connectors and cables, wire,
headers and general-purpose passives. 10x probes wait for the front end (tier 3): the modules'
inputs are about 50 Ω (github.com/wavius/Scoped, read 2026-10-01), and a 10:1 passive probe "is
intended to be connected to the 1 megohm (MΩ) input termination of the oscilloscope" (Art Pini,
"Selecting a Replacement Oscilloscope Probe is Easy—When You Know How", Digi-Key, 2021-12-27).

**The front end's parts** (tier 3, no build yet): the ADC (an ADS4245, `scope.md`), op amps,
precision resistors and capacitors in the signal path, the LM4040 voltage reference, low-jitter
oscillators and signal relays. Which of them a budget bench can check on arrival is decided when
their records are written: a resistor whose tolerance is finer than the multimeter's accuracy
cannot be checked with it.

**JLCPCB:** front-end PCB fabrication and SMD assembly with LCSC parts, so anyone can reproduce the
board from published files.

## The LCSC API

Source: lcsc.com/docs/index.html, lcsc.com/docs/openapi/index.html, lcsc.com/agent.

- **Access is by application**, at lcsc.com/agent/apply after logging in. The form asks for
  the business, its size and purchase volume, the intended use ("for your own use" vs "for your
  customers' use"), the API modules wanted, and a **required IP allow-list** — calls from any
  other address are refused. Whether an individual or an open-source project is approved is
  `unknown`. Via Balaena applied on 2026-09-28 for its own use, Product and Cart modules only;
  not yet approved.
- **Auth:** `key`, `nonce`, `timestamp` and `signature = sha1(key=…&nonce=…&secret=…&timestamp=…)`.
  Timestamps older than 60 s are refused. Host: `https://ips.lcsc.com`.
- **Services:** category, brand, category product list, product info
  (`/rest/wmsc2agent/product/info/{C-number}`), keyword search (30 per page), submit order, order
  query, shipment options. The response schema for price breaks, stock and datasheet URL is
  `unknown` until we hold a key.
- **Limits:** 1000 searches/day, 200/minute (more on approval). HTTP 429 = per-minute, 430 = daily.
- **Terms that shape the design:** no bulk data capture; no hosting or providing retrieved
  material (including datasheets and images) to any third party; no selling or aggregating it
  into public APIs or data services; LCSC must be credited; the key may not be shared outside
  the holder's company.

### What that means for Tentzhen

- The repo ships a **client**, never data. Each user brings their own key; responses are cached
  locally under the user's data dir and never committed.
- Parts records in the repo hold only identifiers (`C` numbers, MPNs) and our own measurements.
- The client does not implement `submit order`. Carts and BOMs yes, orders no (`CLAUDE.md`).
- Output that shows LCSC data credits LCSC.
- Keys come from the macOS Keychain or the environment, never a file in the tree.
- **Without a key** the tools must still work, and this is the path built first: jlcparts
  (below) for search, stock, prices and JLCPCB basic/extended status, and LCSC's website **Upload
  a BOM** page for carts — the BOM tool exports a file, a person uploads it. The API is an upgrade
  on top: live stock at checkout and product-change notifications.
- **An API rejection may mean the address changed, not the key.** When the client gets one, it
  prints the machine's current public address beside the error.

## JLCPCB

- **Parts API:** by application at api.jlcpcb.com, reviewed against "previous orders at JLCPCB,
  company and business situation" — not all are approved (jlcpcb.com help, updated 2026-09-09).
  Offers PCB, stencil, 3D printing and components APIs. An assembly-ordering API is `unknown`.
  Rate limits and data-reuse terms are behind the portal login: `unknown`.
- **Basic vs extended parts drive assembly cost** (jlcpcb.com/help/article/pcb-assembly-faqs,
  2026-09-09): basic parts are always loaded on the machines, so no loading fee; extended parts
  cost $3 each, charged per **unique** part. So the design rule is: prefer basic parts, and have
  the BOM tool report the count of unique extended parts, because that number is a cost.

## AliExpress and Amazon

AliExpress's Terms of Use (the version effective 2026-09-26, read 2026-10-01), §3.2(a): a user
will not "copy, reproduce, download, re-publish, sell, distribute or resell" its listings, and
"Systematic retrieval of Site Content from the Sites to create or compile, directly or
indirectly, a collection, compilation, database or directory (whether through robots, spiders,
automatic devices or manual processes) without written permission from AliExpress.com is
prohibited." A search on 2026-10-01 found no open catalogue like jlcparts for AliExpress, only
scrapers and scraping services, which that clause rules out.

Amazon's Conditions of Use (last updated 2026-08-14, read 2026-10-01), "License and Access": "This
license does not include … any collection and use of any product listings, descriptions, or
prices; … or any use of data mining, robots, or similar data gathering and extraction tools."

### What that means for Tentzhen

- A person reads a listing. No tool here fetches or collects listings.
- The repo holds a listing's item number (on Amazon, its ASIN) and store, in `lab/sourcing.toml`
  with how far each thing has got, and what we measured on the parts that arrived. Screenshots,
  photos, titles, descriptions, prices and reviews stay out, whichever the shop (`CLAUDE.md`).
- AliExpress's affiliate API, on an affiliate account and an approved app, is the route with its
  permission. Its agreement ties the API to promoting products, "for the purpose of driving the
  traffic for Seller(s)" (AliExpress Affiliate Program Service Agreement, read 2026-10-01); what
  it returns is `unknown` here.

## Tools to reuse, not rebuild

- **jlcparts** (github.com/yaqwsx/jlcparts, MIT): rebuilds a searchable JLCPCB/LCSC catalogue
  three times a day and publishes JSONL and SQLite at yaqwsx.github.io/jlcparts/data/. It fetches
  with official API keys; how its publishing squares with LCSC's no-redistribution term is
  `unknown`, so we **link to it and query it at runtime; we do not vendor its data.**
- **KiCad:** Bouni/kicad-jlcpcb-tools (MIT) assigns LCSC numbers per footprint;
  bennymeg/Fabrication-Toolkit (Apache-2.0) reads a symbol field named `LCSC Part #`. We use that
  field name.
- **Rust:** no crate wraps the official LCSC API (crates.io search, 2026-09-28). Ours will be
  `crates/lcsc`.

## Incoming QA

Every received part, whatever its source, is checked on the bench against what its build relies on.
The check is a measurement record in `lab/records/`, and `passed_qa` in `lab/sourcing.toml` names
it, so the parts page shows the result beside the source the part came from. The seller-quality data
we publish is **our own measurements only**: never LCSC-retrieved data, and never a listing's
content.

### The parts sold by authorized sellers only

Reviewed 2026-10-02 against the rule above: a part's mark comes off when a bench test on arrival
covers what its build relies on. A unit bought from anyone but an authorized distributor rests its
claims on `resold-parts`, not `maker-datasheets`, which assumes one: `lab/sourcing.toml`'s
`authorized` says where the maker lists a seller, and `crates/records` refuses a line that names a
source or a stage without it, for a part sold by authorized sellers only, or for one whose claims
rest on `maker-datasheets` unless they move to `resold-parts`. The AO3401A and the AO3400A have lost
the mark; the INA239, the TLV3011B, the RP2350 and the RP2040 keep it, each for the reason it gives.
The tests below are worth running on any unit, whatever its source, since they catch a wrong or
remarked part. Passing them shows a part fit for what they check, and no more: "No amount of testing
can confirm an item as authentic" (SAE AS6171A, scope, revised 2018-04-18). Each test is a hardware
action, logged; an agent runs one only within `lab/limits.toml` or on a person's approval
(`CLAUDE.md`). Code a Pico runs to read a part counts as flashing the Pico (`lab/targets.toml`), so
it needs a person's approval until the Pico is a lab target.

- **INA239**, the enforcer's measurement. Read MANUFACTURER_ID (3Eh), which reads 5449h, "TI" in
  ASCII, and DEVICE_ID (3Fh), which reads 2391h (TI SLYS027A, Tables 7-20 and 7-21); then compare
  its bus-voltage reading with a multimeter's near 3.3 V. **Keeps the mark:** the enforcer's
  `measuring-error` claim is 9.97 mV at 3.4 V, and a multimeter checks the INA239 only as finely as
  its own accuracy there, which no record in the repo gives (`unknown`). A meter or reference finer
  than that, with a record, would lift it.
- **TLV3011B**, the hardware cut. With V+ at 3.3 V, measure REF (pin 5) to ground: 1.223 V to
  1.260 V (TI SBOS300C, section 6.9). Then check the output is open-drain: with IN- below REF and
  OUT pulled to ground through 100 kΩ, OUT reads near 0 V, where a push-pull part reads near V+. Its
  push-pull twin, the TLV3012, would hold EN high whenever it is not tripped, within 200 mV of V+
  while sourcing 5 mA (VOH, at VS = 5 V; no row is given at 3.3 V), more than GP20 pulls through
  1 kΩ, so GP20 could not open the switch (from the circuit in `builds/enforcer/v1.toml`). **Keeps
  the mark:** the `hardware-cut` claim holds across a room within 10 K of 25 °C, and no record in
  the repo gives a thermometer or a way to hold a part at a set temperature, so how a unit's
  reference moves with temperature goes unmeasured.
- **AO3401A**, the enforcer's switch, and **AO3400A**, its driver. A diode test finds each one's
  orientation, and a resistor and the supply its gate threshold at 250 µA: 0.5 V to 1.3 V below the
  source for the AO3401A, 0.65 V to 1.45 V above it for the AO3400A. If the DPS5005 fails, SUPPLY
  can rise toward the 6.5 V that `[supply.upstream]` feeds it, and the AO3401A must then block it
  (`switch-blocks-upstream`), while the AO3400A, off, holds it on its drain through 100 kΩ (from the
  circuit). So, with its gate at its source, each passes under 1 µA at 6.5 V, the bench supply's own
  setting (IDSS, rated at 30 V and 25 °C). And the AO3401A, with its gate 2.5 V below its source,
  drops under 17 mV at 200 mA, `[supply]`'s `max_amps` through its 85 mΩ, which the datasheet rates
  at 2.5 A (Alpha and Omega datasheets, Rev 3.1). A person can run these; an agent cannot without
  approval, since `lab/limits.toml` sets no limit for a part on the bench. **Mark off**
  (2026-10-02): these tests cover what the enforcer relies on from the two, at the bench's
  temperature. A unit not from an authorized distributor rests their claims on `resold-parts`.
- **RP2350**, the enforcer's microcontroller, and the one `builds/debug-probe/v1` runs on. In
  BOOTSEL, read its OTP with picotool, and only read it: writing OTP is irreversible and needs
  approval (`CLAUDE.md`). A blank part's OTP is "all zeroes, except for some basic device
  information pre-programmed during manufacturing test" (RP2350 datasheet, build-date 2025-07-29,
  chapter 13). Whether picotool's reads load code onto the chip has not been checked here, so treat
  them as needing approval too. **Keeps the mark:** no read can show a tampered chip or flash. Micro
  Center is on Raspberry Pi's list of approved resellers for the Pico 2 and the Pico 2 W in the US,
  "for home" (content-api.raspberrypi.com, read 2026-10-02).
- **RP2040**, the microcontroller in the lab's debug probe (`builds/debug-probe/v2`). **Keeps the
  mark,** as the RP2350 does: no read can show a tampered chip or flash, and the probe is one way
  code reaches the lab's other boards. The lab's came from Raspberry Pi itself (Jon, 2026-10-02).
