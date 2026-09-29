# Draws brand/mural-street-night.svg: a Chinatown street at night, seen from the sidewalk.
#
#   python3 brand/mural.py
#
# rewrites the SVG next to this file. The Quality Gate runs it and fails if the committed SVG
# differs, so change the drawing here, never in the SVG.
#
# World units are metres: x across the street (road centre 0), y up, z away from the viewer.
import math
import random
from pathlib import Path

random.seed(11)

W, H = 1600, 1000
CAM_X, EYE, F, VPX, VPY = -7.6, 1.6, 820.0, 800.0, 540.0
# Palette. Copper and yellow belong to the network alone: traces, vias, pads, antennas and radio.
# Everything else takes its colour from a Chinatown street at night: teal shadow, red and orange
# lanterns, magenta, red and orange neon, cool white street and shop light.
BG, DRILL = '#111111', '#111111'
COPPER, YELLOW = '#e0aa45', '#ffc81a'                                # the network
TEAL_FILL, TEAL_FILL_2, TEAL_DEEP = '#0f1a1e', '#12242a', '#162a2f'   # walls, fabric, the street
TEAL_LINE, TEAL_EDGE = '#24424a', '#3f7a80'                          # outlines, quiet then bright
COOL = '#9fe0d8'                                                     # street and shop light
NEON_RED, NEON_MAGENTA, NEON_ORANGE = '#ff4a3a', '#ff3fa4', '#ff8a3a'
LAMPLIGHT = '#ffb35a'                                                # light from inside a tent
HOMELIGHT = '#f0dfb8'                                                # a lamp in a home window
GREY_D, GREY_DD, FACADE, SHUTTER = '#3a3e44', '#23262a', '#141517', '#2a2d31'  # street buildings, poles
METAL, METAL_FAR, ALUMINIUM = '#7a8088', '#555b63', '#8a9098'         # towers, window and shop frames
# One night, one set of lights, for everything but the network: LED streetlights along the far kerb
# (cool, from above and across the street), the lanterns and signs overhead (warm red), each tent's
# lamp (amber, from inside), and the windows. Every surface is a solid, shaded by those lights and
# dimmed toward grey the way night dims colour; only a light source is bright. The network is the
# exception on purpose: flat copper laid over the scene.
LEFT_W, CURB_L, CURB_R, RIGHT_W = -11.0, -6.5, 5.5, 10.0  # a 4.5 m sidewalk under you, the road, a 4.5 m one

def P(x, y, z): return (VPX + F * (x - CAM_X) / z, VPY - F * (y - EYE) / z)
def sw(z, k=0.10, lo=0.8, hi=4.0): return max(lo, min(hi, F * k / z))
def pts(ps): return ' '.join(f'{x:.1f},{y:.1f}' for x, y in ps)

defs, out = [], []
def poly(ps, fill='none', stroke='none', w=1.0, extra=''):
    out.append(f'<polygon points="{pts(ps)}" fill="{fill}" stroke="{stroke}" stroke-width="{w:.2f}" stroke-linejoin="round" {extra}/>')
def line(a, b, stroke, w, extra=''):
    out.append(f'<line x1="{a[0]:.1f}" y1="{a[1]:.1f}" x2="{b[0]:.1f}" y2="{b[1]:.1f}" stroke="{stroke}" stroke-width="{w:.2f}" stroke-linecap="round" {extra}/>')
def pline(ps, stroke, w, extra=''):
    out.append(f'<polyline points="{pts(ps)}" fill="none" stroke="{stroke}" stroke-width="{w:.2f}" stroke-linecap="round" stroke-linejoin="round" {extra}/>')
def circle(c, r, fill, extra=''):
    out.append(f'<circle cx="{c[0]:.1f}" cy="{c[1]:.1f}" r="{r:.2f}" fill="{fill}" {extra}/>')
def ellipse(c, rx, ry, fill, extra=''):
    out.append(f'<ellipse cx="{c[0]:.1f}" cy="{c[1]:.1f}" rx="{rx:.2f}" ry="{ry:.2f}" fill="{fill}" {extra}/>')
def text(c, s, size, fill, extra=''):
    out.append(f'<text x="{c[0]:.1f}" y="{c[1]:.1f}" font-size="{size:.1f}" fill="{fill}" text-anchor="middle" dominant-baseline="central" font-family="Noto Sans SC, PingFang SC, sans-serif" font-weight="500" {extra}>{s}</text>')
def upside_down(c): return f'transform="matrix(1 0 0 -1 0 {2 * c[1]:.1f})"'
def pad(c, r, color): circle(c, r, color); circle(c, r * 0.42, DRILL)
def ground_pad(x, z, r, color, drill=True):
    c = P(x, 0, z); rx = F * r / z; ry = F * EYE * r / (z * z)
    ellipse(c, rx, ry, color)
    if drill: ellipse(c, rx * 0.42, ry * 0.42, DRILL)

out.append(f'<rect width="{W}" height="{H}" fill="{BG}"/>')
# A city's sky at night is not black: it glows toward the horizon with the street's own light.
SKY_GLOW = '#1f1a1b'
defs.append(f'<linearGradient id="sky" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="0" y2="{VPY}">'
            f'<stop offset="0" stop-color="{BG}"/><stop offset="1" stop-color="{SKY_GLOW}"/></linearGradient>')
out.append(f'<rect width="{W}" height="{VPY}" fill="url(#sky)"/>')

