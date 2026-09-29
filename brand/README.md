# Brand

The mark is a peregrine falcon in a fast banking glide. 腾振 (téng zhèn) is drawn as PCB traces
at 0°, 45° and 90°, with through-hole pads at the free ends.

| file | use |
|---|---|
| `lockup-green.svg` | falcon + 腾振 on soldermask green |
| `lockup-black.svg` | falcon + 腾振 on black |
| `lockup-light.svg` | falcon + 腾振 on white |
| `mark-{green,black,light}.svg` | the falcon alone |
| `favicon.svg`, `favicon-32.png` | the falcon without barring, which turns to noise below ~64 px |
| `mural-street-night.svg` | the homepage mural: a Chinatown street at night, seen from the sidewalk (desktop, 16:10) |

## Colours

| role | hex |
|---|---|
| soldermask green (background) | `#0f3d24` |
| copper (text) | `#e0aa45` |
| peregrine hood | `#3d434c` (`#474e58` on black) |
| bare parts: eye-ring, cere, legs | `#ffc81a` |
| underwing greys, lit to dark | `#dfe2e5` `#c9cdd2` `#b2b7bd` `#7f858d` |
| breast | `#f6efe0` |
| barring | `#2c3137` |

## PCB details, on purpose

- The body's facets are copper islands separated by clearance gaps.
- The eye is a via: yellow ring, dark drill.
- The legs are yellow traces routed at 45°/90°, with a pad at the ankle; the toes end in talon
  pads.

## The mural

First person, standing on a Chinatown sidewalk at night, looking down the street to a skyline.

- **The street.** Brick commercial buildings, each laid in its own brick as if from neighbouring
  quarries (red, brown, buff, clinker, orange, grey), in running bond with paler mortar, falling
  dark above the lanterns. Every window is lit. A shop fills each 5 m bay (one building has a single
  wide glass front): a glass door and display window under a lettered sign band. Along the tents the
  shops are shut, dark glass with a roller shutter every third bay; everywhere else they are open,
  lit in lamplight, cool white or cream. Red paper lanterns hang across the street on wires, red
  sign boards with pale-gold characters hang from the walls as on Yaowarat Road, a red lacquer gate
  spans the street, and a blue parking sign stands at the kerb.
- **The tents.** Dome tents along the wall as they stand on Portland sidewalks (two-tone rainflies,
  some under blue tarps), each lit from inside so its nylon glows in its own colour; the
  streetlights catch their upper right and each sits in a contact shadow. The nearest, its door
  rolled wide open, is kept as a home: a rug, a made bed, books, a crate desk with a laptop and a
  lantern, fairy lights along the back wall.
- **The network.** Everyone on the street is on it. The tents are on copper, soldered in: each trace
  runs out from under the door, jogs 45° and tees into a path that runs from your feet, under the
  gate, to the door of the tallest tower. The copper is inlaid flush in the concrete in thin dark
  joints and lit by what it mirrors: dim at your feet, bright far off, with a glint of the lantern
  above it on every wire. The buildings talk by radio: an antenna on every roof along the street, a
  pad on each tower in front, faint links along the street, across it and up through the towers.
- **Light.** One set of lights shades everything but the network in the sky: the LED streetlights
  across the road, the lanterns and signs, each tent's lamp, the windows and shops. Surfaces are
  solid and dimmed toward grey as night dims colour, the sky glows toward the horizon, and nearer
  things cover farther ones. The road is wet asphalt: lights, windows, lanterns and signs reflect in
  it as streaks, stretched toward you, blurred more up and down than across and broken by ripples.
  The sidewalks are opaque grey concrete, cut into slabs, and reflect nothing.
- **The skyline.** Like Shenzhen's or Chongqing's: glass towers in layers, paler in the haze, each a
  solid with the side it turns toward you, offices lit in runs along its floors and cut into panes
  by fins, LED on a few crowns. The tallest, where your path ends, tapers to a stepped crown and
  spire; there is a ribbed tower with a rounded top, one with a cupped crown, and twin towers joined
  two thirds of the way up by a skybridge.
- **Behind the headline** windows and lanterns stay dark and the mortar quiet, so the words keep
  their contrast.
- **Colour.** Copper belongs to the network alone. Everything else takes its colour from a Chinatown
  street at night; the poles, shutters and building trim stay grey, and nothing inside a window or
  shop is red.

- **Perspective breaks the 0°/45°/90° rule**, because it has to. Traces on the ground keep it in
  plan: the tent's trace runs 45° then 0° into a path at 90°.
- **Signs** are SVG text, so they render in the viewer's own Chinese font. Only rendered so far
  with `rsvg-convert` on macOS.
- **Source:** `brand/mural.py` draws it — camera, street, towers and signs are all in there.
  Change the script, then run `python3 brand/mural.py` to rewrite the SVG. The Quality Gate runs
  it and fails if the committed SVG differs.

## Source

The falcon's body outline was simplified and mirrored from a PhyloPic silhouette of
*Falco peregrinus* by Andy Wilson
(`b961c530-3960-4b4c-89d8-33b48ffaeb9b`), released under
[CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/). CC0 asks for no credit; it is
given anyway. The head, colouring, facets, barring and legs are drawn for Tentzhen.

## Exporting a PNG

```fish
rsvg-convert -w 512 brand/mark-green.svg -o mark-green-512.png
```
