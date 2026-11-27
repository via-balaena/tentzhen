# Project #5: a makerspace's stores

Research date for everything below: 2026-11-26, unless a line says otherwise.

A makerspace's parts stores, stocked from AliExpress and the rest of the surplus, and a wired
pick-to-light of our own that lights the bins a build needs. From there the stores grow toward the
records a fab keeps: chemicals by lot, then wafers by lot, for project #4 (`fab.md`, "Running the
fab"). The plan came from Jon's Claude session of 2026-11-26 ("Where this came from", below).
Nothing is built or ordered, and what each piece would do is `unknown` until it earns a grade.

- **Where it sits.** The town's centre is to be the fab, "together with what made Shenzhen: a
  parts market, PCB fabrication and assembly, a machine shop, and the bench's test gear" (`fab.md`,
  "The town"). The stores are a first piece of that parts market.
- **What the repo already keeps,** for the things its builds need: a part's claims (`parts/`);
  where a thing came from and how far it has got, through incoming QA (`lab/sourcing.toml`,
  `sourcing.md`); measurement records (`lab/records/`); and the append-only log of hardware
  actions (`lab/log/`).

## What already exists

The sources are in `prior-art.md`, "A makerspace's stores".

- **Inventory software with a locate button.** InvenTree (MIT) has a plugin interface for it,
  `LocateMixin`, and inventree-wled-locator (MIT) uses it to light one WLED controller's LEDs.
  WLEDger (AGPL-3.0) lights bins through several WLED controllers on the network, and already
  does BOM kitting: its "Pick Mode" lights every bin a project needs at once. It also has roles
  and "full audit logging". Part-DB has users and groups, and Binner keeps a part's moisture
  sensitivity level as text. A code search finds no bin lighting in InvenTree's, Part-DB's or
  Binner's own code, and none of the four counts down a part's floor life (WLEDger's README names
  no moisture level).
- **A wired wall exists off the shelf:** WLED builds for 17 Ethernet board types.
- **So does pick confirmation, apart.** CAN nodes on PIC18 chips light a location and send an
  acknowledgement back: "Operator picks item / Client sends ACK back to server / Server confirms
  completion" (SakthiArumugam642/Pick_To_Light, with no licence). Pick2Light (2017, no licence)
  wires "182 LEDs, 182 Buttons" to Arduinos on I2C. irvan2004juanda-bit/Pick-To-Light (MIT)
  gives each of its 8 modules a confirmation button ("Konfirmasi pengambilan"), over Wi-Fi and
  MQTT.
- **A floor-life clock per lot, apart:** an unlicensed inventory data model keeps
  `floor_life_seconds_remaining`, a bake history and reels split into cut tape, with no lights.

**What no project we found does:** carry power and the bus in one cable to identical shelf nodes,
send a pick confirmation back to an inventory system others use, such as InvenTree or WLEDger, give
each member their own colour, and count down each reel's floor life, in one open design. That is the
part built here. The store, the kitting and the LEDs' API are used as they are, and credited.

## The stores

As the session laid them out, to be checked before anything is bought:
- **Bins:** dividers or Gridfinity for small bags and cut tape; larger ESD bins for modules and
  dev boards; a sealed box with desiccant for moisture-sensitive chips.
- **Intake:** a label of our own, printed as each thing arrives, with our own identifiers.
  Whatever of it goes in the repo follows CLAUDE.md's rules on shop data.
- **A search kiosk** anyone can use, on the lab network ("Safety", below).
- **Low-stock alerts.**
- **Members put things back in the wrong bin,** and the session asks for lights that tolerate it.
  How the wall finds a part in the wrong bin is not designed.

## Two kinds of record

The repo records what a part is, where it came from and whether it passed incoming QA. A store's
counts and bins change at every pick, and a commit for each would put every pick in the repo's
history. So:
- **the repo** keeps what a part is and whether it passed incoming QA;
- **a live store,** on the lab network, keeps counts, bins and picks, and takes each part's
  identity from the repo, so that the two never name one part two ways.

`lab/sourcing.toml` holds only what a build needs (its header says so), and a makerspace stocks
much that no build names. That stock's records would be a new file, with its schema and loader
written before anything reads it (CLAUDE.md, "Conventions"). It is not designed.

**The live store is WLEDger, and InvenTree if WLEDger fails a measure** (Jon, 2026-11-26, taking
the recommendation). The measures, named before it is tried: it takes each part's identity and QA
result from the repo, with no listing content; it runs on the lab network with no internet; it
drives the gateway through WLED's API; and it takes a pick confirmation back through its own API.
Neither store is the point of the project. WLEDger is AGPL-3.0, under which a modified copy "must
prominently offer all users interacting with it remotely through a computer network ... an
opportunity to receive the Corresponding Source" (its LICENSE, section 13).