RADIO = "#c39b52"  # radio stays quiet in the sky; copper carries the ground
# Buildings talk by radio: dotted arcs between rooftop antennas, and a few ripples at each mast.
def radio_link(a, b, c, w=1.5):
    out.append(f'<path d="M{a[0]:.1f} {a[1]:.1f} Q{c[0]:.1f} {c[1]:.1f} {b[0]:.1f} {b[1]:.1f}" '
               f'fill="none" stroke="{RADIO}" stroke-width="{w:.2f}" stroke-linecap="round" '
               f'stroke-dasharray="0.1 {3.4 * w:.1f}" opacity="0.5"/>')
def ripples(c, r):
    for k in (1.8, 2.8):
        rr = r * k
        x0, y0 = c[0] - rr * 0.7, c[1] - rr * 0.7
        x1, y1 = c[0] + rr * 0.7, c[1] - rr * 0.7
        out.append(f'<path d="M{x0:.1f} {y0:.1f} A{rr:.1f} {rr:.1f} 0 0 1 {x1:.1f} {y1:.1f}" fill="none" '
                   f'stroke="{RADIO}" stroke-width="1.1" stroke-linecap="round" opacity="0.45"/>')

# Wet asphalt is a rough mirror. Each light shows below itself, mirrored in the ground plane,
# stretched toward you, strongest near its base and fading as it comes closer, softened more up
# and down than across, and broken by ripple bands that widen with nearness. Every reflection is
# collected here and drawn as one layer on the ground: after the road markings, before anything
# that stands on the street.
REFL, GRADS = [], {}
def reflect(shape, color, alpha, stretch=1.5):
    key = (color, alpha)
    if key not in GRADS:
        gid = f'wet{len(GRADS)}'
        GRADS[key] = gid
        defs.append(f'<linearGradient id="{gid}" x1="0" y1="0" x2="0" y2="1">'
                    f'<stop offset="0" stop-color="{color}" stop-opacity="{alpha}"/>'
                    f'<stop offset="1" stop-color="{color}" stop-opacity="0"/></linearGradient>')
    mirrored = []
    for x, y, z in shape:
        base_y = P(x, 0, z)[1]
        mx, my = P(x, -y, z)
        mirrored.append((mx, base_y + (my - base_y) * stretch))
    REFL.append(f'<polygon points="{pts(mirrored)}" fill="url(#{GRADS[key]})"/>')
def streak(x, y, z, width, color, alpha):
    # a small light: a column from just under its base to past its mirror image
    hw = width / 2
    reflect([(x - hw, 0.1 * y, z), (x - hw, 1.3 * y, z), (x + hw, 1.3 * y, z), (x + hw, 0.1 * y, z)],
            color, alpha, stretch=1.2)

# ---------------- Downtown towers ----------------
towers = [
    (-120, 780, 52, 230, 'flat'), (-88, 660, 40, 250, 'chamfer'), (-58, 600, 34, 300, 'pyramid'),
    (-30, 740, 46, 330, 'chamfer'), (-40, 560, 30, 240, 'flat'), (-7.6, 640, 38, 380, 'spire'),
    (28, 580, 34, 290, 'chamfer'), (52, 700, 50, 340, 'flat'), (80, 620, 36, 260, 'chamfer'),
    (110, 760, 54, 300, 'flat'), (142, 680, 40, 220, 'flat'),
]
towers.sort(key=lambda t: -t[1])
def draw_towers(Q):
    rnd = random.Random(3)
    for x, z, w, h, top in towers:
        near = z < 650
        col = METAL if near else METAL_FAR
        x0, x1 = x - w / 2, x + w / 2
        bl, br = Q(x0, 0, z), Q(x1, 0, z)
        if top == 'pyramid':
            shape = [bl, Q(x0 + 1, h * 0.74, z), Q(x, h, z), Q(x1 - 1, h * 0.74, z), br]
        elif top == 'chamfer':
            c = w * 0.22
            shape = [bl, Q(x0, h - c, z), Q(x0 + c, h, z), Q(x1 - c, h, z), Q(x1, h - c, z), br]
        else:
            shape = [bl, Q(x0, h, z), Q(x1, h, z), br]
        poly(shape, fill='#131417' if near else '#16161a', stroke=col, w=1.6 if near else 1.2)
        if top == 'spire':
            a, b = Q(x, h, z), Q(x, h + 40, z)
            line(a, b, col, 1.6); pad(b, 3.6, col)
        top_y = h * (0.72 if top == 'pyramid' else 0.93)
        cols = max(2, int(w / 6.5))
        for i in range(cols):
            xx = x0 + (i + 0.5) * w / cols
            yy = 7.0
            while yy < top_y - 3:
                bright = rnd.random() < 0.25
                circle(Q(xx, yy, z), 1.3, HOMELIGHT if bright else '#5aa8a2')
                yy += 8.0
    # Every rooftop linked to its neighbours by an arc through the sky.
    by_x = sorted(towers, key=lambda t: t[0])
    for (xa, za, _, ha, _), (xb, zb, _, hb, _) in zip(by_x, by_x[1:]):
        a, b = Q(xa, ha, za), Q(xb, hb, zb)
        c = Q((xa + xb) / 2, max(ha, hb) + 45, (za + zb) / 2)
        radio_link(a, b, c)
    for x, z, _, h, _ in towers:
        pad(Q(x, h, z), 2.4, COPPER)

draw_towers(P)

# ---------------- Street facades ----------------
# The site sets its headline over the top left. At the narrowest desktop width (1280 px) it covers
# about this box of the mural; windows touching it stay dark so the words keep their contrast.
HEADLINE = (60, 20, 780, 480)
def behind_headline(quad):
    xs, ys = [p[0] for p in quad], [p[1] for p in quad]
    return max(xs) > HEADLINE[0] and min(xs) < HEADLINE[2] and max(ys) > HEADLINE[1] and min(ys) < HEADLINE[3]
