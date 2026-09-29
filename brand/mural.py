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
GREY_D, GREY_DD, FACADE, SHUTTER = '#3a3e44', '#23262a', '#141517', '#2a2d31'  # street buildings, poles
LEFT_W, CURB_L, CURB_R, RIGHT_W = -11.0, -5.5, 5.5, 10.0

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
def pad(c, r, color): circle(c, r, color); circle(c, r * 0.42, DRILL)
def ground_pad(x, z, r, color, drill=True):
    c = P(x, 0, z); rx = F * r / z; ry = F * EYE * r / (z * z)
    ellipse(c, rx, ry, color)
    if drill: ellipse(c, rx * 0.42, ry * 0.42, DRILL)

out.append(f'<rect width="{W}" height="{H}" fill="{BG}"/>')

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
        col = TEAL_EDGE if near else TEAL_LINE
        x0, x1 = x - w / 2, x + w / 2
        bl, br = Q(x0, 0, z), Q(x1, 0, z)
        if top == 'pyramid':
            shape = [bl, Q(x0 + 1, h * 0.74, z), Q(x, h, z), Q(x1 - 1, h * 0.74, z), br]
        elif top == 'chamfer':
            c = w * 0.22
            shape = [bl, Q(x0, h - c, z), Q(x0 + c, h, z), Q(x1 - c, h, z), Q(x1, h - c, z), br]
        else:
            shape = [bl, Q(x0, h, z), Q(x1, h, z), br]
        poly(shape, fill=BG, stroke=col, w=1.6 if near else 1.2)
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
                circle(Q(xx, yy, z), 1.3, NEON_ORANGE if bright else '#5aa8a2')
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
            # every window is on, warm or teal, except behind the headline
            fill, edge, glow, alpha = [('#3a1f10', '#8a4a2a', NEON_ORANGE, 0.14),
                                       ('#10333a', '#2f7a80', COOL, 0.1),
                                       ('#2a160c', '#5a3420', NEON_ORANGE, 0.06)][min(2, int(random.random() * 2.6))]
            if behind_headline(wq):
                poly(wq, fill='#16171a', stroke=GREY_DD, w=sw(zz, 0.04, 0.5, 1.4))
            else:
                poly(wq, fill=fill, stroke=edge, w=sw(zz, 0.04, 0.5, 1.4))
                reflect([(xw, fl, zz), (xw, fl + 2.0, zz), (xw, fl + 2.0, zz + 1.4), (xw, fl, zz + 1.4)],
                        glow, alpha, stretch=1.25)
            zz += 3.0
        fl += 3.4
    if lit_shop:
        a, b = z0 + 1.0, z1 - 1.0
        poly([P(xw, 0.15, a), P(xw, 3.3, a), P(xw, 3.3, b), P(xw, 0.15, b)], fill='#1d3c3e', stroke=COOL, w=sw(z0, 0.07, 0.8, 2.0))
        reflect([(xw, 0.15, a), (xw, 3.3, a), (xw, 3.3, b), (xw, 0.15, b)], COOL, 0.35, stretch=1.6)
        for k in range(1, 4):
            zz = a + (b - a) * k / 4
            line(P(xw, 0.15, zz), P(xw, 3.3, zz), COOL, sw(zz, 0.03, 0.6, 1.2))
        line(P(xw, 1.1, a), P(xw, 1.1, b), COOL, sw(z0, 0.03, 0.6, 1.2))
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
ZF = 240
for xc in (CURB_L, CURB_R, LEFT_W, RIGHT_W):
    line(P(xc, 0, 2.5), P(xc, 0, ZF), TEAL_LINE, 1.4)
z = 2.8
while z < 44:
    line(P(LEFT_W, 0, z), P(CURB_L, 0, z), '#14232a', sw(z, 0.03, 0.5, 1.5)); z *= 1.12
