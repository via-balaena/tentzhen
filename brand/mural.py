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

W, H = 1600, 1000
CAM_X, EYE, F, VPX, VPY = -7.6, 1.6, 820.0, 800.0, 540.0
# Palette. Copper belongs to the network alone: the traces in the ground, antennas, rooftop pads
# and radio. Everything else takes its colour from a Chinatown street at night: red lanterns and
# sign boards, magenta neon, brick, cool white street and shop light, lamplight.
BG, DRILL = '#111111', '#111111'
COPPER = '#e0aa45'                                                   # the network
COOL = '#9fe0d8'                                                     # street and shop light
NEON_MAGENTA = '#ff3fa4'                                             # the repair shop's neon
LAMPLIGHT = '#ffb35a'                                                # a lamp's warm light
HOMELIGHT = '#f0dfb8'                                                # a lamp in a home window
GREY_D, GREY_DD, SHUTTER = '#3a3e44', '#23262a', '#2a2d31'          # building trim, shutters, poles
METAL, METAL_FAR, ALUMINIUM = '#7a8088', '#555b63', '#8a9098'         # towers, window and shop frames
BOARD_RED, BOARD_GOLD = '#b8261c', '#f5dca0'                         # sign boards and their lettering
# One night, one set of lights, for everything but the network: LED streetlights along the far kerb
# (cool, from above and across the street), the lanterns and signs overhead (warm red), each tent's
# lamp (amber, from inside), and the windows. Every surface is a solid, shaded by those lights and
# dimmed toward grey the way night dims colour; only a light source is bright. The network on the
# ground is copper inlaid in the concrete and lit like the rest; in the sky it is the exception on
# purpose: flat copper antennas, pads and radio drawn over the scene.
LEFT_W, CURB_L, CURB_R, RIGHT_W = -11.0, -6.5, 5.5, 10.0  # a 4.5 m sidewalk under you, the road, a 4.5 m one
TENT_WALL = -10.9  # the tents stand with their backs 10 cm off the left wall

def P(x, y, z): return (VPX + F * (x - CAM_X) / z, VPY - F * (y - EYE) / z)
def sw(z, k=0.10, lo=0.8, hi=4.0): return max(lo, min(hi, F * k / z))
def pts(ps): return ' '.join(f'{x:.1f},{y:.1f}' for x, y in ps)

defs, out = [], []
def poly(ps, fill='none', stroke='none', w=1.0, extra=''):
    edge = f' stroke="{stroke}" stroke-width="{w:.2f}" stroke-linejoin="round"' if stroke != 'none' else ''
    out.append(f'<polygon points="{pts(ps)}" fill="{fill}"{edge} {extra}/>')
def line(a, b, stroke, w, extra=''):
    out.append(f'<line x1="{a[0]:.1f}" y1="{a[1]:.1f}" x2="{b[0]:.1f}" y2="{b[1]:.1f}" stroke="{stroke}" stroke-width="{w:.2f}" stroke-linecap="round" {extra}/>')
def pline(ps, stroke, w, extra=''):
    out.append(f'<polyline points="{pts(ps)}" fill="none" stroke="{stroke}" stroke-width="{w:.2f}" stroke-linecap="round" stroke-linejoin="round" {extra}/>')
def circle(c, r, fill, extra=''):
    out.append(f'<circle cx="{c[0]:.1f}" cy="{c[1]:.1f}" r="{r:.2f}" fill="{fill}" {extra}/>')
def ellipse(c, rx, ry, fill, extra=''):
    out.append(f'<ellipse cx="{c[0]:.1f}" cy="{c[1]:.1f}" rx="{rx:.2f}" ry="{ry:.2f}" fill="{fill}" {extra}/>')
FONT = 'font-family="Noto Sans SC, PingFang SC, sans-serif" font-weight="500"'
def text(c, s, size, fill, extra=''):
    out.append(f'<text x="{c[0]:.1f}" y="{c[1]:.1f}" font-size="{size:.1f}" fill="{fill}" text-anchor="middle" dominant-baseline="central" {FONT} {extra}>{s}</text>')
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
# collected here and drawn as one layer, clipped to the road: after the road markings, before
# anything that stands on the street.
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
# A skyline like Shenzhen's or Chongqing's at night: glass towers packed in layers, the farther ones
# paler in the haze. Each tower is a solid in dark glass that catches the city's glow: its front,
# and the side it turns toward you a shade lighter; slab lines across it; offices lit in runs along
# some floors, cut into panes by vertical fins; LED lines on the crowns of a few. Shapes after the
# two cities' landmarks: a tapered tower with corner columns, a stepped crown and a spire (the
# tallest, where your path ends), a tapered cylinder with a rounded top and ribs, a slab with a
# cupped crown, twin towers joined high up by a lit skybridge, and stepped setbacks.
LED_CYAN, LED_WHITE, OFFICE_COOL = '#7fdfe6', '#e6eef0', '#cfe6e8'
def glass_gradients(shades):  # (id, top, bottom): the glass darkest high up, catching the sky low down
    for gid, top, bottom in shades:
        defs.append(f'<linearGradient id="{gid}" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="0" y2="{VPY}">'
                    f'<stop offset="0" stop-color="{top}"/><stop offset="1" stop-color="{bottom}"/></linearGradient>')
glass_gradients((('glass', '#101216', '#1c1b21'), ('glassside', '#16181d', '#24232a'), ('glassfar', '#17171b', '#221f24')))
FLOOR = 4.2  # floor to floor, m
towers = [  # (x, z, width, roof height, shape): the layer in front
    (-120, 780, 52, 230, 'setback'), (-88, 660, 36, 262, 'slab'), (-72, 612, 22, 300, 'twin'),
    (-34, 612, 22, 300, 'twin'), (-30, 760, 46, 336, 'slab'), (-7.6, 640, 40, 385, 'crown'),
    (26, 580, 30, 318, 'bamboo'), (100, 740, 44, 370, 'cup'), (82, 620, 34, 262, 'setback'),
    (112, 760, 54, 300, 'slab'), (144, 680, 40, 228, 'slab'),
]
towers.sort(key=lambda t: -t[1])
LED = {'crown': LED_WHITE, 'bamboo': LED_CYAN, 'cup': LED_CYAN}
def half_width(w, h, shape, y):  # half the tower's width at height y
    if shape == 'crown':
        return w / 2 + (0.31 * w - w / 2) * min(1.0, y / (0.88 * h)) if y < 0.88 * h else (0.26 * w if y < 0.94 * h else 0.18 * w)
    if shape == 'bamboo':
        if y < 0.84 * h:
            return w / 2 + (0.4 * w - w / 2) * y / (0.84 * h)
        return 0.4 * w * math.sqrt(max(0.0, 1 - ((y - 0.84 * h) / (0.16 * h)) ** 2))
    if shape == 'setback':
        return w / 2 if y < 0.55 * h else (0.37 * w if y < 0.8 * h else 0.25 * w)
    return w / 2
def outline(x, w, h, shape):  # the front face, bottom left round to bottom right
    if shape == 'crown':
        left = [(x - w / 2, 0), (x - 0.31 * w, 0.88 * h), (x - 0.26 * w, 0.88 * h), (x - 0.26 * w, 0.94 * h),
                (x - 0.18 * w, 0.94 * h), (x - 0.18 * w, h)]
    elif shape == 'bamboo':
        left = [(x - w / 2, 0), (x - 0.4 * w, 0.84 * h)]
        left += [(x - 0.4 * w * math.cos(math.radians(a)), 0.84 * h + 0.16 * h * math.sin(math.radians(a)))
                 for a in range(10, 91, 10)]
    elif shape == 'setback':
        left = [(x - w / 2, 0), (x - w / 2, 0.55 * h), (x - 0.37 * w, 0.55 * h), (x - 0.37 * w, 0.8 * h),
                (x - 0.25 * w, 0.8 * h), (x - 0.25 * w, h)]
    elif shape == 'cup':  # the crown dips between its two top corners
        left = [(x - w / 2, 0), (x - w / 2, h)]
        left += [(x - w / 2 * math.cos(math.radians(a)), h - 0.1 * h * math.sin(math.radians(a)))
                 for a in range(10, 91, 10)]
    else:
        left = [(x - w / 2, 0), (x - w / 2, h)]
    return left + [(2 * x - px, py) for px, py in reversed(left)]