def facade(xw, z0, z1, h, lit_shop=False):
    q = [P(xw, 0, z0), P(xw, h, z0), P(xw, h, z1), P(xw, 0, z1)]
    poly(q, fill=FACADE, stroke=GREY_D, w=sw(z0, 0.07, 0.8, 2.2))
    line(P(xw, h - 0.8, z0), P(xw, h - 0.8, z1), GREY_D, sw(z0, 0.05, 0.6, 1.6))
    line(P(xw, 3.8, z0), P(xw, 3.8, z1), GREY_D, sw(z0, 0.05, 0.6, 1.6))
    fl = 4.6
    while fl + 2.0 < h - 1.2:
        zz = z0 + 1.2
        while zz + 1.4 < z1 - 0.6:
            wq = [P(xw, fl, zz), P(xw, fl + 2.0, zz), P(xw, fl + 2.0, zz + 1.4), P(xw, fl, zz + 1.4)]
            # every window is on, lamplight or screen-teal, except behind the headline
            # aluminium frames; the light is in the glass
            fill, glow, alpha = [('#5a5040', HOMELIGHT, 0.12),
                                 ('#1a4a52', COOL, 0.1),
                                 ('#36322a', HOMELIGHT, 0.05)][min(2, int(random.random() * 2.6))]
            if behind_headline(wq):
                poly(wq, fill='#16171a', stroke=GREY_DD, w=sw(zz, 0.04, 0.5, 1.4))
            else:
                poly(wq, fill=fill, stroke=ALUMINIUM, w=sw(zz, 0.04, 0.5, 1.4))
                reflect([(xw, fl, zz), (xw, fl + 2.0, zz), (xw, fl + 2.0, zz + 1.4), (xw, fl, zz + 1.4)],
                        glow, alpha, stretch=1.25)
            zz += 3.0
        fl += 3.4
    if lit_shop:
        a, b = z0 + 1.0, z1 - 1.0
        poly([P(xw, 0.15, a), P(xw, 3.3, a), P(xw, 3.3, b), P(xw, 0.15, b)], fill='#1d3c3e', stroke=ALUMINIUM, w=sw(z0, 0.07, 0.8, 2.0))
        reflect([(xw, 0.15, a), (xw, 3.3, a), (xw, 3.3, b), (xw, 0.15, b)], COOL, 0.35, stretch=1.6)
        for k in range(1, 4):
            zz = a + (b - a) * k / 4
            line(P(xw, 0.15, zz), P(xw, 3.3, zz), ALUMINIUM, sw(zz, 0.03, 0.6, 1.2))
        line(P(xw, 1.1, a), P(xw, 1.1, b), ALUMINIUM, sw(z0, 0.03, 0.6, 1.2))
    else:
        for yy in (0.9, 1.5, 2.1, 2.7, 3.3):
            line(P(xw, yy, z0 + 0.8), P(xw, yy, z1 - 0.8), SHUTTER, sw(z0, 0.03, 0.5, 1.2))

right = [(140, 240, 16), (100, 140, 13), (74, 100, 15), (52, 74, 12), (32, 52, 17), (12, 32, 14)]
left = [(135, 240, 14), (95, 135, 15), (66, 95, 12), (44, 66, 18), (24, 44, 13), (1.5, 24, 16)]
for z0, z1, h in right: facade(RIGHT_W, z0, z1, h, lit_shop=(z0 == 32))
for z0, z1, h in left: facade(LEFT_W, z0, z1, h)

# Every building is on the network by radio: an antenna on its roof, linked to its neighbours
# along the street, across it, and up to the towers.
def antenna(xw, z0, z1, h):
    x, z = xw - 0.5 if xw > 0 else xw + 0.5, (z0 + z1) / 2
    base, tip = P(x, h, z), P(x, h + 2.4, z)
    line(base, tip, COPPER, sw(z, 0.08, 0.9, 2.4))
    r = max(1.8, F * 0.22 / z)
    pad(tip, r, COPPER)
    ripples(tip, r)
    return tip, (x, h + 2.4, z)
masts = {'right': [antenna(RIGHT_W, *b) for b in right], 'left': [antenna(LEFT_W, *b) for b in left]}
def arc(a, b, lift=6.0):
    (pa, wa), (pb, wb) = a, b
    c = P((wa[0] + wb[0]) / 2, max(wa[1], wb[1]) + lift, (wa[2] + wb[2]) / 2)
    radio_link(pa, pb, c)
for side in masts.values():  # along the street
    for a, b in zip(side, side[1:]):
        arc(a, b)
for a, b in zip(masts['left'], masts['right']):  # across it
    arc(a, b, lift=4.0)
for m in (masts['left'][0], masts['right'][0]):  # the far end of the street up to the towers
    x, z, _, h, _ = min(towers, key=lambda t: abs(t[0] - m[1][0]) + t[1] / 100)
    tip = P(x, h, z)
    arc(m, (tip, (x, h, z)), lift=40.0)

