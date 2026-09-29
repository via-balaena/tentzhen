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
BG = '#111111'
COPPER, YELLOW, DIMCOPPER = '#e0aa45', '#ffc81a', '#7a5c26'
GREY, GREY_D, GREY_DD = '#6b7078', '#3a3e44', '#23262a'
FACADE, DRILL = '#141517', '#111111'
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
        col = COPPER if near else '#b58a3c'
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
                lit = rnd.random() < (0.8 if near else 0.62)
                circle(Q(xx, yy, z), 1.3, col if lit else GREY_DD)
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
            lit = zz > 9 and random.random() < 0.2
            poly(wq, fill='#2e2413' if lit else '#16171a', stroke=DIMCOPPER if lit else GREY_DD, w=sw(zz, 0.04, 0.5, 1.4))
            zz += 3.0
        fl += 3.4
    if lit_shop:
        a, b = z0 + 1.0, z1 - 1.0
        poly([P(xw, 0.15, a), P(xw, 3.3, a), P(xw, 3.3, b), P(xw, 0.15, b)], fill='#33270f', stroke=COPPER, w=sw(z0, 0.07, 0.8, 2.0))
        for k in range(1, 4):
            zz = a + (b - a) * k / 4
            line(P(xw, 0.15, zz), P(xw, 3.3, zz), COPPER, sw(zz, 0.03, 0.6, 1.2))
        line(P(xw, 1.1, a), P(xw, 1.1, b), COPPER, sw(z0, 0.03, 0.6, 1.2))
    else:
        for yy in (0.9, 1.5, 2.1, 2.7, 3.3):
            line(P(xw, yy, z0 + 0.8), P(xw, yy, z1 - 0.8), '#2a2d31', sw(z0, 0.03, 0.5, 1.2))

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
    line(P(xc, 0, 2.5), P(xc, 0, ZF), GREY_D, 1.4)
z = 2.8
while z < 44:
    line(P(LEFT_W, 0, z), P(CURB_L, 0, z), '#1c1e21', sw(z, 0.03, 0.5, 1.5)); z *= 1.12
for i in range(11):
    xs = -5.0 + i * 1.0
    poly([P(xs, 0, 46), P(xs + 0.5, 0, 46), P(xs + 0.5, 0, 51), P(xs, 0, 51)], fill='#1f2124')
ground_pad(2.6, 7.5, 0.55, '#24272b')   # a manhole cover, pad-shaped

# ---------------- Reflections on the wet street ----------------
rid = [0]
def reflect(shape_pts_world, color, alpha, step=3):
    """Mirror a vertical shape in the ground plane and draw it as scanlines."""
    pts_m = [P(x, -y, z) for x, y, z in shape_pts_world]
    rid[0] += 1; cid = f'r{rid[0]}'
    defs.append(f'<clipPath id="{cid}"><polygon points="{pts(pts_m)}"/></clipPath>')
    xs = [p[0] for p in pts_m]; ys = [p[1] for p in pts_m]
    y = min(ys)
    g = [f'<g clip-path="url(#{cid})" opacity="{alpha}">']
    while y < max(ys):
        g.append(f'<line x1="{min(xs):.1f}" y1="{y:.1f}" x2="{max(xs):.1f}" y2="{y:.1f}" stroke="{color}" stroke-width="1.3"/>')
        y += step
    g.append('</g>')
    out.append(''.join(g))

# ---------------- Gate ----------------
GZ = 82
gc = '#a88240'
for xp in (-9.2, 8.6):
    poly([P(xp - 0.4, 0, GZ), P(xp - 0.4, 8.2, GZ), P(xp + 0.4, 8.2, GZ), P(xp + 0.4, 0, GZ)], fill=BG, stroke=gc, w=1.6)