for i in range(11):
    xs = -5.0 + i * 1.0
    poly([P(xs, 0, 46), P(xs + 0.5, 0, 46), P(xs + 0.5, 0, 51), P(xs, 0, 51)], fill='#17282e')
ground_pad(2.6, 7.5, 0.55, '#1c2e34')   # a manhole cover, pad-shaped

REFL_AT = len(out)  # the reflection layer goes here, on the ground

# ---------------- Gate ----------------
GZ = 82
gc = '#c23a2a'
for xp in (-9.2, 8.6):
    poly([P(xp - 0.4, 0, GZ), P(xp - 0.4, 8.2, GZ), P(xp + 0.4, 8.2, GZ), P(xp + 0.4, 0, GZ)], fill=BG, stroke=gc, w=1.6)
poly([P(-10.0, 6.4, GZ), P(-10.0, 7.0, GZ), P(9.4, 7.0, GZ), P(9.4, 6.4, GZ)], fill=BG, stroke=gc, w=1.4)
poly([P(-10.4, 8.2, GZ), P(-10.4, 8.8, GZ), P(9.8, 8.8, GZ), P(9.8, 8.2, GZ)], fill=BG, stroke=gc, w=1.4)
poly([P(-12.0, 9.4, GZ), P(-11.2, 8.8, GZ), P(10.6, 8.8, GZ), P(11.4, 9.4, GZ), P(8.6, 10.8, GZ), P(-9.2, 10.8, GZ)], fill=BG, stroke=gc, w=1.6)
pad(P(-12.0, 9.4, GZ), 3.0, gc); pad(P(11.4, 9.4, GZ), 3.0, gc); pad(P(-0.3, 10.8, GZ), 3.0, gc)
poly([P(-1.8, 7.2, GZ), P(-1.8, 8.0, GZ), P(1.2, 8.0, GZ), P(1.2, 7.2, GZ)], fill=BG, stroke=gc, w=1.2)

# ---------------- Streetlights (right curb) ----------------
# Modern LED streetlights: a grey pole and an arm over the road, a slim head, cool light from an
# LED strip on its underside, a faint cone to the ground and a pool of light where it lands.
defs.append(f'<linearGradient id="cone" x1="0" y1="0" x2="0" y2="1">'
            f'<stop offset="0" stop-color="{COOL}" stop-opacity="0.13"/>'
            f'<stop offset="1" stop-color="{COOL}" stop-opacity="0"/></linearGradient>')
LAMPS = (22, 42, 64, 96, 150)
for zl in LAMPS:
    line(P(5.9, 0, zl), P(5.9, 6.8, zl), GREY_D, sw(zl, 0.14, 0.8, 3))
    line(P(5.9, 6.8, zl), P(4.95, 6.95, zl), GREY_D, sw(zl, 0.1, 0.8, 2.5))
    poly([P(4.15, 6.86, zl), P(4.95, 6.86, zl), P(5.7, 0, zl), P(3.4, 0, zl)], fill='url(#cone)')
    ellipse(P(4.55, 0, zl), F * 1.3 / zl, F * EYE * 1.3 / zl ** 2, COOL, extra='opacity="0.09"')
    poly([P(4.1, 7.02, zl), P(4.98, 7.02, zl), P(4.98, 6.88, zl), P(4.1, 6.88, zl)], fill=SHUTTER, stroke=GREY_D,
         w=sw(zl, 0.03, 0.6, 1.4))
    line(P(4.18, 6.87, zl), P(4.9, 6.87, zl), COOL, max(1.0, F * 0.05 / zl))
    streak(4.55, 6.87, zl, 0.7, COOL, 0.35)