# ---------------- Ground ----------------
# Grey concrete sidewalks, opaque and lighter than the road, cut into slabs by dark joints; the road
# between them is dark wet asphalt, with no line painted along it, and only the road mirrors the
# lights. The change of surface marks the kerb.
ZF = 240
SIDEWALK, JOINT, KERB, KERB_FACE = '#242528', '#17181a', '#3a3c41', '#2a2c30'
for x0, x1 in ((LEFT_W, CURB_L), (CURB_R, RIGHT_W)):
    poly([P(x0, 0, 2.4), P(x0, 0, ZF), P(x1, 0, ZF), P(x1, 0, 2.4)], fill=SIDEWALK)
    # square slabs, 1.5 m, jointed across and along
    z = 2.4
    while z < 90:
        line(P(x0, 0, z), P(x1, 0, z), JOINT, sw(z, 0.025, 0.4, 1.2)); z += 1.5
    inner = CURB_L - 0.3 if x1 == CURB_L else CURB_R + 0.3
    xs = [x for x in (-9.5, -8.0, 7.1, 8.6) if min(x0, x1) < x < max(x0, x1)]
    for x in xs:
        line(P(x, 0, 2.4), P(x, 0, 90), JOINT, 1.0)
    # the kerbstone along the road edge
    poly([P(inner, 0, 2.4), P(inner, 0, ZF), P(CURB_L if x1 == CURB_L else CURB_R, 0, ZF),
          P(CURB_L if x1 == CURB_L else CURB_R, 0, 2.4)], fill=KERB)
# the far kerb's face, dropping 15 cm to the road, is turned toward you
poly([P(CURB_R, 0, 3.0), P(CURB_R, 0, ZF), P(CURB_R, -0.15, ZF), P(CURB_R, -0.15, 3.0)], fill=KERB_FACE)
for xc in (LEFT_W, RIGHT_W):  # where the buildings meet the sidewalk
    line(P(xc, 0, 2.5), P(xc, 0, ZF), GREY_D, 1.4)
for i in range(12):  # a crosswalk, kerb to kerb
    xs = CURB_L + 0.25 + i * 1.0
    poly([P(xs, 0, 46), P(xs + 0.5, 0, 46), P(xs + 0.5, 0, 51), P(xs, 0, 51)], fill='#26282c')
ground_pad(2.6, 7.5, 0.55, '#26282c')   # a manhole cover, pad-shaped

REFL_AT = len(out)  # the reflection layer goes here, on the ground

# Everything that stands in or hangs over the street is drawn by a function of the projection Q, so
# the puddle further down can draw it again, turned over.

# ---------------- A parking sign at the kerb ----------------
# Portland's magenta "P" on a pole, standing between the tents and the road.
PZ_SIGN, PX_SIGN = 8.0, CURB_L - 0.4
def parking_sign(Q=P):
    line(Q(PX_SIGN, 0, PZ_SIGN), Q(PX_SIGN, 2.9, PZ_SIGN), GREY_D, sw(PZ_SIGN, 0.06, 0.8, 2.4))
    sq = [(PX_SIGN - 0.24, 2.35, PZ_SIGN), (PX_SIGN - 0.24, 2.85, PZ_SIGN), (PX_SIGN + 0.24, 2.85, PZ_SIGN), (PX_SIGN + 0.24, 2.35, PZ_SIGN)]
    poly([Q(*q) for q in sq], fill='#b0306a', stroke='#d8d8dc', w=sw(PZ_SIGN, 0.02, 0.5, 1.0))
    c = Q(PX_SIGN, 2.6, PZ_SIGN)
    text(c, 'P', F * 0.36 / PZ_SIGN, '#f2f2f4', extra=upside_down(c) if Q is not P else '')

# ---------------- Gate ----------------
GZ = 82
gc, lacquer, roof = '#c23a2a', '#4a1611', '#241816'  # red lacquer lit at its edges, in shadow between
def gate(Q=P):
    for xp in (-9.2, 8.6):
        poly([Q(xp - 0.4, 0, GZ), Q(xp - 0.4, 8.2, GZ), Q(xp + 0.4, 8.2, GZ), Q(xp + 0.4, 0, GZ)], fill=lacquer, stroke=gc, w=1.6)
    poly([Q(-10.0, 6.4, GZ), Q(-10.0, 7.0, GZ), Q(9.4, 7.0, GZ), Q(9.4, 6.4, GZ)], fill=lacquer, stroke=gc, w=1.4)
    poly([Q(-10.4, 8.2, GZ), Q(-10.4, 8.8, GZ), Q(9.8, 8.8, GZ), Q(9.8, 8.2, GZ)], fill=lacquer, stroke=gc, w=1.4)
    poly([Q(-12.0, 9.4, GZ), Q(-11.2, 8.8, GZ), Q(10.6, 8.8, GZ), Q(11.4, 9.4, GZ), Q(8.6, 10.8, GZ), Q(-9.2, 10.8, GZ)], fill=roof, stroke=gc, w=1.6)
    pad(Q(-12.0, 9.4, GZ), 3.0, gc); pad(Q(11.4, 9.4, GZ), 3.0, gc); pad(Q(-0.3, 10.8, GZ), 3.0, gc)
    poly([Q(-1.8, 7.2, GZ), Q(-1.8, 8.0, GZ), Q(1.2, 8.0, GZ), Q(1.2, 7.2, GZ)], fill=lacquer, stroke=gc, w=1.2)

# ---------------- Streetlights (right curb) ----------------
# Modern LED streetlights: a grey pole and an arm over the road, a slim head, cool light from an
# LED strip on its underside, a faint cone to the ground and a pool of light where it lands.
defs.append(f'<linearGradient id="cone" x1="0" y1="0" x2="0" y2="1">'
            f'<stop offset="0" stop-color="{COOL}" stop-opacity="0.13"/>'
            f'<stop offset="1" stop-color="{COOL}" stop-opacity="0"/></linearGradient>')
