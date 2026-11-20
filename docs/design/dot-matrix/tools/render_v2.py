# Renders the approved dot-matrix mockups (04, 06, 07, 08) into ../mockups/.
# Run: python docs/design/dot-matrix/tools/render_v2.py
# The app's real renderer is src/dotmatrix.rs (pinned by tests); this script is
# the design-time sketch it was ported from. Change the app there, not here.

import zlib, struct, os, math

# ---------- palette ----------
BG     = (14, 15, 19)
OFF    = (38, 40, 46)
ORANGE = (255, 179, 71)   # #FFB347 (user-specified)
WHITE  = (232, 232, 236)
GREY   = (136, 136, 136)
DIML   = (58, 61, 69)     # separators
SOFT   = (96, 99, 108)    # dim text

def blend(a, b, t):
    return (int(a[0]+(b[0]-a[0])*t), int(a[1]+(b[1]-a[1])*t), int(a[2]+(b[2]-a[2])*t))

def write_png(path, W, H, img):
    def chunk(t, d):
        c = t + d
        return struct.pack(">I", len(d)) + c + struct.pack(">I", zlib.crc32(c) & 0xFFFFFFFF)
    raw = b"".join(b"\x00" + b"".join(bytes(p) for p in img[y]) for y in range(H))
    with open(path, "wb") as f:
        f.write(b"\x89PNG\r\n\x1a\n")
        f.write(chunk(b"IHDR", struct.pack(">IIBBBBB", W, H, 8, 2, 0, 0, 0)))
        f.write(chunk(b"IDAT", zlib.compress(raw, 9)))
        f.write(chunk(b"IEND", b""))

# ---------- "real dot": superellipse (squircle), exponent 4 ----------
def make_lut(pitch, ratio=0.78, n=4.0, ss=16):
    a = pitch * ratio / 2.0
    lut = [[0.0]*pitch for _ in range(pitch)]
    for oy in range(pitch):
        for ox in range(pitch):
            c = 0
            for sy in range(ss):
                dyv = abs(oy + (sy+0.5)/ss - pitch/2.0)/a
                dyv = dyv**n
                for sx in range(ss):
                    dxv = abs(ox + (sx+0.5)/ss - pitch/2.0)/a
                    if dxv**n + dyv <= 1.0:
                        c += 1
            lut[oy][ox] = c/(ss*ss)
    return lut

def blank(W, H):
    return [[BG]*W for _ in range(H)]

def stamp_grid(img, W, H, pitch, cells, ox=0, oy=0, ratio=0.78):
    lut = make_lut(pitch, ratio)
    for (c, r), col in cells.items():
        x0, y0 = ox + c*pitch, oy + r*pitch
        if x0 >= W or y0 >= H: continue
        for oy2 in range(min(pitch, H-y0)):
            for ox2 in range(min(pitch, W-x0)):
                cov = lut[oy2][ox2]
                if cov > 0:
                    img[y0+oy2][x0+ox2] = blend(BG, col, cov)

def stamp_dot(img, W, H, cx, cy, rad, color, n=4.0, ss=8):
    x0, x1 = int(cx-rad-1), int(cx+rad+2)
    y0, y1 = int(cy-rad-1), int(cy+rad+2)
    for y in range(max(0, y0), min(H, y1)):
        for x in range(max(0, x0), min(W, x1)):
            c = 0
            for sy in range(ss):
                dyv = abs(y + (sy+0.5)/ss - cy)/rad
                dyv = dyv**n
                for sx in range(ss):
                    dxv = abs(x + (sx+0.5)/ss - cx)/rad
                    if dxv**n + dyv <= 1.0:
                        c += 1
            cov = c/(ss*ss)
            if cov > 0:
                img[y][x] = blend(img[y][x], color, cov)