# ---------------- Blade signs ----------------
# 茶楼 tea house, 电器维修 appliance repair, 饭店 restaurant, 腾振, 药材 herbal medicine.
def sign(xs0, xs1, y0, y1, z, chars, color, refl=0.15):
    q = [(xs0, y0, z), (xs0, y1, z), (xs1, y1, z), (xs1, y0, z)]
    reflect(q, color, refl, stretch=1.4)
    poly([P(*p) for p in q], fill='#160e14', stroke=color, w=sw(z, 0.08, 0.8, 2.2))
    n = len(chars); step = (y1 - y0) / n
    size = F * min(xs1 - xs0, step) * 0.64 / z
    for i, ch in enumerate(chars):
        text(P((xs0 + xs1) / 2, y1 - step * (i + 0.5), z), ch, size, color)
sign(-10.9, -9.5, 4.6, 9.8, 26, '茶楼', NEON_RED)
sign(8.6, 9.9, 4.2, 10.2, 36, '电器维修', NEON_MAGENTA, refl=0.35)
sign(-10.9, -9.8, 4.6, 9.0, 54, '饭店', NEON_ORANGE)
sign(8.7, 9.9, 4.2, 8.4, 60, '腾振', NEON_RED, refl=0.3)
sign(8.8, 9.9, 4.4, 8.2, 92, '药材', NEON_MAGENTA)

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
def lantern(x, y, z):
    c = P(x, y, z)
    rx, ry = F * 0.26 / z, F * 0.21 / z
    cap_w, cap_h = rx * 0.55, max(0.8, ry * 0.18)
    if HEADLINE[0] < c[0] < HEADLINE[2] and HEADLINE[1] < c[1] < HEADLINE[3]:
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
    streak(x, y, z, 0.4, LANTERN_RED, 0.2)
for zl in (16, 21, 27, 34, 43, 55, 70, 90, 118):
    xsamp = [LEFT_W + i * (RIGHT_W - LEFT_W) / 30 for i in range(31)]
    mid, half = (LEFT_W + RIGHT_W) / 2, (RIGHT_W - LEFT_W) / 2
    ys = [8.4 - 1.0 * (1 - ((xx - mid) / half) ** 2) for xx in xsamp]
    pline([P(xx, yy, zl) for xx, yy in zip(xsamp, ys)], '#3a2a24', sw(zl, 0.025, 0.5, 1.0))
    for i in range(1, 30, 2):
        xx, yy = xsamp[i], ys[i]
        line(P(xx, yy, zl), P(xx, yy - 0.25, zl), '#5a3a1c', sw(zl, 0.02, 0.4, 0.9))
        lantern(xx, yy - 0.25 - 0.21, zl)

# ---------------- A puddle at your feet that holds the towers ----------------
PX, PZ, PRX, PRZ = -7.35, 3.75, 0.95, 0.85
puddle = [P(PX + PRX * math.cos(t) * (1 + 0.12 * math.sin(3 * t)), 0, PZ + PRZ * math.sin(t)) for t in [i * math.pi / 24 for i in range(48)]]
defs.append(f'<clipPath id="puddle"><polygon points="{pts(puddle)}"/></clipPath>')
poly(puddle, fill='#0a1316', stroke=TEAL_DEEP, w=1.2)
out.append('<g clip-path="url(#puddle)" opacity="0.55">')
draw_towers(lambda x, y, z: P(x, -y, z))
out.append('</g>')
ys_ = [p[1] for p in puddle]; y = min(ys_)
rip = ['<g clip-path="url(#puddle)" opacity="0.55">']
while y < max(ys_):
    rip.append(f'<line x1="0" y1="{y:.1f}" x2="{W}" y2="{y:.1f}" stroke="#0a1316" stroke-width="1.6"/>'); y += 4.5
rip.append('</g>'); out.append(''.join(rip))