def roof(x, w, h, shape):  # where the rooftop pad sits
    return (x + w / 2 - 1.5, h) if shape == 'cup' else (x, h)
def floor_top(h, shape):
    return {'crown': 0.86, 'bamboo': 0.82, 'cup': 0.88}.get(shape, 0.97) * h - 2
def lit_runs(rnd, runs, z, xa, xb, y, p_lit):
    # one floor's offices, lit in runs; each run pattern is its own path so floors differ
    if xb - xa < 2 or rnd.random() > p_lit:
        return
    key = (rnd.random() < 0.5, rnd.randrange(3))
    period = sum(RUN_PATTERNS[key[1]])
    a, b = P(xa - rnd.random() * period, y, z), P(xb, y, z)
    runs.setdefault(key, []).append(f'M{a[0]:.1f} {a[1]:.1f}H{b[0]:.1f}')
RUN_PATTERNS = [(9, 3), (5, 2, 12, 4), (3, 1.5, 3, 6)]  # lit and dark stretches along a floor, m
def draw_runs(runs, z, band, opacity, clip):
    for (warm, k), d in sorted(runs.items()):
        dash = ' '.join(f'{v * F / z:.2f}' for v in RUN_PATTERNS[k])
        out.append(f'<path d="{"".join(d)}" stroke="{HOMELIGHT if warm else OFFICE_COOL}" stroke-width="{band:.2f}" '
                   f'stroke-dasharray="{dash}" opacity="{opacity}" fill="none" clip-path="url(#{clip})"/>')
def draw_tower(rnd, x, z, w, h, shape, col):
    shp = outline(x, w, h, shape)
    face = [P(px, py, z) for px, py in shp]
    cid = f'tw{int(x * 10)}_{int(z)}'
    defs.append(f'<clipPath id="{cid}"><polygon points="{pts(face)}"/></clipPath>')
    # the side it turns toward you, for the box-shaped ones
    if shape in ('slab', 'twin', 'setback') and (x - w / 2 > CAM_X or x + w / 2 < CAM_X):
        d, left_side = 0.8 * w, x - w / 2 > CAM_X
        tiers = [(0, 0.55 * h, 0.5), (0.55 * h, 0.8 * h, 0.37), (0.8 * h, h, 0.25)] if shape == 'setback' else [(0, h, 0.5)]
        for y0, y1, k in tiers:
            xs = x - k * w if left_side else x + k * w
            side = [P(xs, y0, z), P(xs, y1, z), P(xs, y1, z + d), P(xs, y0, z + d)]
            poly(side, fill='url(#glassside)', stroke=col, w=0.8)
            runs, y = {}, y0 + 3
            while y < min(y1, floor_top(h, shape)) - 1:
                if rnd.random() < 0.5:
                    a, b = P(xs, y, z + 1), P(xs, y, z + d - 1)
                    runs.setdefault((rnd.random() < 0.5, 0), []).append(f'M{a[0]:.1f} {a[1]:.1f}L{b[0]:.1f} {b[1]:.1f}')
                y += FLOOR
            for (warm, _), dd in runs.items():
                out.append(f'<path d="{"".join(dd)}" stroke="{HOMELIGHT if warm else OFFICE_COOL}" '
                           f'stroke-width="{0.45 * FLOOR * F / (z + d / 2):.2f}" opacity="0.55" fill="none"/>')
    poly(face, fill='url(#glass)')
    # slab lines, then the lit offices, then the fins that cut them into panes
    slabs, runs, y = [], {}, 3.0
    while y < floor_top(h, shape):
        hw = half_width(w, h, shape, y)
        a, b = P(x - hw, y, z), P(x + hw, y, z)
        slabs.append(f'M{a[0]:.1f} {a[1]:.1f}H{b[0]:.1f}')
        lit_runs(rnd, runs, z, x - hw + 0.8, x + hw - 0.8, y + 0.55 * FLOOR, 0.6)
        y += FLOOR
    out.append(f'<path d="{"".join(slabs)}" stroke="#262a30" stroke-width="{max(0.4, 0.35 * F / z):.2f}" fill="none"/>')
    draw_runs(runs, z, 0.5 * FLOOR * F / z, 0.85, cid)
    top = floor_top(h, shape)
    fins, n = [], max(3, int(w / (1.6 if shape == 'bamboo' else 3.0)))
    for i in range(1, n):
        u = -1 + 2 * i / n
        ys = [0, top] + ([0.84 * h, 0.93 * h, 0.985 * h] if shape == 'bamboo' else [])
        fins.append('M' + 'L'.join('{:.1f} {:.1f}'.format(*P(x + u * half_width(w, h, shape, yy), yy, z)) for yy in ys))
    rib = shape == 'bamboo'
    out.append(f'<path d="{"".join(fins)}" stroke="{METAL_FAR if rib else "#0e1013"}" '
               f'stroke-width="{0.9 if rib else 0.8}" opacity="{0.6 if rib else 0.75}" fill="none"/>')
    if shape == 'crown':  # its corner columns, running the length of the shaft
        for sgn in (-1, 1):
            line(P(x + sgn * 0.42 * w, 0, z), P(x + sgn * 0.27 * w, 0.88 * h, z), col, 1.2)
        a, b = P(x, h, z), P(x, h + 34, z)
        line(a, b, col, 1.6); pad(b, 3.6, col)
    poly(face, stroke=col, w=1.6 if z < 650 else 1.2)
    if shape in LED:  # LED lines along the crown
        crown = [P(px, py, z) for px, py in shp if py >= 0.8 * h]
        for width, alpha in ((4.0, 0.18), (1.1, 0.9)):
            pline(crown, LED[shape], width, extra=f'opacity="{alpha}"')
def haze(gid, depth, alpha):  # the air between layers, thicker toward the horizon
    defs.append(f'<linearGradient id="{gid}" gradientUnits="userSpaceOnUse" x1="0" y1="{VPY - depth}" x2="0" y2="{VPY}">'
                f'<stop offset="0" stop-color="{SKY_GLOW}" stop-opacity="0"/>'
                f'<stop offset="1" stop-color="{SKY_GLOW}" stop-opacity="{alpha}"/></linearGradient>')
    out.append(f'<rect x="0" y="{VPY - depth}" width="{W}" height="{depth}" fill="url(#{gid})"/>')