poly([P(-10.0, 6.4, GZ), P(-10.0, 7.0, GZ), P(9.4, 7.0, GZ), P(9.4, 6.4, GZ)], fill=BG, stroke=gc, w=1.4)
poly([P(-10.4, 8.2, GZ), P(-10.4, 8.8, GZ), P(9.8, 8.8, GZ), P(9.8, 8.2, GZ)], fill=BG, stroke=gc, w=1.4)
poly([P(-12.0, 9.4, GZ), P(-11.2, 8.8, GZ), P(10.6, 8.8, GZ), P(11.4, 9.4, GZ), P(8.6, 10.8, GZ), P(-9.2, 10.8, GZ)], fill=BG, stroke=gc, w=1.6)
pad(P(-12.0, 9.4, GZ), 3.0, gc); pad(P(11.4, 9.4, GZ), 3.0, gc); pad(P(-0.3, 10.8, GZ), 3.0, gc)
poly([P(-1.8, 7.2, GZ), P(-1.8, 8.0, GZ), P(1.2, 8.0, GZ), P(1.2, 7.2, GZ)], fill=BG, stroke=gc, w=1.2)

# ---------------- Streetlights (right curb) ----------------
LAMPS = (22, 42, 64, 96, 150)
for zl in LAMPS:
    base, topp, arm = P(5.9, 0, zl), P(5.9, 6.6, zl), P(4.8, 6.6, zl)
    line(base, topp, GREY_D, sw(zl, 0.14, 0.8, 3)); line(topp, arm, GREY_D, sw(zl, 0.12, 0.8, 3))
for zl in LAMPS:
    reflect([(4.65, 2.2, zl), (4.65, 6.8, zl), (4.95, 6.8, zl), (4.95, 2.2, zl)], YELLOW, 0.3, 2)
for zl in LAMPS:
    pad(P(4.8, 6.6, zl), max(1.8, F * 0.2 / zl), YELLOW)

# ---------------- Blade signs ----------------
# 茶楼 tea house, 电器维修 appliance repair, 饭店 restaurant, 腾振, 药材 herbal medicine.
def sign(xs0, xs1, y0, y1, z, chars, color, refl=0.0):
    q = [(xs0, y0, z), (xs0, y1, z), (xs1, y1, z), (xs1, y0, z)]
    if refl: reflect(q, color, refl)
    poly([P(*p) for p in q], fill='#131416', stroke=color, w=sw(z, 0.08, 0.8, 2.2))
    n = len(chars); step = (y1 - y0) / n
    size = F * min(xs1 - xs0, step) * 0.64 / z
    for i, ch in enumerate(chars):
        text(P((xs0 + xs1) / 2, y1 - step * (i + 0.5), z), ch, size, color)
sign(-10.9, -9.5, 4.6, 9.8, 26, '茶楼', DIMCOPPER)
sign(8.6, 9.9, 4.2, 10.2, 36, '电器维修', YELLOW, refl=0.28)
sign(-10.9, -9.8, 4.6, 9.0, 54, '饭店', DIMCOPPER)
sign(8.7, 9.9, 4.2, 8.4, 60, '腾振', COPPER, refl=0.22)
sign(8.8, 9.9, 4.4, 8.2, 92, '药材', DIMCOPPER)
# the lit repair shop's glow on the pavement
reflect([(10.0, 0.2, 33), (10.0, 3.3, 33), (10.0, 3.3, 51), (10.0, 0.2, 51)], COPPER, 0.18)

# ---------------- Lanterns ----------------
for zl in (18, 27, 38, 52, 72, 104):
    xsamp = [LEFT_W + i * (RIGHT_W - LEFT_W) / 30 for i in range(31)]
    mid, half = (LEFT_W + RIGHT_W) / 2, (RIGHT_W - LEFT_W) / 2
    ys = [8.4 - 1.0 * (1 - ((xx - mid) / half) ** 2) for xx in xsamp]
    pline([P(xx, yy, zl) for xx, yy in zip(xsamp, ys)], '#8a6a2e', sw(zl, 0.025, 0.5, 1.0))
    for i in range(3, 30, 3):
        xx, yy = xsamp[i], ys[i]
        a, b = P(xx, yy, zl), P(xx, yy - 0.4, zl)
        line(a, b, GREY_D, sw(zl, 0.02, 0.4, 0.9))
        pad(b, max(1.3, F * 0.2 / zl), COPPER)