# ---------------- The network on the ground ----------------
# Every tent is on copper: out of the door, a 45-degree jog, and across to your path, which runs
# from your feet to the tallest tower's door. The buildings above talk by radio.
TENT_LIT = 5.6
TENTS = [(TENT_LIT, 2.2, 1.25, True), (10.4, 2.0, 1.1, False), (13.2, 2.4, 1.3, True), (19.6, 1.8, 1.0, False),
         (22.1, 2.2, 1.2, True), (30.5, 2.0, 1.15, False), (38.0, 2.3, 1.3, True)]
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
for z0, wd, _, _ in TENTS:  # every tent: out of the door, a 45-degree jog, across to your path
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
def tent(z0, wd, ht, antenna):
    xa, xb = -10.9, -10.9 + wd
    xm = (xa + xb) / 2
    hw = 0.2 * ht   # the skirt wall under the roof
    ln = 2.2
    w = sw(z0, 0.045, 0.9, 3.0)
    rope = TEAL_LINE
    # the pool of light on the pavement in front of the door
    ellipse(P(xm, 0, z0 - 0.7), F * 1.25 / z0, F * EYE * 1.25 / (z0 - 0.7) ** 2, 'url(#pool)')
    # guy lines first, so the tent stands in front of them: eave corners out to stakes
    for (x, z), (sx, sz) in [((xa, z0), (xa - 0.55, z0 - 0.45)), ((xb, z0), (xb + 0.55, z0 - 0.45)),
                             ((xb, z0 + ln), (xb + 0.55, z0 + ln + 0.45))]:
        line(P(x, hw, z), P(sx, 0, sz), rope, max(0.6, w * 0.35))
        line(P(sx, 0, sz), P(sx, 0.12, sz), rope, max(0.8, w * 0.5))
    # the side you can see: roof panel from ridge to eave, then the skirt wall
    side = [P(xm, ht, z0), P(xm, ht, z0 + ln), P(xb, hw, z0 + ln), P(xb, hw, z0)]
    poly(side, fill=TEAL_FILL_2, stroke=TEAL_EDGE, w=w)
    poly(side, fill='url(#sideglow)')
    poly([P(xb, hw, z0), P(xb, hw, z0 + ln), P(xb, 0, z0 + ln), P(xb, 0, z0)], fill=TEAL_FILL, stroke=TEAL_EDGE, w=w)
    # the front: an A over the skirt
    front = [P(xa, 0, z0), P(xa, hw, z0), P(xm, ht, z0), P(xb, hw, z0), P(xb, 0, z0)]
    poly(front, fill='#163038', stroke=TEAL_EDGE, w=w)
    poly(front, fill='url(#fabricglow)')
    line(P(xa, hw, z0), P(xb, hw, z0), rope, w * 0.5)   # the seam where roof meets skirt
    # the door: the front's own triangle, smaller, lit from inside, with its flap tied back
    hd, dw = 0.86 * ht, 0.24 * wd
    poly([P(xm - dw, 0, z0), P(xm, hd, z0), P(xm + dw, 0, z0)], fill='url(#tentlight)')
    reflect([(xm - dw, 0, z0), (xm, hd, z0), (xm + dw, 0, z0)], LAMPLIGHT, 0.2, stretch=1.0)
    poly([P(xm, hd, z0), P(xm - dw, 0, z0), P(xm - dw - 0.28 * wd, 0.18 * ht, z0)], fill='url(#flaplit)', stroke=TEAL_EDGE, w=w * 0.5)
    if antenna:
        a, b = P(xm + 0.15, ht, z0 + 0.3), P(xm + 0.15, 1.9 * ht, z0 + 0.3)
        line(a, b, COPPER, w); pad(b, F * 0.12 / z0, COPPER)
    ground_pad(xm, z0, 0.2, COPPER)
for z0, wd, ht, antenna in reversed(TENTS):
    tent(z0, wd, ht, antenna)

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
out.insert(REFL_AT, '<g filter="url(#wet)" mask="url(#ripples)">' + ''.join(REFL) + '</g>')

svg = (f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}">'
       f'<defs>{"".join(defs)}</defs>' + ''.join(out) + '</svg>')
Path(__file__).with_name('mural-street-night.svg').write_text(svg, encoding='utf-8')