def draw_towers(far_x=230, far_n=20):
    # the far layer: plain slabs in the haze, a few floors lit, spread across far_x either side
    far = random.Random(21)
    back = sorted([(far.uniform(-far_x, far_x), far.uniform(950, 1500), far.uniform(22, 50), far.uniform(140, 330))
                   for _ in range(far_n)], key=lambda t: -t[1])
    for x, z, w, h in back:
        face = [P(x - w / 2, 0, z), P(x - w / 2, h, z), P(x + w / 2, h, z), P(x + w / 2, 0, z)]
        poly(face, fill='url(#glassfar)', stroke=METAL_FAR, w=0.8, extra='stroke-opacity="0.5"')
        runs, y = {}, 3.0
        while y < h - 3:
            lit_runs(far, runs, z, x - w / 2 + 0.8, x + w / 2 - 0.8, y + 0.55 * FLOOR, 0.35)
            y += FLOOR
        cid = f'tf{int(x * 10)}_{int(z)}'
        defs.append(f'<clipPath id="{cid}"><polygon points="{pts(face)}"/></clipPath>')
        draw_runs(runs, z, 0.5 * FLOOR * F / z, 0.45, cid)
    haze('haze_far', 300, 0.5)
    rnd, twins = random.Random(3), []
    for x, z, w, h, shape in towers:
        draw_tower(rnd, x, z, w, h, shape, METAL if z < 650 else METAL_FAR)
        if shape == 'twin':
            twins.append((x, z, w, h))
            if len(twins) == 2:  # the skybridge across the gap between them, two floors of glass, lit
                (xa, za, wa, ha), (xb, _, wb, hb) = sorted(twins)
                x0, x1, y0 = xa + wa / 2, xb - wb / 2, 0.66 * min(ha, hb)
                poly([P(x0, y0, za), P(x0, y0 + 2 * FLOOR, za), P(x1, y0 + 2 * FLOOR, za), P(x1, y0, za)],
                     fill='url(#glass)', stroke=METAL, w=1.0)
                for k in (0, 1):
                    yy = y0 + (k + 0.55) * FLOOR
                    out.append(f'<line x1="{P(x0, yy, za)[0]:.1f}" y1="{P(x0, yy, za)[1]:.1f}" x2="{P(x1, yy, za)[0]:.1f}" '
                               f'y2="{P(x1, yy, za)[1]:.1f}" stroke="{OFFICE_COOL}" stroke-width="{0.5 * FLOOR * F / za:.2f}" '
                               f'stroke-dasharray="{3 * F / za:.2f} {1 * F / za:.2f}" opacity="0.8"/>')
    haze('haze_near', 160, 0.25)
    # Every rooftop linked to its neighbours by an arc through the sky.
    by_x = sorted(towers, key=lambda t: t[0])
    for ta, tb in zip(by_x, by_x[1:]):
        (xa, ha), (xb, hb) = roof(ta[0], ta[2], ta[3], ta[4]), roof(tb[0], tb[2], tb[3], tb[4])
        a, b = P(xa, ha, ta[1]), P(xb, hb, tb[1])
        c = P((xa + xb) / 2, max(ha, hb) + 45, (ta[1] + tb[1]) / 2)
        radio_link(a, b, c)
    for x, z, w, h, shape in towers:
        pad(P(*roof(x, w, h, shape), z), 2.4, COPPER)

draw_towers()

# ---------------- Street facades ----------------
TENT_STRETCH = (4.0, 41.0)  # the stretch of the left wall where the tents stand, front to back
# The site sets its headline over the top left. At the narrowest desktop width (1280 px) it covers
# about this box of the mural; windows touching it stay dark so the words keep their contrast.
HEADLINE = (60, 20, 780, 480)
defs.append('<filter id="soft" x="-20%" y="-20%" width="140%" height="140%"><feGaussianBlur stdDeviation="30"/></filter>')
defs.append(f'<mask id="quiet" maskUnits="userSpaceOnUse" x="0" y="0" width="{W}" height="{H}">'
            f'<rect width="{W}" height="{H}" fill="white"/>'
            f'<rect x="{HEADLINE[0]}" y="{HEADLINE[1]}" width="{HEADLINE[2] - HEADLINE[0]}" '
            f'height="{HEADLINE[3] - HEADLINE[1]}" fill="black" opacity="0.7" filter="url(#soft)"/></mask>')
def behind_headline(quad):
    xs, ys = [p[0] for p in quad], [p[1] for p in quad]
    return max(xs) > HEADLINE[0] and min(xs) < HEADLINE[2] and max(ys) > HEADLINE[1] and min(ys) < HEADLINE[3]
# The street's commercial buildings are brick, each laid in its own, as if from neighbouring
# quarries: red, brown, buff, clinker, orange and grey, dimmed for night, with paler mortar.
# (brick, mortar)
BRICKS = {'red': ('#2c1512', '#3e2a25'), 'brown': ('#271b15', '#3a2e26'), 'buff': ('#2e2820', '#433d31'),
          'clinker': ('#211518', '#342729'), 'orange': ('#331c14', '#472f25'), 'grey': ('#22201e', '#35322e')}
BRICK, COURSE, JOINT_W = 0.225, 0.075, 0.01  # a brick and a course with their mortar; a joint, m
WINDOWS = random.Random(11)  # which windows are lamplight and which are screens
def brickwork(xw, z0, z1, h, mortar):
    # Running bond. Bed joints are thin wedges toward the vanishing point. Along any vertical line on
    # the wall the courses are evenly spaced, so each head-joint position is one dashed line, the
    # dashes on every other course. Joints finer than the eye can pick out are left to the colour.
    dx = abs(xw - CAM_X)
    z_bed = min(z1, F * COURSE / 1.5)  # courses closer than 1.5 px merge
    if z_bed > z0:
        d = []
        for c in range(1, int(h / COURSE)):
            y = c * COURSE
            q = [P(xw, y, z0), P(xw, y, z_bed), P(xw, y + JOINT_W, z_bed), P(xw, y + JOINT_W, z0)]
            d.append('M' + 'L'.join(f'{x:.1f} {yy:.1f}' for x, yy in q) + 'Z')
        out.append(f'<path d="{"".join(d)}" fill="{mortar}" mask="url(#quiet)"/>')
    k = 0
    while True:
        z = z0 + k * BRICK / 2
        if z >= z1 or F * dx * BRICK / z ** 2 < 3:  # head joints closer than 3 px merge
            break
        a, b = P(xw, 0, z), P(xw, h, z)
        course = F * COURSE / z
        out.append(f'<line x1="{a[0]:.1f}" y1="{a[1]:.1f}" x2="{b[0]:.1f}" y2="{b[1]:.1f}" stroke="{mortar}" mask="url(#quiet)" '
                   f'stroke-width="{max(0.3, F * dx * JOINT_W / z ** 2):.2f}" stroke-dasharray="{course:.2f}" '
                   f'stroke-dashoffset="{course if k % 2 else 0:.2f}"/>')
        k += 1
def wall_light(xw, z0, z1, h):
    # the lanterns and streetlights light a wall up to about 8.5 m; above that it falls dark
    y = 8.5
    while y < h:
        k = min(1.0, (y + 0.25 - 8.5) / 7)
        poly([P(xw, y, z0), P(xw, min(h, y + 0.5), z0), P(xw, min(h, y + 0.5), z1), P(xw, y, z1)],
             fill='#000000', extra=f'opacity="{0.55 * k:.2f}"')
        y += 0.5
def facade(xw, z0, z1, h, brick, lit_shop=False):
    q = [P(xw, 0, z0), P(xw, h, z0), P(xw, h, z1), P(xw, 0, z1)]
    poly(q, fill=BRICKS[brick][0])
    brickwork(xw, z0, z1, h, BRICKS[brick][1])
    wall_light(xw, z0, z1, h)
    poly(q, stroke=GREY_D, w=sw(z0, 0.07, 0.8, 2.2))
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
                                 ('#36322a', HOMELIGHT, 0.05)][min(2, int(WINDOWS.random() * 2.6))]
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
    else:  # a shop in each 5 m bay, brick piers between: shut where the tents are, open everywhere else
        n = max(1, round((z1 - z0) / 5.0))
        rnd = random.Random(int(z0 * 10) + (1000 if xw > 0 else 0))
        for i in range(n):
            a, b = z0 + (z1 - z0) * i / n + 0.8, z0 + (z1 - z0) * (i + 1) / n - 0.8
            name = SHOP_NAMES[rnd.randrange(len(SHOP_NAMES))]
            if xw < 0 and a < TENT_STRETCH[1] and b > TENT_STRETCH[0]:  # by the tents: shut, mostly dark glass
                if i % 3 == 1:
                    shutter(xw, a, b)
                else:
                    shopfront(xw, a, b, None, name, door_near=i % 2 == 0)
            else:
                shopfront(xw, a, b, SHOPS[rnd.randrange(len(SHOPS))], name, door_near=i % 2 == 0)