LAMPS = (22, 42, 64, 96, 150)
def streetlight(zl, Q=P):
    seen = Q is P  # the cone and the pool are light in the air and on the ground; a mirror has neither
    line(Q(5.9, 0, zl), Q(5.9, 6.8, zl), GREY_D, sw(zl, 0.14, 0.8, 3))
    line(Q(5.9, 6.8, zl), Q(4.95, 6.95, zl), GREY_D, sw(zl, 0.1, 0.8, 2.5))
    if seen:
        poly([P(4.15, 6.86, zl), P(4.95, 6.86, zl), P(5.7, 0, zl), P(3.4, 0, zl)], fill='url(#cone)')
        ellipse(P(4.55, 0, zl), F * 1.3 / zl, F * EYE * 1.3 / zl ** 2, COOL, extra='opacity="0.09"')
    poly([Q(4.1, 7.02, zl), Q(4.98, 7.02, zl), Q(4.98, 6.88, zl), Q(4.1, 6.88, zl)], fill=SHUTTER, stroke=GREY_D,
         w=sw(zl, 0.03, 0.6, 1.4))
    line(Q(4.18, 6.87, zl), Q(4.9, 6.87, zl), COOL, max(1.0, F * 0.05 / zl))
    if seen:
        streak(4.55, 6.87, zl, 0.7, COOL, 0.35)

# ---------------- Blade signs ----------------
# 茶楼 tea house, 电器维修 appliance repair, 饭店 restaurant, 腾振, 药材 herbal medicine.
# Most are backlit red boards with pale-gold characters and trim, as on Yaowarat Road; the
# electronics repair shop is neon.
BOARD_RED, BOARD_GOLD = '#b8261c', '#f5dca0'
def sign(xs0, xs1, y0, y1, z, chars, color=None, refl=0.18, Q=P):
    q = [(xs0, y0, z), (xs0, y1, z), (xs1, y1, z), (xs1, y0, z)]
    mirrored = Q is not P
    if not mirrored:
        reflect(q, color or BOARD_RED, refl, stretch=1.4)
    if color is None:  # a board
        poly([Q(*p) for p in q], fill=BOARD_RED, stroke=BOARD_GOLD, w=sw(z, 0.08, 0.8, 2.2))
        ink = BOARD_GOLD
    else:              # neon
        poly([Q(*p) for p in q], fill='#160e14', stroke=color, w=sw(z, 0.08, 0.8, 2.2))
        ink = color
    n = len(chars); step = (y1 - y0) / n
    size = F * min(xs1 - xs0, step) * 0.64 / z
    for i, ch in enumerate(chars):
        c = Q((xs0 + xs1) / 2, y1 - step * (i + 0.5), z)
        text(c, ch, size, ink, extra=upside_down(c) if mirrored else '')
SIGNS = [(-10.9, -9.5, 4.6, 9.8, 26, '茶楼'), (8.6, 9.9, 4.2, 10.2, 36, '电器维修', NEON_MAGENTA, 0.35),
         (-10.9, -9.8, 4.6, 9.0, 54, '饭店'), (8.7, 9.9, 4.2, 8.4, 60, '腾振', None, 0.25),
         (8.8, 9.9, 4.4, 8.2, 92, '药材')]

# ---------------- Lanterns ----------------
# Red paper lanterns on wires across the street, hung as in Chinatown: round, lit from inside,
# ribbed, capped top and bottom, a soft bloom around each, and a red streak below on the wet
# street. The ones behind the headline hang unlit, for contrast.
LANTERN_RED, LANTERN_GLOW, LANTERN_DARK = '#e0452b', '#ff9a3c', '#3a1a12'
defs.append('<radialGradient id="lantern" cx="0.5" cy="0.45" r="0.6">'
            '<stop offset="0" stop-color="#ffd27a"/><stop offset="0.35" stop-color="#ff8a3a"/>'
            '<stop offset="0.8" stop-color="#e0452b"/><stop offset="1" stop-color="#9c2a1a"/></radialGradient>')
defs.append('<radialGradient id="bloom"><stop offset="0" stop-color="#ff7a3a" stop-opacity="0.35"/>'
            '<stop offset="1" stop-color="#ff7a3a" stop-opacity="0"/></radialGradient>')
def lantern(x, y, z, Q=P):
    c, seen = Q(x, y, z), P(x, y, z)
    rx, ry = F * 0.26 / z, F * 0.21 / z
    cap_w, cap_h = rx * 0.55, max(0.8, ry * 0.18)
    if HEADLINE[0] < seen[0] < HEADLINE[2] and HEADLINE[1] < seen[1] < HEADLINE[3]:
        ellipse(c, rx, ry, LANTERN_DARK, extra=f'stroke="#5a2a1c" stroke-width="{max(0.6, rx * 0.08):.2f}"')
        return
    circle(c, rx * 2.6, 'url(#bloom)')
    ellipse(c, rx, ry, 'url(#lantern)')
    if rx > 6:  # near enough to see the paper's ribs
        for k in (0.62, 0.22):
            out.append(f'<ellipse cx="{c[0]:.1f}" cy="{c[1]:.1f}" rx="{rx * k:.2f}" ry="{ry:.2f}" fill="none" '
                       f'stroke="#b8341f" stroke-width="{max(0.5, rx * 0.05):.2f}" opacity="0.7"/>')
    for dy in (-ry - cap_h / 2, ry + cap_h / 2):
        out.append(f'<rect x="{c[0] - cap_w / 2:.1f}" y="{c[1] + dy - cap_h / 2:.1f}" width="{cap_w:.2f}" '
                   f'height="{cap_h:.2f}" fill="#8c3a1f"/>')
    if Q is P:
        streak(x, y, z, 0.4, LANTERN_RED, 0.2)