# ---------------- A puddle at your feet that holds the towers ----------------
PX, PZ, PRX, PRZ = -7.35, 3.75, 0.95, 0.85
puddle = [P(PX + PRX * math.cos(t) * (1 + 0.12 * math.sin(3 * t)), 0, PZ + PRZ * math.sin(t)) for t in [i * math.pi / 24 for i in range(48)]]
defs.append(f'<clipPath id="puddle"><polygon points="{pts(puddle)}"/></clipPath>')
poly(puddle, fill='#0c0d0f', stroke='#2a2d31', w=1.2)
out.append('<g clip-path="url(#puddle)" opacity="0.55">')
draw_towers(lambda x, y, z: P(x, -y, z))
out.append('</g>')
ys_ = [p[1] for p in puddle]; y = min(ys_)
rip = ['<g clip-path="url(#puddle)" opacity="0.55">']
while y < max(ys_):
    rip.append(f'<line x1="0" y1="{y:.1f}" x2="{W}" y2="{y:.1f}" stroke="#0c0d0f" stroke-width="1.6"/>'); y += 4.5
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

spill = P(-9.8, 0, TENT_LIT - 0.9)
ellipse(spill, F * 0.8 / (TENT_LIT - 0.9), F * EYE * 0.8 / (TENT_LIT - 0.9) ** 2, YELLOW, extra='opacity="0.08"')
poly(ribbon(MAIN[0], MAIN[1], 0.24), fill=COPPER, extra='opacity="0.12"')
poly(ribbon(MAIN[0], MAIN[1], 0.075), fill=COPPER, stroke=COPPER, w=0.8)
for z0, wd, _, _ in TENTS:  # every tent: out of the door, a 45-degree jog, across to your path
    xm = -10.9 + wd / 2
    trace([(xm, z0), (xm + 0.6, z0 - 0.6), (CAM_X, z0 - 0.6)], vias=[(CAM_X, z0 - 0.6)])
pad(P(CAM_X, 0.0, 640.0), 3.6, COPPER)

# ---------------- Tents along the left wall ----------------
def tent(z0, wd, ht, antenna):
    xa, xb = -10.9, -10.9 + wd
    xm = (xa + xb) / 2
    prof = [(xa, 0), (xa, 0.37 * ht), (xa + 0.35, 0.82 * ht), (xm, ht), (xb - 0.35, 0.82 * ht), (xb, 0.37 * ht), (xb, 0)]
    ln = 2.2
    stroke = COPPER
    w = sw(z0, 0.045, 0.9, 3.0)
    right_half = prof[3:]
    side = [P(x, y, z0) for x, y in right_half] + [P(x, y, z0 + ln) for x, y in reversed(right_half)]
    poly(side, fill='#1a1611', stroke=stroke, w=w)
    poly([P(x, y, z0) for x, y in prof], fill='#3a2c12', stroke=stroke, w=w)
    pole = '#8a6a2e'
    line(P(xa, 0, z0), P(xb - 0.35, 0.82 * ht, z0), pole, w * 0.6); line(P(xb, 0, z0), P(xa + 0.35, 0.82 * ht, z0), pole, w * 0.6)
    d = 0.4
    poly([P(xm - d, 0, z0), P(xm - d, 0.55, z0), P(xm, 0.8, z0), P(xm + d, 0.55, z0), P(xm + d, 0, z0)],
         fill=YELLOW, stroke=stroke, w=w * 0.8)
    if antenna:
        a, b = P(xm - 0.5, 0.92 * ht, z0), P(xm - 0.5, 1.84 * ht, z0)
        line(a, b, COPPER, w); pad(b, F * 0.12 / z0, COPPER)
    ground_pad(xm, z0, 0.2, COPPER)
for z0, wd, ht, antenna in reversed(TENTS):
    tent(z0, wd, ht, antenna)

svg = (f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}">'
       f'<defs>{"".join(defs)}</defs>' + ''.join(out) + '</svg>')
Path(__file__).with_name('mural-street-night.svg').write_text(svg, encoding='utf-8')