def shutter(xw, a, b):  # shut for the night: a corrugated roller shutter under its box
    poly([P(xw, 0.0, a), P(xw, 3.1, a), P(xw, 3.1, b), P(xw, 0.0, b)], fill='#1a1c1f', stroke=GREY_D,
         w=sw(a, 0.04, 0.5, 1.4))
    d, yy = [], 0.2
    while yy < 3.1 and F * 0.2 / a >= 1.5:  # slats closer than 1.5 px merge
        u, v = P(xw, yy, a), P(xw, yy, b)
        d.append(f'M{u[0]:.1f} {u[1]:.1f}L{v[0]:.1f} {v[1]:.1f}')
        yy += 0.2
    if d:
        out.append(f'<path d="{"".join(d)}" stroke="{SHUTTER}" stroke-width="{sw(a, 0.02, 0.4, 1.0):.2f}" fill="none"/>')
    poly([P(xw, 3.1, a), P(xw, 3.35, a), P(xw, 3.35, b), P(xw, 3.1, b)], fill=GREY_DD)

# Shops: glass behind aluminium, a glass door with a transom at one end, and a sign band with the
# shop's name. An open shop's glass is lit, brightest under the ceiling lights, with a strip of its
# light under the sign; the light reflects in the road and spills onto the sidewalk. Interiors are
# lamplight, cool shop white or cream, never red. A shut shop's glass is dark and its sign unlit.
def mix(c1, c2, t):
    a_, b_ = [int(c1[i:i + 2], 16) for i in (1, 3, 5)], [int(c2[i:i + 2], 16) for i in (1, 3, 5)]
    return '#' + ''.join(f'{round(u + (v - u) * t):02x}' for u, v in zip(a_, b_))
SHOPS = [('#7a6242', LAMPLIGHT), ('#3a6c69', COOL), ('#6c6450', HOMELIGHT)]  # (glass, light)
for n_, (glass, light) in enumerate(SHOPS):
    defs.append(f'<linearGradient id="shop{n_}" x1="0" y1="0" x2="0" y2="1">'
                f'<stop offset="0" stop-color="{mix(glass, light, 0.55)}"/><stop offset="0.4" stop-color="{glass}"/>'
                f'<stop offset="1" stop-color="{mix(glass, "#000000", 0.35)}"/></linearGradient>')
defs.append('<linearGradient id="shut" x1="0" y1="0" x2="1" y2="1">'  # dark glass, a faint sheen across it
            '<stop offset="0" stop-color="#1a1f22"/><stop offset="0.42" stop-color="#20272b"/>'
            '<stop offset="0.5" stop-color="#15191c"/><stop offset="1" stop-color="#0f1113"/></linearGradient>')
SHOP_NAMES = ['面馆', '烧腊', '超市', '五金', '书店', '电子', '面包', '药房', '茶饮', '花店']
SPILL = []  # light on the sidewalk in front of each open shop: (x, z, half-length along the wall, colour)
def wall_text(xw, yc, zc, chars, hc, fill):
    # characters painted on the wall, each drawn in the wall's own perspective at its centre;
    # they read left to right: away from you on the left wall, toward you on the right
    along = 1 if xw < 0 else -1
    for i, ch in enumerate(chars):
        z = zc + along * (i - (len(chars) - 1) / 2) * hc * 1.25
        cx, cy = P(xw, yc, z)
        k = hc / 100
        ux, uy = along * -F * (xw - CAM_X) / z ** 2 * k, along * F * (yc - EYE) / z ** 2 * k
        out.append(f'<text transform="matrix({ux:.4f} {uy:.4f} 0 {F / z * k:.4f} {cx:.1f} {cy:.1f})" font-size="100" '
                   f'fill="{fill}" text-anchor="middle" dominant-baseline="central" {FONT}>{ch}</text>')
def shopfront(xw, a, b, shop, name, door_near):
    near = F * abs(xw - CAM_X) * (b - a) / a ** 2 >= 12  # wide enough on screen for the details
    fw = sw(a, 0.04, 0.5, 1.6)
    frame = ALUMINIUM if shop else GREY_D
    fill = f'url(#shop{SHOPS.index(shop)})' if shop else 'url(#shut)'
    da, db = (a, a + 1.1) if door_near else (b - 1.1, b)  # the door
    wa, wb = (db + 0.15, b) if door_near else (a, da - 0.15)  # the display window
    window = [(xw, 0.45, wa), (xw, 2.9, wa), (xw, 2.9, wb), (xw, 0.45, wb)]
    door = [(xw, 0.0, da), (xw, 2.9, da), (xw, 2.9, db), (xw, 0.0, db)]
    for q in (window, door):
        poly([P(*v) for v in q], fill=fill, stroke=frame, w=fw)
        if shop:
            reflect(q, shop[1], 0.3, stretch=1.6)
    if near:
        line(P(xw, 2.4, da), P(xw, 2.4, db), frame, fw)       # the door's transom
        line(P(xw, 1.05, da + 0.12), P(xw, 1.05, db - 0.12), frame, fw)  # its push bar
        panes = max(1, round((wb - wa) / 1.2))
        for k in range(1, panes):
            zz = wa + (wb - wa) * k / panes
            line(P(xw, 0.45, zz), P(xw, 2.9, zz), frame, sw(zz, 0.03, 0.5, 1.2))
    band = [P(xw, 3.1, a), P(xw, 3.7, a), P(xw, 3.7, b), P(xw, 3.1, b)]
    poly(band, fill=GREY_DD)  # the sign band
    if F * 0.6 / a >= 8 and not (not shop and behind_headline(band)):  # tall enough to letter
        wall_text(xw, 3.4, (a + b) / 2, name, 0.4, BOARD_GOLD if shop else '#3e3f41')
    if shop:
        line(P(xw, 3.08, a), P(xw, 3.08, b), shop[1], max(0.6, F * 0.04 / a), extra='opacity="0.8"')
        SPILL.append((xw - 1.25 if xw > 0 else xw + 1.25, (a + b) / 2, (b - a) / 2, shop[1]))

right = [(140, 240, 16), (100, 140, 13), (74, 100, 15), (52, 74, 12), (32, 52, 17), (12, 32, 14)]
left = [(135, 240, 14), (95, 135, 15), (66, 95, 12), (44, 66, 18), (24, 44, 13), (1.5, 24, 16)]
right_brick = ['brown', 'clinker', 'buff', 'red', 'grey', 'orange']  # far to near, no two neighbours alike
left_brick = ['grey', 'brown', 'orange', 'clinker', 'buff', 'red']
for (z0, z1, h), brick in zip(right, right_brick): facade(RIGHT_W, z0, z1, h, brick, lit_shop=(z0 == 32))
for (z0, z1, h), brick in zip(left, left_brick): facade(LEFT_W, z0, z1, h, brick)

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
    x, z, w, h, shape = min(towers, key=lambda t: abs(t[0] - m[1][0]) + t[1] / 100)
    xr, yr = roof(x, w, h, shape)
    arc(m, (P(xr, yr, z), (xr, yr, z)), lift=40.0)

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
spills = {}
for x, z, half, light in SPILL:
    if light not in spills:
        spills[light] = f'spill{len(spills)}'
        defs.append(f'<radialGradient id="{spills[light]}"><stop offset="0" stop-color="{light}" stop-opacity="0.3"/>'
                    f'<stop offset="1" stop-color="{light}" stop-opacity="0"/></radialGradient>')
    poly([P(x + 1.2 * math.cos(t), 0, z + half * 1.3 * math.sin(t)) for t in [i * math.pi / 12 for i in range(24)]],
         fill=f'url(#{spills[light]})')

REFL_AT = len(out)  # the reflection layer goes here, on the ground

