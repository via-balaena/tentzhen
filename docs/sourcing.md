# Sourcing

Research date for everything below: 2026-09-28, unless a line says otherwise.

## Where each kind of part comes from

**AliExpress (commodity, low risk):** Colorlight i9 + extension board (Muse Lab official store
only), AD9226 modules (buy 3, keep the best 2), DAC module, SMA/BNC connectors and cables, wire,
headers, general-purpose passives. What `builds/digitizer/v1.toml` lists. 10x probes wait for the
front end (tier 3): the modules' inputs are about 50 Ω (github.com/wavius/Scoped, read
2026-10-01), and a 10:1 passive probe "is intended to be connected to the 1 megohm (MΩ) input
termination of the oscilloscope" (Art Pini, "Selecting a Replacement Oscilloscope Probe is
Easy—When You Know How", Digi-Key, 2021-12-27).

**LCSC or authorized distributors (quality-critical):** front-end op amps, precision
resistors/capacitors in the signal path, LM4040 voltage reference, low-jitter oscillators,
signal relays, Pico boards, anything security-related.

**JLCPCB:** front-end PCB fabrication and SMD assembly with LCSC parts, so anyone can reproduce
the board from published files.

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
  with how far each thing has got, and what we measured on the parts that arrived. Screenshots, photos, titles, descriptions, prices and reviews stay out,
  whichever the shop (`CLAUDE.md`).
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

Every received part is tested against genuine references and logged in DuckDB: part, seller,
order, measurements, pass/fail. The seller-quality data we publish is **our own measurements
only** — never LCSC-retrieved data.