WIRES = (16, 21, 27, 34, 43, 55, 70, 90, 118)
def lantern_wire(zl, Q=P, box=None):  # box: draw only what lands inside it (x0, y0, x1, y1)
    def inside(c, r):
        return box is None or (box[0] - r < c[0] < box[2] + r and box[1] - r < c[1] < box[3] + r)
    xsamp = [LEFT_W + i * (RIGHT_W - LEFT_W) / 30 for i in range(31)]
    mid, half = (LEFT_W + RIGHT_W) / 2, (RIGHT_W - LEFT_W) / 2
    ys = [8.4 - 1.0 * (1 - ((xx - mid) / half) ** 2) for xx in xsamp]
    wire = [Q(xx, yy, zl) for xx, yy in zip(xsamp, ys)]
    if not any(inside(c, 0) for c in wire):
        return
    pline(wire, '#3a2a24', sw(zl, 0.025, 0.5, 1.0))
    for i in range(1, 30, 2):
        xx, yy = xsamp[i], ys[i]
        if not inside(Q(xx, yy - 0.46, zl), F * 0.7 / zl):
            continue
        line(Q(xx, yy, zl), Q(xx, yy - 0.25, zl), '#5a3a1c', sw(zl, 0.02, 0.4, 0.9))
        lantern(xx, yy - 0.25 - 0.21, zl, Q)

# ---------------- The street, far to near ----------------
# Gate, streetlights, signs, lantern wires and the parking sign are drawn from the far end toward
# you, so whatever is nearer covers what is behind it.
def street(Q=P, box=None):
    items = [(GZ, lambda: gate(Q)), (PZ_SIGN, lambda: parking_sign(Q))]
    items += [(zl, lambda zl=zl: streetlight(zl, Q)) for zl in LAMPS]
    items += [(sg[4], lambda sg=sg: sign(*sg, Q=Q)) for sg in SIGNS]
    items += [(zl, lambda zl=zl: lantern_wire(zl, Q, box)) for zl in WIRES]
    for _, draw in sorted(items, key=lambda it: -it[0]):
        draw()
street()

# ---------------- A puddle at your feet ----------------
# Still water is a clean mirror. It holds whatever stands above and beyond it, turned over: the
# towers, the wall beside you and the street's signs and lanterns, dimmed, with the dark of the sky
# between them. The concrete around it is darker where it is wet, and it has no drawn edge.
PX, PZ, PRX, PRZ = -7.6, 3.75, 0.78, 0.85
def rim(k):
    return [P(PX + k * PRX * math.cos(t) * (1 + 0.12 * math.sin(3 * t)), 0, PZ + k * PRZ * math.sin(t))
            for t in [i * math.pi / 24 for i in range(48)]]
puddle = rim(1.0)
defs.append('<filter id="damp" x="-20%" y="-20%" width="140%" height="140%"><feGaussianBlur stdDeviation="4"/></filter>')
poly(rim(1.2), fill='#1b1c1f', extra='filter="url(#damp)"')
defs.append(f'<clipPath id="puddle"><polygon points="{pts(puddle)}"/></clipPath>')
poly(puddle, fill='#0f1012')
MIRROR = lambda x, y, z: P(x, -y, z)
out.append('<g clip-path="url(#puddle)" opacity="0.6">')
draw_towers(MIRROR)
for z0, z1, h in left:
    poly([MIRROR(LEFT_W, 0, z0), MIRROR(LEFT_W, h, z0), MIRROR(LEFT_W, h, z1), MIRROR(LEFT_W, 0, z1)],
         fill=FACADE, stroke=GREY_D, w=sw(z0, 0.07, 0.8, 2.2))
street(MIRROR, box=(min(p[0] for p in puddle), min(p[1] for p in puddle),
                    max(p[0] for p in puddle), max(p[1] for p in puddle)))
out.append('</g>')

# ---------------- The network on the ground ----------------
# Every tent is on copper: out of the door, a 45-degree jog, and across to your path, which runs
# from your feet to the tallest tower's door. The buildings above talk by radio.
TENT_LIT = 5.6
# Dome tents as they stand on Portland sidewalks: two-tone rainflies, some under a blue tarp.
# (front z, width, height, antenna, body colour, fly colour, tarp). Colours are dimmed for night.
NAVY, GREEN, RED_FLY, GREY_FLY, TARP = '#1c2438', '#3d4d2b', '#662824', '#4c5157', '#27365e'
TENTS = [(TENT_LIT, 2.3, 1.3, True, NAVY, NAVY, True), (10.4, 2.1, 1.15, False, GREEN, GREY_FLY, False),
         (13.4, 2.3, 1.25, True, RED_FLY, GREY_FLY, False), (19.6, 1.9, 1.05, False, NAVY, GREY_FLY, False),
         (22.2, 2.2, 1.2, True, NAVY, NAVY, True), (30.5, 2.0, 1.1, False, GREEN, GREY_FLY, False),
         (38.0, 2.3, 1.25, True, RED_FLY, GREY_FLY, False)]
MAIN = [(CAM_X, 2.4), (CAM_X, 640.0)]
def ribbon(a, b, wd):
    (x0, z0), (x1, z1) = a, b
    dx, dz = x1 - x0, z1 - z0; L = math.hypot(dx, dz); nx, nz = -dz / L * wd, dx / L * wd
    return [P(x0 + nx, 0, z0 + nz), P(x1 + nx, 0, z1 + nz), P(x1 - nx, 0, z1 - nz), P(x0 - nx, 0, z0 - nz)]