# ---------------- A parking sign at the kerb ----------------
# A blue "P" on a pole, standing between the tents and the road.
PZ_SIGN, PX_SIGN = 8.0, CURB_L - 0.4
def parking_sign():
    line(P(PX_SIGN, 0, PZ_SIGN), P(PX_SIGN, 2.9, PZ_SIGN), GREY_D, sw(PZ_SIGN, 0.06, 0.8, 2.4))
    sq = [(PX_SIGN - 0.24, 2.35, PZ_SIGN), (PX_SIGN - 0.24, 2.85, PZ_SIGN), (PX_SIGN + 0.24, 2.85, PZ_SIGN), (PX_SIGN + 0.24, 2.35, PZ_SIGN)]
    poly([P(*q) for q in sq], fill='#2e64c2', stroke='#d8d8dc', w=sw(PZ_SIGN, 0.02, 0.5, 1.0))
    text(P(PX_SIGN, 2.6, PZ_SIGN), 'P', F * 0.36 / PZ_SIGN, '#f2f2f4')

# ---------------- Gate ----------------
GZ = 82
gc, lacquer, tiles = '#c23a2a', '#4a1611', '#241816'  # red lacquer lit at its edges, in shadow between
def gate():
    for xp in (-9.2, 8.6):
        poly([P(xp - 0.4, 0, GZ), P(xp - 0.4, 8.2, GZ), P(xp + 0.4, 8.2, GZ), P(xp + 0.4, 0, GZ)], fill=lacquer, stroke=gc, w=1.6)
    poly([P(-10.0, 6.4, GZ), P(-10.0, 7.0, GZ), P(9.4, 7.0, GZ), P(9.4, 6.4, GZ)], fill=lacquer, stroke=gc, w=1.4)
    poly([P(-10.4, 8.2, GZ), P(-10.4, 8.8, GZ), P(9.8, 8.8, GZ), P(9.8, 8.2, GZ)], fill=lacquer, stroke=gc, w=1.4)
    poly([P(-12.0, 9.4, GZ), P(-11.2, 8.8, GZ), P(10.6, 8.8, GZ), P(11.4, 9.4, GZ), P(8.6, 10.8, GZ), P(-9.2, 10.8, GZ)], fill=tiles, stroke=gc, w=1.6)
    pad(P(-12.0, 9.4, GZ), 3.0, gc); pad(P(11.4, 9.4, GZ), 3.0, gc); pad(P(-0.3, 10.8, GZ), 3.0, gc)
    poly([P(-1.8, 7.2, GZ), P(-1.8, 8.0, GZ), P(1.2, 8.0, GZ), P(1.2, 7.2, GZ)], fill=lacquer, stroke=gc, w=1.2)

# ---------------- Streetlights (right curb) ----------------
# Modern LED streetlights: a grey pole and an arm over the road, a slim head, cool light from an
# LED strip on its underside, a faint cone to the ground and a pool of light where it lands.
defs.append(f'<linearGradient id="cone" x1="0" y1="0" x2="0" y2="1">'
            f'<stop offset="0" stop-color="{COOL}" stop-opacity="0.13"/>'
            f'<stop offset="1" stop-color="{COOL}" stop-opacity="0"/></linearGradient>')
LAMPS = (22, 42, 64, 96, 150)
def streetlight(zl):
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
# Most are backlit red boards with pale-gold characters and trim, as on Yaowarat Road; the
# electronics repair shop is neon.
def sign(xs0, xs1, y0, y1, z, chars, color=None, refl=0.18):
    q = [(xs0, y0, z), (xs0, y1, z), (xs1, y1, z), (xs1, y0, z)]
    reflect(q, color or BOARD_RED, refl, stretch=1.4)
    if color is None:  # a board
        poly([P(*p) for p in q], fill=BOARD_RED, stroke=BOARD_GOLD, w=sw(z, 0.08, 0.8, 2.2))
        ink = BOARD_GOLD
    else:              # neon
        poly([P(*p) for p in q], fill='#160e14', stroke=color, w=sw(z, 0.08, 0.8, 2.2))
        ink = color
    n = len(chars); step = (y1 - y0) / n
    size = F * min(xs1 - xs0, step) * 0.64 / z
    for i, ch in enumerate(chars):
        text(P((xs0 + xs1) / 2, y1 - step * (i + 0.5), z), ch, size, ink)
SIGNS = [(-10.9, -9.5, 4.6, 9.8, 26, '茶楼'), (8.6, 9.9, 4.2, 10.2, 36, '电器维修', NEON_MAGENTA, 0.35),
         (-10.9, -9.8, 4.6, 9.0, 54, '饭店'), (8.7, 9.9, 4.2, 8.4, 60, '腾振', None, 0.25),
         (8.8, 9.9, 4.4, 8.2, 92, '药材')]

# ---------------- Lanterns ----------------
# Red paper lanterns on wires across the street, hung as in Chinatown: round, lit from inside,
# ribbed, capped top and bottom, a soft bloom around each, and a red streak below on the wet
# street. The ones behind the headline hang unlit, for contrast.
LANTERN_RED, LANTERN_DARK = '#e0452b', '#3a1a12'
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
WIRES = (16, 21, 27, 34, 43, 55, 70, 90, 118)
def wire_y(x):  # a wire sags a metre between the walls
    mid, half = (LEFT_W + RIGHT_W) / 2, (RIGHT_W - LEFT_W) / 2
    return 8.4 - 1.0 * (1 - ((x - mid) / half) ** 2)
WIRE_XS = [LEFT_W + i * (RIGHT_W - LEFT_W) / 30 for i in range(31)]  # 31 points along a wire, wall to wall
LANTERN_XS = WIRE_XS[1:30:2]  # a lantern on every other one
def lantern_wire(zl):
    pline([P(xx, wire_y(xx), zl) for xx in WIRE_XS], '#3a2a24', sw(zl, 0.025, 0.5, 1.0))
    for xx in LANTERN_XS:
        yy = wire_y(xx)
        line(P(xx, yy, zl), P(xx, yy - 0.25, zl), '#5a3a1c', sw(zl, 0.02, 0.4, 0.9))
        lantern(xx, yy - 0.25 - 0.21, zl)

# ---------------- The street, far to near ----------------
# Gate, streetlights, signs, lantern wires and the parking sign are drawn from the far end toward
# you, so whatever is nearer covers what is behind it.
def street():
    items = [(GZ, gate), (PZ_SIGN, parking_sign)]
    items += [(zl, lambda zl=zl: streetlight(zl)) for zl in LAMPS]
    items += [(sg[4], lambda sg=sg: sign(*sg)) for sg in SIGNS]
    items += [(zl, lambda zl=zl: lantern_wire(zl)) for zl in WIRES]
    for _, draw in sorted(items, key=lambda it: -it[0]):
        draw()
street()

# ---------------- The network on the ground ----------------
# Every tent is on copper, soldered in: its trace runs out from under the door, jogs 45 degrees and
# tees into your path, which runs from your feet to the tallest tower's door. Radio, and antennas,
# belong to the buildings above.
# On the ground the copper is inlaid: flat strips set flush in the concrete, each in a
# thin dark joint. Polished metal is lit by what it mirrors: far off, at a grazing angle, the lit
# street; at your feet, the dark sky overhead.
TENT_LIT = 5.6
# Dome tents as they stand on Portland sidewalks: two-tone rainflies, some under a blue tarp.
# (front z, width, height, body colour, fly colour, tarp). Colours are dimmed for night.
NAVY, GREEN, RED_FLY, GREY_FLY, TARP = '#1c2438', '#3d4d2b', '#662824', '#4c5157', '#27365e'
TENTS = [(TENT_LIT, 2.3, 1.3, NAVY, NAVY, True), (10.4, 2.1, 1.15, GREEN, GREY_FLY, False),
         (13.4, 2.3, 1.25, RED_FLY, GREY_FLY, False), (19.6, 1.9, 1.05, NAVY, GREY_FLY, False),
         (22.2, 2.2, 1.2, NAVY, NAVY, True), (30.5, 2.0, 1.1, GREEN, GREY_FLY, False),
         (38.0, 2.3, 1.25, RED_FLY, GREY_FLY, False)]
