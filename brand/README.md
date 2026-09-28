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

First person, standing on a sidewalk at night. Tents line the wall beside you; the nearest is lit,
with an antenna. Its trace leaves the door, jogs 45°, and joins a path that starts at your feet and
runs straight down the street, under a gate, to the door of the tallest tower. A puddle at your
feet reflects the towers.

- **Perspective breaks the 0°/45°/90° rule**, because it has to. Traces on the ground keep it in
  plan: the tent's trace runs 45° then 0° into a path at 90°.
- **Camera**, for redrawing it: eye 1.6 m above the sidewalk, 7.6 m left of the road's centre
  line, focal length 820 px, vanishing point (800, 540) in a 1600 × 1000 viewBox. Road ±5.5 m,
  walls at −11 m and +10 m, gate 82 m away, towers 560–780 m away.
- **Signs:** 茶楼 tea house, 饭店 restaurant, 药材 herbal medicine, 电器维修 appliance repair,
  腾振. They are SVG text, so they render in the viewer's own Chinese font. Only rendered so far
  with `rsvg-convert` on macOS.
- **Source:** a throwaway Python script drew it and was not kept. This SVG is the source.

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