def trace(points, vias=()):
    for a, b in zip(points, points[1:]):
        poly(ribbon(a, b, 0.06), fill=COPPER, stroke=COPPER, w=0.6)
    for x, z in vias:
        ground_pad(x, z, 0.2, COPPER)

poly(ribbon(MAIN[0], MAIN[1], 0.24), fill=COPPER, extra='opacity="0.12"')
poly(ribbon(MAIN[0], MAIN[1], 0.075), fill=COPPER, stroke=COPPER, w=0.8)
for z0, wd, *_ in TENTS:  # every tent: out of the door, a 45-degree jog, across to your path
    xm = -10.9 + wd / 2
    trace([(xm, z0), (xm + 0.6, z0 - 0.6), (CAM_X, z0 - 0.6)], vias=[(CAM_X, z0 - 0.6)])
pad(P(CAM_X, 0.0, 640.0), 3.6, COPPER)

# ---------------- Tents along the left wall ----------------
# Light from a lamp inside: brightest low in the opening and falling off to amber at its edges; the
# translucent fabric warming around the door; the tied-back flap catching it; a pool on the wet
# pavement that fades with distance.
defs.append('<radialGradient id="tentlight" cx="0.5" cy="0.82" r="0.75">'
            '<stop offset="0" stop-color="#fff1c8"/><stop offset="0.22" stop-color="#ffc070"/>'
            '<stop offset="0.6" stop-color="#e0782a"/><stop offset="1" stop-color="#5a2610"/></radialGradient>')
defs.append('<radialGradient id="fabricglow" cx="0.5" cy="0.9" r="0.75">'
            '<stop offset="0" stop-color="#ff9a4a" stop-opacity="0.42"/>'
            '<stop offset="1" stop-color="#ff9a4a" stop-opacity="0"/></radialGradient>')
defs.append('<radialGradient id="sideglow" cx="0.12" cy="0.85" r="0.8">'
            '<stop offset="0" stop-color="#ff9a4a" stop-opacity="0.22"/>'
            '<stop offset="1" stop-color="#ff9a4a" stop-opacity="0"/></radialGradient>')
defs.append('<radialGradient id="pool"><stop offset="0" stop-color="#ffb35a" stop-opacity="0.3"/>'
            '<stop offset="0.5" stop-color="#ff9a4a" stop-opacity="0.12"/>'
            '<stop offset="1" stop-color="#ff9a4a" stop-opacity="0"/></radialGradient>')
defs.append('<linearGradient id="flaplit" x1="1" y1="0.6" x2="0" y2="0.9">'
            '<stop offset="0" stop-color="#c8743a"/><stop offset="1" stop-color="#4a2a18"/></linearGradient>')
def hull(points):
    pts_ = sorted(set((round(x, 2), round(y, 2)) for x, y in points))
    def half(seq):
        h = []
        for p in seq:
            while len(h) >= 2 and (h[-1][0] - h[-2][0]) * (p[1] - h[-2][1]) - (h[-1][1] - h[-2][1]) * (p[0] - h[-2][0]) <= 0:
                h.pop()
            h.append(p)
        return h
    lower, upper = half(pts_), half(reversed(pts_))
    return lower[:-1] + upper[:-1]
# the streetlights across the road light each dome from the upper right; the side against the wall
# and the ground falls into shadow
defs.append('<linearGradient id="flyshade" x1="1" y1="0" x2="0.2" y2="1">'
            f'<stop offset="0" stop-color="{COOL}" stop-opacity="0.12"/>'
            '<stop offset="0.45" stop-color="#000000" stop-opacity="0"/>'
            '<stop offset="1" stop-color="#000000" stop-opacity="0.45"/></linearGradient>')