# ---------- 5x7 dot font ----------
FONT = {
'A': ('01110','10001','10001','11111','10001','10001','10001'),
'B': ('11110','10001','10001','11110','10001','10001','11110'),
'C': ('01111','10000','10000','10000','10000','10000','01111'),
'D': ('11100','10010','10001','10001','10001','10010','11100'),
'E': ('11111','10000','10000','11110','10000','10000','11111'),
'F': ('11111','10000','10000','11110','10000','10000','10000'),
'G': ('01111','10000','10000','10111','10001','10001','01110'),
'H': ('10001','10001','10001','11111','10001','10001','10001'),
'I': ('01110','00100','00100','00100','00100','00100','01110'),
'J': ('00111','00010','00010','00010','00010','10010','01100'),
'K': ('10001','10010','10100','11000','10100','10010','10001'),
'L': ('10000','10000','10000','10000','10000','10000','11111'),
'M': ('10001','11011','10101','10101','10001','10001','10001'),
'N': ('10001','11001','10101','10011','10001','10001','10001'),
'O': ('01110','10001','10001','10001','10001','10001','01110'),
'P': ('11110','10001','10001','11110','10000','10000','10000'),
'Q': ('01110','10001','10001','10001','10101','10010','01101'),
'R': ('11110','10001','10001','11110','10100','10010','10001'),
'S': ('01111','10000','10000','01110','00001','00001','11110'),
'T': ('11111','00100','00100','00100','00100','00100','00100'),
'U': ('10001','10001','10001','10001','10001','10001','01110'),
'V': ('10001','10001','10001','10001','10001','01010','00100'),
'W': ('10001','10001','10001','10101','10101','11011','10001'),
'X': ('10001','10001','01010','00100','01010','10001','10001'),
'Y': ('10001','10001','01010','00100','00100','00100','00100'),
'Z': ('11111','00001','00010','00100','01000','10000','11111'),
'0': ('01110','10001','10011','10101','11001','10001','01110'),
'1': ('00100','01100','00100','00100','00100','00100','01110'),
'2': ('01110','10001','00001','00110','01000','10000','11111'),
'3': ('11110','00001','00001','01110','00001','00001','11110'),
'4': ('00010','00110','01010','10010','11111','00010','00010'),
'5': ('11111','10000','11110','00001','00001','10001','01110'),
'6': ('01110','10000','10000','11110','10001','10001','01110'),
'7': ('11111','00001','00010','00100','01000','01000','01000'),
'8': ('01110','10001','10001','01110','10001','10001','01110'),
'9': ('01110','10001','10001','01111','00001','00001','01110'),
' ': ('00000','00000','00000','00000','00000','00000','00000'),
':': ('00000','00100','00000','00000','00000','00100','00000'),
'-': ('00000','00000','00000','01110','00000','00000','00000'),
'(': ('00010','00100','01000','01000','01000','00100','00010'),
')': ('01000','00100','00010','00010','00010','00100','01000'),
'.': ('00000','00000','00000','00000','00000','00000','00100'),
}

def put_text(cells, text, x0, y0, color):
    x = x0
    for ch in text:
        g = FONT.get(ch.upper(), FONT[' '])
        for r in range(7):
            bits = g[r]
            for c in range(5):
                if bits[c] == '1':
                    cells[(x+c, y0+r)] = color
        x += 6

# ---------- padlock glyph, 18x18 (shape approved in round 2, recoloured) ----------
def padlock_solid():
    lit = set()
    for r in (2, 3):
        for c in range(5, 13): lit.add((r, c))      # shackle top, 2 thick
    for r in range(4, 9):
        for c in (5, 6, 11, 12): lit.add((r, c))    # shackle sides
    for r in range(9, 17):
        for c in range(3, 15): lit.add((r, c))      # body
    for (r, c) in ((11,8),(11,9),(12,8),(12,9)): lit.discard((r, c))  # keyhole
    return lit

def padlock_hollow():
    lit = set()
    for c in range(5, 13): lit.add((2, c))          # shackle top outline
    for r in range(3, 9):
        lit.add((r, 5)); lit.add((r, 12))           # shackle sides, 1 thick
    for c in range(3, 15): lit.add((9, c)); lit.add((16, c))  # body top/bottom
    for r in range(10, 16):
        lit.add((r, 3)); lit.add((r, 14))           # body sides
    for (r, c) in ((11,8),(11,9),(12,8),(12,9)): lit.add((r, c))  # keyhole lit
    return lit

def glyph_panel(lit, color, pitch=15):
    cells = {}
    for r in range(18):
        for c in range(18):
            cells[(c, r)] = OFF
    for (r, c) in lit:
        cells[(c, r)] = color
    return cells

out = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "mockups")
os.makedirs(out, exist_ok=True)

# ---------- 04: padlock, locked / unlocked, #FFB347 ----------
M, GAP, P = 40, 50, 15
W, H = M + 270 + GAP + 270 + M, M + 270 + M
img = blank(W, H)
stamp_grid(img, W, H, P, glyph_panel(padlock_solid(), ORANGE), ox=M, oy=M)
stamp_grid(img, W, H, P, glyph_panel(padlock_solid(), WHITE), ox=M+270+GAP, oy=M)
p4 = os.path.join(out, "04-glyph-a-padlock.png")
write_png(p4, W, H, img)
print("wrote", p4, os.path.getsize(p4))

