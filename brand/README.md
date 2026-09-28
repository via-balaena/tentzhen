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