def tent(z0, wd, ht, antenna, body, fly, tarp):
    # a half-ellipsoid dome: base ellipse wd x ln, height ht; its front touches z0
    xm, ln = -10.9 + wd / 2, 2.2
    zc = z0 + ln / 2
    def surf(u, v):  # u around the base, v from the ground (0) to the top (pi/2)
        return (xm + wd / 2 * math.cos(v) * math.cos(u), ht * math.sin(v), zc + ln / 2 * math.cos(v) * math.sin(u))
    def facing(pt):  # is this surface point turned toward the viewer?
        x, y, z = pt
        n = ((x - xm) / (wd / 2) ** 2, y / ht ** 2, (z - zc) / (ln / 2) ** 2)
        v = (CAM_X - x, EYE - y, -z)
        return n[0] * v[0] + n[1] * v[1] + n[2] * v[2] > 0
    def visible_runs(world):  # the parts of a curve on the side you can see
        run = []
        for pt in world:
            if facing(pt):
                run.append(P(*pt))
            elif len(run) > 1:
                yield run
                run = []
            else:
                run = []
        if len(run) > 1:
            yield run
    def on_front(x, y):  # the front surface point at this x and height
        r = 1 - ((x - xm) / (wd / 2)) ** 2 - (y / ht) ** 2
        return (x, y, zc - ln / 2 * math.sqrt(max(0.0, r)))
    w = sw(z0, 0.04, 0.8, 2.6)
    ellipse(P(xm, 0, z0 - 0.7), F * 1.25 / z0, F * EYE * 1.25 / (z0 - 0.7) ** 2, 'url(#pool)')
    grid = [P(*surf(math.radians(a), math.radians(b))) for a in range(0, 360, 10) for b in range(0, 91, 10)]
    outline = hull(grid)
    poly(outline, fill=body, stroke='#0c0e12', w=w)
    # the rainfly: the upper cap, in its own colour
    cap = hull([P(*surf(math.radians(a), math.radians(b))) for a in range(0, 360, 10) for b in range(38, 91, 8)])
    poly(cap, fill=fly, stroke='#0c0e12', w=w * 0.8)
    poly(outline, fill='url(#flyshade)')
    # two poles crossing over the top, where they face you
    for a in (45, 135):
        arc_ = [surf(math.radians(a + (180 if b < 0 else 0)), math.radians(abs(b))) for b in range(-90, 91, 4)]
        for run in visible_runs(arc_):
            pline(run, '#0c0e12', max(0.6, w * 0.6), extra='opacity="0.7"')
    if tarp:  # a blue tarp thrown over the top and down one side, its hem uneven
        rnd = random.Random(int(z0 * 10))
        drape = [P(*surf(math.radians(a), math.radians(b))) for a in range(-60, 181, 10) for b in range(20, 91, 10)]
        hem = [P(*surf(math.radians(a), math.radians(12 + 14 * rnd.random()))) for a in range(-60, 181, 15)]
        poly(hull(drape + hem), fill=TARP, stroke='#101a3a', w=w * 0.8)
        for a in (-20, 40, 100):
            crease = [surf(math.radians(a + 6 * math.sin(b)), math.radians(b)) for b in range(18, 88, 6)]
            for run in visible_runs(crease):
                pline(run, '#3a5ab0', max(0.5, w * 0.4), extra='opacity="0.5"')
    # the door: a D-shaped zip opening on the front, lit from inside, its panel rolled to one side
    dw, hd = 0.22 * wd, 0.62 * ht
    d_world = [on_front(xm - dw, 0.0)]
    d_world += [on_front(xm - dw, hd * t) for t in (0.3, 0.6, 0.85)]
    d_world += [on_front(xm + dw * math.cos(math.radians(a)) * -1, hd * 0.85 + hd * 0.15 * math.sin(math.radians(a)))
                for a in range(0, 181, 20)]
    d_world += [on_front(xm + dw, hd * t) for t in (0.85, 0.6, 0.3)] + [on_front(xm + dw, 0.0)]
    door = [P(*p) for p in d_world]
    cid = f'tent{int(z0 * 10)}'
    defs.append(f'<clipPath id="{cid}"><polygon points="{pts(outline)}"/></clipPath>')
    dc = P(*on_front(xm, hd * 0.4))
    out.append(f'<g clip-path="url(#{cid})"><circle cx="{dc[0]:.1f}" cy="{dc[1]:.1f}" r="{F * 0.9 / z0:.1f}" '
               f'fill="url(#bloom)"/></g>')
    poly(door, fill='url(#tentlight)')
    reflect(d_world, LAMPLIGHT, 0.2, stretch=1.0)
    roll = [P(*on_front(xm - dw - 0.12, hd * t)) for t in (0.0, 0.9)]
    line(roll[0], roll[1], '#6a4a2a', max(1.2, w * 1.6))
    if antenna:
        a, b = P(xm + 0.2, ht * 0.95, zc), P(xm + 0.2, ht * 1.85, zc)
        line(a, b, COPPER, w); pad(b, F * 0.12 / z0, COPPER)
    ground_pad(xm, z0, 0.2, COPPER)
for spec in reversed(TENTS):
    tent(*spec)

defs.append('<filter id="wet" x="-5%" y="-5%" width="110%" height="115%">'
            '<feGaussianBlur stdDeviation="1.4 5"/></filter>')
bands, rnd, z = [f'<rect width="{W}" height="{H}" fill="white"/>'], random.Random(5), 2.6
while z < 300:
    y, y_far = VPY + F * EYE / z, VPY + F * EYE / (z * 1.07)
    gap = y - y_far
    bands.append(f'<rect x="0" y="{y - 0.4 * gap:.1f}" width="{W}" height="{0.4 * gap:.2f}" fill="black" '
                 f'opacity="{0.35 + 0.35 * rnd.random():.2f}"/>')
    z *= 1.07
defs.append(f'<mask id="ripples" maskUnits="userSpaceOnUse" x="0" y="0" width="{W}" height="{H}">'
            + ''.join(bands) + '</mask>')
# the road's surface, seen from the kerb you stand on: the kerb's top edge hides the road's near
# edge on your side; the far kerb's face drops to it on the other
road = [P(CURB_L, 0, 1.2), P(CURB_L, 0, ZF), P(CURB_R, -0.15, ZF), P(CURB_R, -0.15, 1.2)]
defs.append(f'<linearGradient id="sheen" gradientUnits="userSpaceOnUse" x1="0" y1="{VPY}" x2="0" y2="{VPY + 160}">'
            f'<stop offset="0" stop-color="{SKY_GLOW}"/><stop offset="1" stop-color="{SKY_GLOW}" stop-opacity="0"/>'
            '</linearGradient>')
REFL.insert(0, f'<rect x="0" y="{VPY}" width="{W}" height="160" fill="url(#sheen)"/>')
defs.append(f'<clipPath id="road"><polygon points="{pts(road)}"/></clipPath>')
out.insert(REFL_AT, '<g clip-path="url(#road)"><g filter="url(#wet)" mask="url(#ripples)">'
           + ''.join(REFL) + '</g></g>')

svg = (f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}">'
       f'<defs>{"".join(defs)}</defs>' + ''.join(out) + '</svg>')
Path(__file__).with_name('mural-street-night.svg').write_text(svg, encoding='utf-8')