assert TENT_STRETCH[0] <= min(t[0] for t in TENTS) and max(t[0] for t in TENTS) + 2.2 <= TENT_STRETCH[1]
MAIN = [(CAM_X, 2.4), (CAM_X, 640.0)]
def ribbon(a, b, wd):
    (x0, z0), (x1, z1) = a, b
    dx, dz = x1 - x0, z1 - z0; L = math.hypot(dx, dz); nx, nz = -dz / L * wd, dx / L * wd
    return [P(x0 + nx, 0, z0 + nz), P(x1 + nx, 0, z1 + nz), P(x1 - nx, 0, z1 - nz), P(x0 - nx, 0, z0 - nz)]
INLAY, INLAY_JOINT = 'url(#inlay)', '#0f1011'
defs.append(f'<linearGradient id="inlay" gradientUnits="userSpaceOnUse" x1="0" y1="{VPY}" x2="0" y2="{H}">'
            f'<stop offset="0" stop-color="#f2c472"/><stop offset="0.25" stop-color="{COPPER}"/>'
            '<stop offset="1" stop-color="#8c5f2a"/></linearGradient>')
def strips(paths):
    # (points, half-width, hairline) for each strip. Every joint is laid before any metal, so where
    # copper meets copper it is one piece: a tent's trace tees into your path with no seam. Corners
    # are round; the hairline keeps a far strip visible.
    for fill, extra in ((INLAY_JOINT, 0.012), (INLAY, 0.0)):
        for points, wd, w in paths:
            for a, b in zip(points, points[1:]):
                poly(ribbon(a, b, wd + extra), fill=fill, stroke=fill if fill == INLAY else 'none', w=w)
            for x, z in points[1:-1]:
                ground_pad(x, z, wd + extra, fill, drill=False)

# every tent: soldered in under its door, a 45-degree jog, and a tee into your path
strips([(MAIN, 0.075, 0.8)] + [([(TENT_WALL + wd / 2, z0 + 0.3), (TENT_WALL + wd / 2, z0),
                                 (TENT_WALL + wd / 2 + 0.6, z0 - 0.6), (CAM_X, z0 - 0.6)], 0.06, 0.6)
                               for z0, wd, *_ in TENTS])
# The lanterns hanging over your path glint in it, one on each wire, receding toward the gate.
defs.append('<radialGradient id="glint"><stop offset="0" stop-color="#ffe0a8" stop-opacity="0.95"/>'
            '<stop offset="0.45" stop-color="#ff8a3a" stop-opacity="0.55"/>'
            '<stop offset="1" stop-color="#ff8a3a" stop-opacity="0"/></radialGradient>')
defs.append(f'<clipPath id="main"><polygon points="{pts(ribbon(MAIN[0], MAIN[1], 0.075))}"/></clipPath>')
out.append('<g clip-path="url(#main)">')
for zl in WIRES:
    xl = min(LANTERN_XS, key=lambda x: abs(x - CAM_X))
    yl = wire_y(xl) - 0.46
    ellipse(P(xl, -yl, zl), F * 0.3 / zl, F * 0.3 / zl * 1.4, 'url(#glint)')
out.append('</g>')

# ---------------- Tents along the left wall ----------------
# Nylon lit from inside glows in its own colour, like a lantern: blue through a blue tent, red
# through a red fly, brightest near the lamp. Outside, the streetlights across the road catch each
# dome's upper right; the side against the wall and the base fall dark, and each sits in its own
# contact shadow. Seams and pole sleeves are folds in the fabric, not outlines. Through each open
# door, the warm inside; through the nearest, wide open, a home someone keeps.
defs.append('<radialGradient id="pool"><stop offset="0" stop-color="#ffb35a" stop-opacity="0.3"/>'
            '<stop offset="0.5" stop-color="#ff9a4a" stop-opacity="0.12"/>'
            '<stop offset="1" stop-color="#ff9a4a" stop-opacity="0"/></radialGradient>')
defs.append('<radialGradient id="tentlight" cx="0.45" cy="0.8" r="0.8">'
            '<stop offset="0" stop-color="#f2c98a"/><stop offset="0.45" stop-color="#b8703a"/>'
            '<stop offset="1" stop-color="#4a2616"/></radialGradient>')
defs.append('<filter id="contact" x="-20%" y="-50%" width="140%" height="200%"><feGaussianBlur stdDeviation="3"/></filter>')
defs.append('<linearGradient id="flyshade" x1="1" y1="0" x2="0.15" y2="1">'
            '<stop offset="0.35" stop-color="#000000" stop-opacity="0"/>'
            '<stop offset="1" stop-color="#000000" stop-opacity="0.5"/></linearGradient>')
defs.append(f'<radialGradient id="domekey" cx="0.7" cy="0.2" r="0.5"><stop offset="0" stop-color="{COOL}" stop-opacity="0.16"/>'
            f'<stop offset="1" stop-color="{COOL}" stop-opacity="0"/></radialGradient>')
defs.append('<radialGradient id="lampbloom"><stop offset="0" stop-color="#fff0c8" stop-opacity="0.8"/>'
            '<stop offset="0.3" stop-color="#ffc070" stop-opacity="0.35"/>'
            '<stop offset="1" stop-color="#ffc070" stop-opacity="0"/></radialGradient>')
FABRIC_LIT = {NAVY: '#4460b0', GREEN: '#76923f', RED_FLY: '#b04c38', GREY_FLY: '#9a9ea3', TARP: '#4d6ccc'}
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
def box(x0, x1, y0, y1, z0, z1, front, top, side):
    # a box on the tent floor, seen from the right and above: its front, its top, its right end
    poly([P(x0, y0, z0), P(x0, y1, z0), P(x1, y1, z0), P(x1, y0, z0)], fill=front)
    poly([P(x0, y1, z0), P(x0, y1, z1), P(x1, y1, z1), P(x1, y1, z0)], fill=top)
    poly([P(x1, y0, z0), P(x1, y1, z0), P(x1, y1, z1), P(x1, y0, z1)], fill=side)