## The parts wall

### A gateway that speaks WLED's API

The two locators made for InvenTree and WLEDger drive their LEDs through WLED's JSON API:
- inventree-wled-locator POSTs each LED's colour to `/json/state` (`WledPlugin.py` at 12c499ff);
- WLEDger reads `/json/info` and `/json`, and POSTs to `/json/state` (`internal/wled/client.go`
  at 9ca719eb).

So the gateway answers those calls on Ethernet, as WLED controllers do, and drives the shelves
behind it. WLEDger, or InvenTree if it comes to that, would then drive the wall unchanged, with no
code of ours in Python or Go (CLAUDE.md, "Language policy"). Whether a gateway
that answers only those calls satisfies both is `unknown` until a test runs their calls against
it.

A pick confirmation travels the other way, from the wall to the store, and WLED's API has no call
for it. It goes through the store's own API, which calls is `unknown`: InvenTree's, or WLEDger's
`/api/v1`, whose README lists "stock adjustments".

The gateway is host software, so it would be written in Rust (CLAUDE.md).

### The bus

The session's design: CAN over Cat5 or Cat6 patch cables, carrying 24 V from the gateway to every
shelf.

- **The pinout:** CiA 303-1's RJ45 assignment, as HMS Networks relays it: 1 CAN_H, 2 CAN_L,
  3 and 7 CAN_GND, 6 an optional shield, and 8 an optional "external voltage supply". CiA's own
  text is behind a login. The pinout gives power one pin, 8; whether one conductor of a patch
  cable carries a shelf run's current is `unknown` until the run's current is worked out.
- **A cable that fits a laptop.** A shelf cable with RJ45 plugs also fits an Ethernet port. On
  plugging an always-powered "Passive PoE" port into a product not built for power, the Ethernet
  Alliance writes: "Usually, you let the 'magic smoke' out of some termination components". A PoE
  supply keeps its port unpowered while it "periodically looks to see if something has been
  plugged in", at power levels "unlikely to damage devices not designed for PoE" (TI's TPS2378
  data sheet, which says its account is no substitute for IEEE 802.3at). So the bus either uses a
  connector that cannot mate with an Ethernet port, or powers a port only once it detects a node.
  Which is not designed.
- **Power:** 24 V from a bought supply on mains. CLAUDE.md's mains rule: bought mains equipment
  "listed by a recognised testing lab may be used as sold, where it is legal to use there, under
  its step's safety plan". The stores are not one of `fab.md`'s steps, so whether they need a
  written plan is open (below). How each shelf's share is fused is not designed.

### The nodes

Every shelf has the same node: a microcontroller, a CAN transceiver, a buck converter from 24 V,
and addressable LEDs, all running the same firmware.

- **Language:** CLAUDE.md's table has no row for a shelf node. The proposal is the Pico's row:
  Rust on Tock, and Embassy where Tock doesn't fit, with the reason beside it (an open decision,
  below).
- **CAN in Tock:** at b89939e0, tock/tock has a CAN interface (`kernel/src/hil/can.rs`), a
  capsule for it (`capsules/extra/src/can.rs`) and a driver for STM32F4 chips
  (`chips/stm32f4xx/src/can.rs`). Of the five STM32 boards read (`nucleo_f429zi`,
  `nucleo_f446re`, `stm32f429idiscovery`, `stm32f412gdiscovery`, `weact_f401ccu6`), only
  `nucleo_f429zi` wires it up.
- **CAN on the RP2350,** which the lab already has: can2040 (GPL-3.0) runs CAN in the chip's PIO
  and has supported "rp2350 chips" since v1.7.0. It is C, and of the three error states the CAN
  specification defines, it "will not automatically enter a 'bus off' state" (`docs/Features.md`
  at 2988d4f1). A node on it would carry GPL-3.0 code, and whether Tock can host it is `unknown`.
  A microcontroller with a CAN controller of its own is the other route.
- **Flashing** a node is a hardware action, and logged. Flashing one not listed in
  `lab/targets.toml` needs a person's approval (CLAUDE.md).

## Picking, kitting, reels and moisture

| feature | where it exists | here |
|---|---|---|
| BOM kitting: every bin for a build, lit at once | WLEDger's "Pick Mode" | used as it is |
| an audit log | WLEDger's "full audit logging"; NEMO's record of each consumable withdrawn | the store's |
| low-stock alerts | WLEDger's stock-health colours and its API's "low-stock checks" | the store's |
| pick confirmation, on the shelf | the PIC18 CAN nodes; Pick2Light's buttons; irvan2004juanda-bit's buttons | ours, sent back to the store |
| a colour per member, at once | none found | ours |
| reels on ESD racks, a dry cabinet | sold as products | bought |
| moisture floor life, per reel | an unlicensed data model; nothing that lights bins | ours |

