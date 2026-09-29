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

First person, standing on a sidewalk at night, every window on except those behind the site's
headline, which stay dark for contrast. The street's commercial buildings are brick, each laid in
its own as if from neighbouring quarries (red, brown, buff, clinker, orange, grey), in running bond
with paler mortar, falling dark above the lanterns, with roller shutters down over the closed shops;
behind the headline the mortar stays quiet. Everyone on the street is on one network. The dome tents
along the wall, as they stand on Portland sidewalks (two-tone rainflies, some under blue tarps, each
lit from inside), are on copper: each door's trace jogs 45° and joins a path that starts at your
feet and runs down the street, under a gate, to the door of the tallest tower. That copper is inlaid
flush in the concrete, in thin dark joints, and lit by what it mirrors: dim at your feet, bright far
off, with a glint of the lantern above it on every wire. The buildings talk by radio: an antenna on
every roof, faint links along the street, across it and up through the towers. Red paper lanterns
hang across the street on wires, lit from inside, the ones behind the headline unlit. The road is
wet asphalt: lights, windows, lanterns and signs reflect in it as streaks, stretched toward you,
blurred more up and down than across and broken by ripples. The sidewalks are opaque grey concrete,
cut into slabs, and reflect nothing. One set of lights shades everything except the network in the
sky: the LED streetlights across the road, the lanterns and signs, each tent's lamp and the windows.
Surfaces are solid and dimmed toward grey as night dims colour, the sky glows toward the horizon
behind solid towers, and nearer things cover farther ones. Its colours come from a Chinatown street
at night: red lanterns, red sign boards with pale-gold characters as on Yaowarat Road, a red lacquer
gate, magenta neon on the repair shop, teal in the window glass, cool white shop light and modern
LED streetlights; the poles, shutters and building trim stay grey. Copper and yellow belong to the
network alone: inlaid in the ground, and drawn flat over the sky as antennas, rooftop pads and
radio.

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