def home(xm, zc, wd, ln, ht, lamp_at):
    # The nearest tent, kept as a home: a rug, a made bed with a pillow and a blanket, books, a crate
    # for a desk with a laptop open and a small board being worked on, a lamp, and fairy lights
    # strung along the back wall.
    lx, ly, lz = lamp_at
    floor = [P(xm + wd / 2 * 0.97 * math.cos(t), 0.01, zc + ln / 2 * 0.97 * math.sin(t))
             for t in [i * math.pi / 18 for i in range(36)]]
    poly(floor, fill='#4a3224')
    poly([P(-10.45, 0.015, 5.95), P(-10.45, 0.015, 6.6), P(-9.3, 0.015, 6.6), P(-9.3, 0.015, 5.95)], fill='#2e4d48')
    poly([P(-10.37, 0.016, 6.02), P(-10.37, 0.016, 6.53), P(-9.38, 0.016, 6.53), P(-9.38, 0.016, 6.02)],
         stroke='#b08a52', w=sw(6.2, 0.012, 0.5, 1.2))  # the rug's border
    # fairy lights along the back wall, sagging between the poles
    bulbs = []
    for i in range(13):
        t = i / 12
        x = -10.6 + 1.7 * t
        y = 0.64 - 0.1 * math.sin(math.pi * t)
        r = 1 - ((x - xm) / (wd / 2)) ** 2 - (y / ht) ** 2
        bulbs.append(P(x, y, zc + ln / 2 * 0.95 * math.sqrt(max(0.0, r))))
    pline(bulbs, '#3a2a1e', 0.6)
    for c in bulbs:
        circle(c, 4.0, 'url(#lampbloom)'); circle(c, 1.1, '#ffe6b0')
    # the bed: a foam mattress under a cream sheet, a pillow, a teal blanket turned down
    box(-10.62, -8.9, 0.0, 0.2, 6.62, 7.25, '#8c7460', '#d6bf9e', '#a88c70')
    box(-10.58, -10.18, 0.2, 0.3, 6.72, 7.16, '#cfc2a8', '#ece2cc', '#dcd0b6')  # the pillow, plumped
    poly([P(-10.38 + 0.2 * math.cos(t), 0.31, 6.94 + 0.22 * math.sin(t)) for t in [i * math.pi / 10 for i in range(20)]],
         fill='#f2eadb')
    poly([P(-10.1, 0.205, 6.6), P(-10.1, 0.205, 7.26), P(-8.88, 0.205, 7.26), P(-8.88, 0.205, 6.6)], fill='#2f5f6c')
    poly([P(-10.1, 0.207, 6.6), P(-10.1, 0.207, 7.26), P(-9.96, 0.207, 7.26), P(-9.96, 0.207, 6.6)],
         fill='#d8c8a8')  # the sheet, turned down over the blanket
    drape = [P(-10.1, 0.205, 6.6)] + [P(-10.1 + 1.22 * k / 8, 0.07 + 0.025 * math.sin(k * 1.7), 6.59) for k in range(9)] \
            + [P(-8.88, 0.205, 6.6)]
    poly(drape, fill='#244c57')
    # a stack of books on the rug, a small board with a lit LED on top
    for k, colour in enumerate(('#6a2c26', '#2a4a6a', '#b89a62')):
        box(-9.98, -9.72, 0.05 * k, 0.05 * (k + 1), 6.36, 6.54, colour, mix(colour, '#ffffff', 0.25), mix(colour, '#000000', 0.2))
    poly([P(-9.94, 0.152, 6.39), P(-9.94, 0.152, 6.5), P(-9.78, 0.152, 6.5), P(-9.78, 0.152, 6.39)], fill='#2f6a44')
    circle(P(-9.82, 0.16, 6.43), 1.0, COOL)
    # a crate for a desk, the laptop open on it, the lamp beside
    box(-10.62, -10.2, 0.0, 0.34, 5.9, 6.3, '#6e5238', '#9a7a56', '#83654a')
    for k in (1, 2):  # the crate's slats
        yy = 0.34 * k / 3
        line(P(-10.62, yy, 5.9), P(-10.2, yy, 5.9), '#4a3424', sw(5.9, 0.012, 0.5, 1.2))
    poly([P(-10.56, 0.345, 5.97), P(-10.56, 0.345, 6.16), P(-10.34, 0.345, 6.16), P(-10.34, 0.345, 5.97)], fill='#2a2c30')
    screen = [P(-10.56, 0.345, 6.16), P(-10.56, 0.52, 6.22), P(-10.34, 0.52, 6.22), P(-10.34, 0.345, 6.16)]
    sc = P(-10.45, 0.44, 6.19)
    circle(sc, F * 0.25 / 6.2, 'url(#screenbloom)')
    poly(screen, fill='#a8dde2', stroke='#2a2c30', w=sw(6.2, 0.012, 0.5, 1.2))
    c = P(lx, ly, lz)
    circle(c, F * 0.35 / lz, 'url(#lampbloom)')
    # a camping lantern: dark base and cap, the lit globe between
    poly([P(lx - 0.045, 0.34, lz), P(lx - 0.045, 0.37, lz), P(lx + 0.045, 0.37, lz), P(lx + 0.045, 0.34, lz)], fill='#2a2c30')
    poly([P(lx - 0.04, 0.37, lz), P(lx - 0.04, 0.46, lz), P(lx + 0.04, 0.46, lz), P(lx + 0.04, 0.37, lz)], fill='#fff4d8')
    poly([P(lx - 0.045, 0.46, lz), P(lx - 0.03, 0.49, lz), P(lx + 0.03, 0.49, lz), P(lx + 0.045, 0.46, lz)], fill='#2a2c30')
defs.append(f'<radialGradient id="screenbloom"><stop offset="0" stop-color="{COOL}" stop-opacity="0.5"/>'
            f'<stop offset="1" stop-color="{COOL}" stop-opacity="0"/></radialGradient>')
def tent(z0, wd, ht, body, fly, tarp):
    # a half-ellipsoid dome: base ellipse wd x ln, height ht; its front touches z0
    xm, ln = TENT_WALL + wd / 2, 2.2
    zc = z0 + ln / 2
    is_home = z0 == TENT_LIT
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
    tid = f'tent{int(z0 * 10)}'
    ellipse(P(xm, 0, z0 - 0.7), F * 1.25 / z0, F * EYE * 1.25 / (z0 - 0.7) ** 2, 'url(#pool)')
    base = [P(xm + wd / 2 * 1.05 * math.cos(t), 0, zc + ln / 2 * 1.05 * math.sin(t)) for t in [i * math.pi / 18 for i in range(36)]]
    poly(base, fill='#000000', extra='opacity="0.5" filter="url(#contact)"')
    outline = hull([P(*surf(math.radians(a), math.radians(b))) for a in range(0, 360, 10) for b in range(0, 91, 10)])
    cap = hull([P(*surf(math.radians(a), math.radians(b))) for a in range(0, 360, 10) for b in range(38, 91, 8)])
    regions = [(outline, body), (cap, fly)]
    if tarp:  # a blue tarp thrown over the top and down one side, its hem uneven
        rnd = random.Random(int(z0 * 10))
        drape = [P(*surf(math.radians(a), math.radians(b))) for a in range(-60, 181, 10) for b in range(20, 91, 10)]
        hem = [P(*surf(math.radians(a), math.radians(12 + 14 * rnd.random()))) for a in range(-60, 181, 15)]
        regions.append((hull(drape + hem), TARP))
    # each fabric, then the lamp's light coming through it in that fabric's colour
    lamp_at = (-10.27, 0.41, 6.1) if is_home else (xm - 0.25, 0.35, zc - 0.2)
    lc, lr = P(*lamp_at), F * wd * 0.7 / z0
    for k, (region, fabric) in enumerate(regions):
        gid = f'{tid}g{k}'
        defs.append(f'<radialGradient id="{gid}" gradientUnits="userSpaceOnUse" cx="{lc[0]:.1f}" cy="{lc[1]:.1f}" r="{lr:.1f}">'
                    f'<stop offset="0" stop-color="{FABRIC_LIT[fabric]}" stop-opacity="0.85"/>'
                    f'<stop offset="0.55" stop-color="{FABRIC_LIT[fabric]}" stop-opacity="0.3"/>'
                    f'<stop offset="1" stop-color="{FABRIC_LIT[fabric]}" stop-opacity="0"/></radialGradient>')
        poly(region, fill=fabric)
        poly(region, fill=f'url(#{gid})')
    # outside light: the streetlights on the upper right, shadow toward the wall and the ground
    poly(outline, fill='url(#flyshade)')
    poly(outline, fill='url(#domekey)')
    # the fly's seam and the two pole sleeves, as folds: a shadow with a lit edge beside it
    seam = [surf(math.radians(a), math.radians(38)) for a in range(0, 361, 6)]
    for run in visible_runs(seam):
        pline(run, mix(fly, '#000000', 0.45), max(0.5, w * 0.45), extra='opacity="0.6"')
    for a in (45, 135):
        arc_ = [surf(math.radians(a + (180 if b < 0 else 0)), math.radians(abs(b))) for b in range(-90, 91, 4)]
        for run in visible_runs(arc_):
            pline(run, mix(fly, '#ffffff', 0.3), max(0.5, w * 0.5), extra='opacity="0.35"')
    if tarp:
        for a in (-20, 40, 100):
            crease = [surf(math.radians(a + 6 * math.sin(b)), math.radians(b)) for b in range(18, 88, 6)]
            for run in visible_runs(crease):
                pline(run, mix(TARP, '#ffffff', 0.35), max(0.5, w * 0.4), extra='opacity="0.3"')
    # the door: a D-shaped zip opening, its panel rolled to one side; the nearest wide open
    dw, hd = (0.3 * wd, 0.72 * ht) if is_home else (0.22 * wd, 0.62 * ht)
    d_world = [on_front(xm - dw, 0.0)]
    d_world += [on_front(xm - dw, hd * t) for t in (0.3, 0.6, 0.85)]
    d_world += [on_front(xm + dw * math.cos(math.radians(a)) * -1, hd * 0.85 + hd * 0.15 * math.sin(math.radians(a)))
                for a in range(0, 181, 20)]
    d_world += [on_front(xm + dw, hd * t) for t in (0.85, 0.6, 0.3)] + [on_front(xm + dw, 0.0)]
    door = [P(*p) for p in d_world]
    defs.append(f'<clipPath id="{tid}d"><polygon points="{pts(door)}"/></clipPath>')
    out.append(f'<g clip-path="url(#{tid}d)">')
    if is_home:
        defs.append(f'<radialGradient id="{tid}in" gradientUnits="userSpaceOnUse" cx="{lc[0]:.1f}" cy="{lc[1]:.1f}" r="{F * 1.3 / z0:.1f}">'
                    '<stop offset="0" stop-color="#f0c68a"/><stop offset="0.4" stop-color="#b87840"/>'
                    '<stop offset="1" stop-color="#4a2a1a"/></radialGradient>')
        poly(door, fill=f'url(#{tid}in)')
        home(xm, zc, wd, ln, ht, lamp_at)
    else:
        poly(door, fill='url(#tentlight)')
        bag = [P(xm - 0.1 + 0.55 * math.cos(t), 0.1 + 0.1 * math.sin(t), zc + 0.2) for t in [i * math.pi / 8 for i in range(16)]]
        poly(bag, fill='#2a1c16', extra='opacity="0.8"')  # a sleeping bag
    out.append('</g>')
    pline(door, mix(body, '#000000', 0.5), max(0.5, w * 0.4), extra='opacity="0.7"')  # the zip
    roll = [P(*on_front(xm - dw - 0.1, hd * t)) for t in (0.02, 0.5, 0.92)]
    pline(roll, mix(body, '#000000', 0.3), max(1.4, w * 2.2))
    pline(roll, FABRIC_LIT[body], max(0.6, w * 0.8), extra='opacity="0.5"')
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