**Floor life,** as TI's note gives it (SPRABY1B, Table 2-1, "Factory Floor Life @ 30°C"): level 1
unlimited, at 85 % RH; at 60 % RH, level 2 one year, 2a four weeks, 3 168 hours, 4 72 hours, 5 48
hours, 5a 24 hours; level 6 "Bake before use and reflow within time on label". Bourns' table
(`msl_mf.pdf`, for its Multifuse parts) gives the same, at ≤ 30 °C, and counts a level 2a part's
four weeks "from the date the bag was opened". Espressif counts floor life from "removal from a
moisture barrier bag, dry storage or dry bake". What time back in a dry cabinet does to the clock
is in J-STD-033, which is paywalled and unread here.

**Baking,** past floor life: TI points to J-STD-033C's drying table and says "Tape and Reel
material should not be baked above 40°C". Espressif bakes parts once they are "taken out of the
tape".

## Safety

- **Power:** 24 V DC from a bought supply on mains, under CLAUDE.md's mains rule ("The bus",
  above). The project designs and wires no mains equipment (CLAUDE.md).
- **Network:** the store, the kiosk and the gateway run on the isolated lab network. Exposing any
  of them beyond it needs approval (CLAUDE.md).
- **Flashing:** a node or the gateway not listed as a lab target is flashed only with a person's
  approval, and each flash is logged (CLAUDE.md).
- **Members:** a log of who took what is a record about people. What it keeps, and for how long,
  is open (below).
- **Baking parts dry** is heat, from an oven on mains, under the same mains rule.

## Open decisions, Jon's

1. **The shelf node's row in CLAUDE.md's language table:** Rust on Tock, and Embassy where Tock
   doesn't fit, as the Pico's row says.
2. **What the pick log keeps about members,** and for how long.
3. **Whether the stores' mains equipment needs a written safety plan:** the 24 V supply, and an
   oven for baking parts dry. CLAUDE.md's mains rule uses bought mains equipment "under its step's
   safety plan", and the stores are not a fab step.

## Where this came from

Jon's Claude session of 2026-11-26, whose summary Jon brought the same day. It set out a
makerspace's stores, a wired pick-to-light as the open-source contribution, a growth path, and a
bridge from Hacker Fab toward the industry. Kept: the storage, the must-haves, the bus and the
nodes, the growth path, and the fab's list of practices with its route (`fab.md`, "Running the
fab"). Changed here:

- **"None do a wired bus, pick confirmation, BOM kitting, or reels/MSL":** each exists on its
  own. WLEDger does BOM kitting; WLED runs on Ethernet boards; the PIC18 CAN nodes and Pick2Light
  confirm picks; an unlicensed data model counts down floor life. What none does is all of them in
  one open design ("What already exists", above).
- **"All are single-strip Wi-Fi 'find one bin' projects":** WLEDger drives several WLED
  controllers, and WLED runs on Ethernet boards too.
- **An InvenTree plugin talking MQTT to the gateway** became a gateway that speaks WLED's API, so
  either store can drive the wall and no code of ours is in Python or Go (CLAUDE.md, "Language
  policy").
- **"The hard, truly industrial parts are pick confirmation and MSL tracking. The LEDs are the
  easy part":** not kept, since no source for which parts are hard was found. Both are among the
  parts built here.
- **The bridge to the fab** starts from Hacker Fab's own process database, read in `fab.md`,
  "Running the fab". The session's list of practices is there.
- **Left out, with no source found:** "AliExpress bags lack useful barcodes"; "Pick'n'Light" and
  "Arduino Storage Cart", not found on GitHub by name; "the Shenzhen-lab model"; and "The main gap
  between hobby fabs and industry is operational discipline, not equipment, and most of it is
  cheap".

## What was searched

On 2026-11-26, one search agent and my own reads. Searched: GitHub through its API (over 100
repository searches and 15 code searches in the agent's log), Gitee, OSHWHub and Bilibili in
Chinese, and nine web searches. About 40 READMEs and pages were read. The rows that `prior-art.md`
marks **re-read** were read again in their own text. Not seen: J-STD-033, CiA 303-1 and IEEE 802.3,
which are paywalled or behind a login; Hackaday.io's project pages, which need a browser; NEMO's
plugins; Coral's source; and commercial pick-to-light systems beyond a search's snippets. GitHub's
repository search matches names, descriptions and topics, not READMEs, so this shows what was found,
not that nothing closer exists. The tables are in `prior-art.md`, "A makerspace's stores".