# ---------- 06: three states with dot captions ----------
SP = 60
W = M + 3*270 + 2*SP + M
H = M + 270 + 16 + 42 + M
img = blank(W, H)
states = [(padlock_solid(), ORANGE, "LOCKED"),
          (padlock_hollow(), ORANGE, "SUSPENDED"),
          (padlock_solid(), WHITE, "RELEASED")]
for i, (lit, col, cap) in enumerate(states):
    x0 = M + i*(270+SP)
    stamp_grid(img, W, H, P, glyph_panel(lit, col), ox=x0, oy=M)
    cw = (6*len(cap)-1)*6
    cx0 = (x0 + 135 - cw//2)//6
    cells = {}
    put_text(cells, cap, cx0, (M+270+16)//6, GREY)
    stamp_grid(img, W, H, 6, cells)
p6 = os.path.join(out, "06-padlock-states.png")
write_png(p6, W, H, img)
print("wrote", p6, os.path.getsize(p6))

# ---------- 07: full window mockup, 640x400, pitch 5 ----------
PW = 5
W, H = 640, 400
img = blank(W, H)
cells = {}
for r in range(H//PW):
    for c in range(W//PW):
        cells[(c, r)] = OFF
# Q12 fix: rows of the same section line up — 1 empty grid row between
# the 7-row-tall status rows (they overlapped in the first mockup).
put_text(cells, "MONITOR LOCK", 8, 7, WHITE)
for c in range(8, 121): cells[(c, 17)] = DIML
for (r, c) in padlock_solid(): cells[(8+c, 21+r)] = ORANGE
# Tweak (user): status rows get a 2-empty-row gap; buttons share the left
# edge (col 8) with MONITOR LOCK and DETAILS.
put_text(cells, "LOCK: ENGAGED", 34, 22, ORANGE)   # rows 22-28
put_text(cells, "MY GAME", 34, 31, WHITE)           # rows 31-37
put_text(cells, "TARGET: MON 1", 34, 40, GREY)      # rows 40-46
for c in range(8, 121): cells[(c, 49)] = DIML
for label, bx in (("PICK", 8), ("AUTO", 38), ("EXIT", 68)):
    w = 6*len(label)-1
    for c in range(bx, bx+w+4):
        cells[(c, 52)] = GREY
        cells[(c, 62)] = GREY
    for r in range(52, 63):
        cells[(bx, r)] = GREY
        cells[(bx+w+3, r)] = GREY
    put_text(cells, label, bx+2, 54, WHITE)
put_text(cells, "DETAILS", 8, 68, SOFT)             # rows 68-74
stamp_grid(img, W, H, PW, cells)
p7 = os.path.join(out, "07-window-mockup.png")
write_png(p7, W, H, img)
print("wrote", p7, os.path.getsize(p7))

# ---------- 08: picker ring, orange, 2 orbiting chasers ----------
W, H = 440, 340
img = blank(W, H)
cells = {}
for r in range(H//10):
    for c in range(W//10):
        cells[(c, r)] = OFF
stamp_grid(img, W, H, 10, cells)

def ring_at(x0, y0, x1, y1, r):
    top = (x1-r) - (x0+r)
    side = (y1-r) - (y0+r)
    arc = math.pi*r/2
    L = 2*top + 2*side + 4*arc
    def at(s):
        s %= L
        if s < top: return (x0+r+s, y0)
        s -= top
        if s < arc:
            a = -math.pi/2 + s/r
            return (x1-r+r*math.cos(a), y0+r+r*math.sin(a))
        s -= arc
        if s < side: return (x1, y0+r+s)
        s -= side
        if s < arc:
            a = s/r
            return (x1-r+r*math.cos(a), y1-r+r*math.sin(a))
        s -= arc
        if s < top: return (x1-r-s, y1)
        s -= top
        if s < arc:
            a = math.pi/2 + s/r
            return (x0+r+r*math.cos(a), y1-r+r*math.sin(a))
        s -= arc
        if s < side: return (x0, y1-r-s)
        s -= side
        a = math.pi + s/r
        return (x0+r+r*math.cos(a), y0+r+r*math.sin(a))
    return at, L

at, L = ring_at(70, 70, 370, 270, 24)
s = 0.0
while s < L:
    x, y = at(s)
    stamp_dot(img, W, H, x, y, 5, ORANGE)
    s += 14
for s0 in (0.30*L, 0.80*L):
    for i, f in ((2, 0.20), (1, 0.40)):
        x, y = at(s0 - i*14)
        stamp_dot(img, W, H, x, y, 5, blend(BG, ORANGE, f))
    x, y = at(s0)
    stamp_dot(img, W, H, x, y, 6.5, ORANGE)
p8 = os.path.join(out, "08-picker-ring.png")
write_png(p8, W, H, img)
print("wrote", p8, os.path.getsize(p8))
print("done")