def write(name):
    svg = (f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}">'
           f'<defs>{"".join(defs)}</defs>' + ''.join(out) + '</svg>')
    Path(__file__).with_name(name).write_text(svg, encoding='utf-8')
write('mural-street-night.svg')

# ================ The arrival: the same skyline at daybreak ================
# The foot of the homepage, where the copper rail that runs down the page reaches the tallest
# tower's door. The same towers, drawn by the same code from a new camera on a fresh canvas: far
# off and level with their feet, as the sky pales from night to first light behind them, the sun
# still below the horizon behind the tallest. Its glass lobby is lit, a pool of that light on the
# plaza. The page draws the rail over this, soldered in at the door at (1180, 520), no via. The
# scene has no background of its own: the page's shows through above the sky and at the sides.
defs, out = [], []
W, H = 1440, 560
CAM_X, EYE, F, VPX, VPY = 0.0, 1.6, 700.0, 720.0, 518.0
DOOR = 1180
def at(sx, z): return (sx - VPX) * z / F + CAM_X  # the world x that lands at screen x sx, depth z
defs.append(f'<linearGradient id="dawn" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="0" y2="{VPY + 2}">'
            f'<stop offset="0" stop-color="{BG}" stop-opacity="0"/><stop offset="0.45" stop-color="#151827"/>'
            '<stop offset="0.75" stop-color="#2a2431"/><stop offset="0.92" stop-color="#5a3b35"/>'
            '<stop offset="1" stop-color="#9a5c3c"/></linearGradient>')
out.append(f'<rect width="{W}" height="{VPY + 2}" fill="url(#dawn)"/>')
defs.append(f'<radialGradient id="sunrise" gradientUnits="userSpaceOnUse" cx="{DOOR}" cy="{VPY + 2}" r="460">'
            '<stop offset="0" stop-color="#ffb070" stop-opacity="0.45"/><stop offset="0.4" stop-color="#ff9a5a" stop-opacity="0.12"/>'
            '<stop offset="1" stop-color="#ff9a5a" stop-opacity="0"/></radialGradient>')
out.append(f'<rect width="{W}" height="{VPY + 2}" fill="url(#sunrise)"/>')
SKY_GLOW = '#6a4438'  # the haze between layers, warm with first light
glass_gradients((('glass', '#101216', '#2a2228'), ('glassside', '#16181d', '#3a2a2c'), ('glassfar', '#1c1b24', '#3e2e30')))
towers = [(at(sx, z), z, w, h, shape) for sx, z, w, h, shape in (
    (270, 700, 50, 230, 'setback'), (390, 640, 36, 280, 'slab'), (515, 620, 26, 320, 'twin'),
    (585, 620, 26, 320, 'twin'), (700, 600, 32, 340, 'bamboo'), (805, 740, 46, 300, 'slab'),
    (935, 680, 44, 360, 'cup'), (DOOR, 600, 44, 385, 'crown'), (1330, 640, 36, 260, 'setback'),
    (1425, 720, 40, 220, 'slab'))]
towers.sort(key=lambda t: -t[1])
draw_towers(far_x=1150, far_n=34)
# the tallest tower's lobby: double-height glass, lit, its doors where the rail comes in
xc = at(DOOR, 600)
defs.append('<linearGradient id="lobby" x1="0" y1="0" x2="0" y2="1">'
            '<stop offset="0" stop-color="#f6d49a"/><stop offset="0.6" stop-color="#d89a58"/>'
            '<stop offset="1" stop-color="#8a5a34"/></linearGradient>')
defs.append(f'<radialGradient id="plaza" gradientUnits="userSpaceOnUse" cx="{DOOR}" cy="{VPY + 3}" r="90">'
            '<stop offset="0" stop-color="#ffc98a" stop-opacity="0.45"/>'
            '<stop offset="1" stop-color="#ffc98a" stop-opacity="0"/></radialGradient>')
ellipse((DOOR, VPY + 4), 90, 14, 'url(#plaza)')
poly([P(xc - 16, 0, 600), P(xc - 16, 34, 600), P(xc + 16, 34, 600), P(xc + 16, 0, 600)], fill='url(#lobby)',
     stroke=ALUMINIUM, w=1.2)
for k in range(1, 8):
    xx = xc - 16 + 32 * k / 8
    line(P(xx, 0, 600), P(xx, 34, 600), ALUMINIUM, 0.8, extra='opacity="0.7"')
line(P(xc - 16, 17, 600), P(xc + 16, 17, 600), ALUMINIUM, 0.8, extra='opacity="0.7"')
poly([P(xc - 19, 34, 600), P(xc - 19, 37, 600), P(xc + 19, 37, 600), P(xc + 19, 34, 600)], fill=GREY_DD,
     stroke=METAL, w=0.8)  # the canopy
# The page is wider than this scene on a wide screen: its sides fade into the page, not stop.
defs.append('<linearGradient id="sides"><stop offset="0" stop-color="black"/><stop offset="0.1" stop-color="white"/>'
            '<stop offset="0.9" stop-color="white"/><stop offset="1" stop-color="black"/></linearGradient>')
defs.append(f'<mask id="edges" maskUnits="userSpaceOnUse" x="0" y="0" width="{W}" height="{H}">'
            f'<rect width="{W}" height="{H}" fill="url(#sides)"/></mask>')
out = ['<g mask="url(#edges)">'] + out + ['</g>']
write('skyline-daybreak.svg')
