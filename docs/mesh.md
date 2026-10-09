# Project #2: the mesh

Research date for everything below: 2026-10-08, unless a line says otherwise.

A LoRa mesh of handhelds, repeaters and a phone, for what UMSH calls "decentralized, wide-area text
chat and location/sensor reporting". It runs [UMSH](https://github.com/darconeous/umsh), an open
mesh protocol, on nRF52840 boards, and the project's own work is porting it to Tock (Jon,
2026-10-08). Nothing is ordered yet.

## What UMSH is

Read from darconeous/umsh at 11f1fd25 (README, `firmware/`, `apps/`) and its release
`fw-2026.10.01`:

- **What it is for.** "decentralized, wide-area text chat and location/sensor reporting"
  (README). Nodes are addressed by their Ed25519 public keys; frames are encrypted with AES-SIV.
- **License and language.** Rust, under Apache-2.0 or MIT (both license files at the root), as
  Tentzhen is. The protocol crates are `no_std`, but "`alloc` is still required" (README).
- **Firmware.** The nRF52840 firmware is built on Embassy, not Tock: each board's
  `firmware/<board>/Cargo.toml` names `embassy-nrf`. `fw-2026.10.01` ships images named for eight
  boards: `xiao-nrf52`, `techo`, `t1000e`, `sensecap-solar`, `wio-tracker-l1`, `heltec-v3`,
  `tbeam-supreme`, `tlora-pager`. Its README says "firmware for seven boards".
- **Apps.** An iOS app is in `apps/ios`. An Android app is an open pull request (#11, opened
  2026-10-03).
- **How it was written.** Its README says "practically all of the content in this repository was
  written with the assistance of an LLM", so it is `unknown` here, like ours, until it earns a
  grade.
- **Its GitHub repo** was created 2026-03-15.

## The boards (Jon's table, 2026-10-08)

The roles, boards, chips and reasons are Jon's. The last column is what has been checked so far; the
hardware details in the reasons (displays, GPS, solar wattage, batteries) are not yet read from the
makers' pages.

| role | board | chip / radio | reason | checked |
|---|---|---|---|---|
| dev, the Tock port (2–3) | Seeed XIAO nRF52840 + Wio-SX1262 kit | nRF52840 / SX1262 | cheap, validated, an ideal pair for Tock; no buttons, needs an SWD probe | UMSH ships `xiao-nrf52`; not an upstream Tock board (tock/tock `boards/` at 238630bf) |
| handheld | LilyGO T-Echo | nRF52840 / SX1262 | e-paper display, GPS, low power | UMSH ships `techo`; not an upstream Tock board |
| other handheld | SenseCAP T1000-E | nRF52840 / LR1110 | the board UMSH is most tested on | UMSH ships `t1000e`; "most tested" `unknown` |
| repeater | SenseCAP Solar Node P1 | nRF52840 / SX1262 | solar, 18650 cells, weatherproof | UMSH ships `sensecap-solar`; which variant it runs on is `unknown` (below) |
| phone | iPhone | — | the only UMSH app | iOS app in `apps/ios`; Android in PR #11 |
| debug probe | Pi Debug Probe or a J-Link clone | — | SWD for the Tock work | the lab has a Raspberry Pi Debug Probe (`builds/debug-probe/v2.toml`) |
| not now | Heltec V3, T-Beam Supreme | ESP32-S3 / SX1262 | only to bridge to the internet over Wi-Fi; more power, closed Wi-Fi blobs | UMSH ships both |
| later | a board of our own | nRF52840 / SX1262 + SE050 | keys in a secure element | not designed |

UMSH's records name the repeater two ways: `docs/hardware/boards.json` calls `sensecap-solar` the
"SenseCAP Solar Node P1", and its hardware notes, `sensecap-solar-node-p1-pro-hardware.md`,
describe the P1-Pro, as "a firmware-level reconstruction, not … a verified electrical schematic".
Whether one image runs on both is `unknown`; it is settled before the repeater goes in a cart.

The table's note that the T1000-E's LR1110 is "a weaker fit for Tock" is not kept: upstream Tock
has a board with the same pair, `wm1110dev` (its README lists the nRF52840 and a Semtech LR1110),
and `apollo3/lora_things_plus`, a LoRa board whose README points to RadioLib for the SX1262.
GitHub's code search found both chips' names in tock/tock only in those boards' files and in
meeting notes, so whether either radio has a kernel driver in Tock is `unknown`.

## Open questions

- **How UMSH runs on Tock.** Its crates as a libtock-rs app, over a radio driver in the kernel or
  the SPI bus from userspace, is one shape; whether its need for `alloc` fits a Tock app is
  `unknown`.
- **Which band and power each node may transmit at.** It depends on the region; UMSH has a
  `regions/` directory, not yet read. Flashing a node is a hardware action like any other: logged,
  and only to a lab target (`lab/targets.toml`).
- **A secure element.** The board of our own names an SE050; the bench's research track names a
  TROPIC01 for signing keys (`roadmap.md`, "Parallel tracks"). Whether one part serves both is not
  weighed.

## What was searched

Two web searches (UMSH and LoRa mesh on the nRF52840) and reads of darconeous/umsh and tock/tock
through GitHub's API, 2026-10-08. MeshCore and Meshtastic, the meshes UMSH compares itself to,
are named here and not examined; Chinese-language sources and forums were not searched. This
shows what these reads found, not that nothing closer exists.
