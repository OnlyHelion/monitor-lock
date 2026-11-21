//! The dot-matrix design language (ADR 0001): one pure, unit-tested module
//! that owns the palette, the 5×7 uppercase dot font, the 18×18 padlock glyph
//! (solid and hollow), the cell grid, the anti-aliased "real dot" renderer,
//! the one-shot wave math, and the picker-ring geometry. No Win32, no egui.
//!
//! Every surface draws from this one source — the window panel, the monitor
//! picker overlay, and the four icon destinations — so they can never drift
//! apart. The constants are copied from the approved prototype renderer
//! (`render_v2.py`, which generated `docs/design/dot-matrix/mockups/`); the
//! implementation must match those mockups at 1× DPI.

/// An RGBA colour (0..255 per channel; the panel is opaque, alpha 255).
pub type Rgba = [u8; 4];

/// Panel background: emitted where no dot covers a pixel.
pub const BG: Rgba = [14, 15, 19, 255];
/// An unlit dot, faintly visible across the whole panel.
pub const OFF: Rgba = [38, 40, 46, 255];
/// Locked / engaged orange (user-specified #FFB347).
pub const ORANGE: Rgba = [255, 179, 71, 255];
/// Released white/gray (#E8E8EC).
pub const WHITE: Rgba = [232, 232, 236, 255];
/// Button frames and the target status row (#888888).
pub const GREY: Rgba = [136, 136, 136, 255];
/// Separator lines (#3A3D45).
pub const DIML: Rgba = [58, 61, 69, 255];
/// The DETAILS label (#60636C).
pub const SOFT: Rgba = [96, 99, 108, 255];

/// The dot's diameter as a fraction of the cell pitch (a 0.78-pitch squircle,
/// exponent 4).
const DOT_RATIO: f32 = 0.78;
/// Picker ring inset from the monitor bounds, and the corner radius.
const RING_INSET: f32 = 24.0;
const RING_RADIUS: f32 = 24.0;
/// The picker ring's dots: radius 5 px on 14 px arc spacing (ticket #10,
/// mockup 08).
pub const RING_DOT_RADIUS: f32 = 5.0;
pub const RING_DOT_SPACING: f32 = 14.0;
/// The orbiting chasers: radius 7 px (ticket #10).
pub const CHASER_RADIUS: f32 = 7.0;
/// A chaser's tail dots, nearest first: (distance behind in arc px, alpha).
/// Same dot size as the ring, fading (ticket #10).
pub const CHASER_TAILS: [(f32, u8); 2] = [(14.0, 102), (28.0, 51)];
/// One chaser orbit takes 6 s — the app's only continuous animation
/// (ticket #10, ADR 0001).
pub const CHASER_PERIOD_SECS: f32 = 6.0;

/// A cell coordinate: (col, row), column first.
pub type Cell = (u32, u32);

/// The padlock glyph variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Padlock {
    /// Full body with the keyhole cut out (locked, released).
    Solid,
    /// 1-dot outline with a lit keyhole (suspended with a pin).
    Hollow,
}

/// The window panel content: the padlock state, the three status rows, and
/// the detail rows (shown only while DETAILS is open).
#[derive(Clone, Debug)]
pub struct Panel {
    /// The padlock glyph (solid locked / solid released / hollow suspended).
    pub padlock: Padlock,
    /// The padlock colour; the first status row shares it, so the lock state
    /// reads in two places at once.
    pub padlock_color: Rgba,
    /// The three status rows, top to bottom (lock state / window title /
    /// target monitor).
    pub status: [String; 3],
    /// The diagnostic rows, shown only while DETAILS is open.
    pub detail_rows: Vec<String>,
}

/// One placed button: its cell rect, for hit-testing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ButtonRect {
    pub col: u32,
    pub row: u32,
    pub w: u32,
    pub h: u32,
}

/// The settled layout's placed positions, in cells, for hit-testing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Placed {
    /// Top-left cell of the MONITOR LOCK heading.
    pub heading: Cell,
    /// The separator rows (below the heading, above the buttons).
    pub separators: [u32; 2],
    /// Top-left cell of the 18×18 padlock.
    pub padlock: Cell,
    /// Top-left cell of each status row.
    pub status: [Cell; 3],
    /// The three button frames (PICK / AUTO / EXIT).
    pub buttons: [ButtonRect; 3],
    /// The DETAILS label (also a button).
    pub details: ButtonRect,
    /// Top-left cell of the first detail row, and the step between rows.
    pub detail_origin: Cell,
    pub detail_step: u32,
}

/// The lit cells of `ch` in the 5×7 dot font, as (col, row) within the cell.
///
/// The font is uppercase Latin; `ch` is uppercased first. A character with no
/// glyph falls back to the space (blank) — the same fallback the approved
/// prototype uses.
pub fn glyph_cells(ch: char) -> Vec<Cell> {
    // The approved 5×7 font, copied verbatim from render_v2.py.
    let rows: &[&str; 7] = match ch.to_ascii_uppercase() {
        'A' => &[
            "01110", "10001", "10001", "11111", "10001", "10001", "10001",
        ],
        'B' => &[
            "11110", "10001", "10001", "11110", "10001", "10001", "11110",
        ],
        'C' => &[
            "01111", "10000", "10000", "10000", "10000", "10000", "01111",
        ],
        'D' => &[
            "11100", "10010", "10001", "10001", "10001", "10010", "11100",
        ],
        'E' => &[
            "11111", "10000", "10000", "11110", "10000", "10000", "11111",
        ],
        'F' => &[
            "11111", "10000", "10000", "11110", "10000", "10000", "10000",
        ],
        'G' => &[
            "01111", "10000", "10000", "10111", "10001", "10001", "01110",
        ],
        'H' => &[
            "10001", "10001", "10001", "11111", "10001", "10001", "10001",
        ],
        'I' => &[
            "01110", "00100", "00100", "00100", "00100", "00100", "01110",
        ],
        'J' => &[
            "00111", "00010", "00010", "00010", "00010", "10010", "01100",
        ],
        'K' => &[
            "10001", "10010", "10100", "11000", "10100", "10010", "10001",
        ],
        'L' => &[
            "10000", "10000", "10000", "10000", "10000", "10000", "11111",
        ],
        'M' => &[
            "10001", "11011", "10101", "10101", "10001", "10001", "10001",
        ],
        'N' => &[
            "10001", "11001", "10101", "10011", "10001", "10001", "10001",
        ],
        'O' => &[
            "01110", "10001", "10001", "10001", "10001", "10001", "01110",
        ],
        'P' => &[
            "11110", "10001", "10001", "11110", "10000", "10000", "10000",
        ],
        'Q' => &[
            "01110", "10001", "10001", "10001", "10101", "10010", "01101",
        ],
        'R' => &[
            "11110", "10001", "10001", "11110", "10100", "10010", "10001",
        ],
        'S' => &[
            "01111", "10000", "10000", "01110", "00001", "00001", "11110",
        ],
        'T' => &[
            "11111", "00100", "00100", "00100", "00100", "00100", "00100",
        ],
        'U' => &[
            "10001", "10001", "10001", "10001", "10001", "10001", "01110",
        ],
        'V' => &[
            "10001", "10001", "10001", "10001", "10001", "01010", "00100",
        ],
        'W' => &[
            "10001", "10001", "10001", "10101", "10101", "11011", "10001",
        ],
        'X' => &[
            "10001", "10001", "01010", "00100", "01010", "10001", "10001",
        ],
        'Y' => &[
            "10001", "10001", "01010", "00100", "00100", "00100", "00100",
        ],
        'Z' => &[
            "11111", "00001", "00010", "00100", "01000", "10000", "11111",
        ],
        '0' => &[
            "01110", "10001", "10011", "10101", "11001", "10001", "01110",
        ],
        '1' => &[
            "00100", "01100", "00100", "00100", "00100", "00100", "01110",
        ],
        '2' => &[
            "01110", "10001", "00001", "00110", "01000", "10000", "11111",
        ],
        '3' => &[
            "11110", "00001", "00001", "01110", "00001", "00001", "11110",
        ],
        '4' => &[
            "00010", "00110", "01010", "10010", "11111", "00010", "00010",
        ],
        '5' => &[
            "11111", "10000", "11110", "00001", "00001", "10001", "01110",
        ],
        '6' => &[
            "01110", "10000", "10000", "11110", "10001", "10001", "01110",
        ],
        '7' => &[
            "11111", "00001", "00010", "00100", "01000", "01000", "01000",
        ],
        '8' => &[
            "01110", "10001", "10001", "01110", "10001", "10001", "01110",
        ],
        '9' => &[
            "01110", "10001", "10001", "01111", "00001", "00001", "01110",
        ],
        ' ' => &[
            "00000", "00000", "00000", "00000", "00000", "00000", "00000",
        ],
        ':' => &[
            "00000", "00100", "00000", "00000", "00000", "00100", "00000",
        ],
        '-' => &[
            "00000", "00000", "00000", "01110", "00000", "00000", "00000",
        ],
        '(' => &[
            "00010", "00100", "01000", "01000", "01000", "00100", "00010",
        ],
        ')' => &[
            "01000", "00100", "00010", "00100", "00100", "00100", "01000",
        ],
        '.' => &[
            "00000", "00000", "00000", "00000", "00000", "00000", "00100",
        ],
        // No glyph: the prototype's space (blank) fallback.
        _ => return Vec::new(),
    };
    let mut cells = Vec::new();
    for (r, row) in rows.iter().enumerate() {
        for (c, bit) in row.bytes().enumerate() {
            if bit == b'1' {
                cells.push((c as u32, r as u32));
            }
        }
    }
    cells
}

/// The lit cells of the 18×18 padlock, as (col, row) — the same coordinate
/// order as every other cell in the panel. The solid variant cuts the
/// keyhole out of the body; the hollow variant is a 1-dot outline with a
/// lit keyhole.
pub fn padlock_cells(variant: Padlock) -> Vec<(u32, u32)> {
    // Copied from padlock_solid / padlock_hollow in render_v2.py.
    let keyhole: [(u32, u32); 4] = [(8, 11), (9, 11), (8, 12), (9, 12)];
    let mut lit: Vec<(u32, u32)> = Vec::new();
    match variant {
        Padlock::Solid => {
            for r in [2u32, 3] {
                for c in 5..13 {
                    lit.push((c, r)); // shackle top, 2 thick
                }
            }
            for r in 4..9 {
                for c in [5u32, 6, 11, 12] {
                    lit.push((c, r)); // shackle sides
                }
            }
            for r in 9..17 {
                for c in 3..15 {
                    lit.push((c, r)); // body
                }
            }
            for (c, r) in keyhole {
                lit.retain(|&p| p != (c, r)); // keyhole cut
            }
        }
        Padlock::Hollow => {
            for c in 5..13 {
                lit.push((c, 2)); // shackle top outline
            }
            for r in 3..9 {
                lit.push((5, r));
                lit.push((12, r)); // shackle sides, 1 thick
            }
            for c in 3..15 {
                lit.push((c, 9));
                lit.push((c, 16)); // body top/bottom
            }
            for r in 10..16 {
                lit.push((3, r));
                lit.push((14, r)); // body sides
            }
            for (c, r) in keyhole {
                lit.push((c, r)); // keyhole lit
            }
        }
    }
    lit
}

/// A cols×rows grid of per-cell colours, defaulting every cell to the off-dot
/// colour.
#[derive(Clone, Debug)]
pub struct CellMap {
    cols: u32,
    rows: u32,
    cells: Vec<Rgba>,
}

impl CellMap {
    /// A cols×rows grid, every cell the off-dot colour.
    pub fn new(cols: u32, rows: u32) -> Self {
        Self {
            cols,
            rows,
            cells: vec![OFF; (cols * rows) as usize],
        }
    }

    pub fn cols(&self) -> u32 {
        self.cols
    }

    pub fn rows(&self) -> u32 {
        self.rows
    }

    fn inside(&self, col: u32, row: u32) -> bool {
        (col as u64) < self.cols as u64 && (row as u64) < self.rows as u64
    }

    /// The colour of one cell; BG outside the grid.
    pub fn get(&self, col: u32, row: u32) -> Rgba {
        if !self.inside(col, row) {
            return BG;
        }
        self.cells[(row * self.cols + col) as usize]
    }

    /// Set one cell (no-op outside the grid).
    pub fn set(&mut self, col: u32, row: u32, color: Rgba) {
        if !self.inside(col, row) {
            return;
        }
        self.cells[(row * self.cols + col) as usize] = color;
    }

    /// Stamp `text` in the 5×7 dot font, top-left at (col, row). Each glyph
    /// advances the column by 6 (5 wide + 1 gap), as in the prototype.
    pub fn stamp_text(&mut self, text: &str, col: u32, row: u32, color: Rgba) {
        let mut x = col;
        for ch in text.chars() {
            for (gc, gr) in glyph_cells(ch) {
                self.set(x + gc, row + gr, color);
            }
            x += 6;
        }
    }

    /// Stamp the 18×18 padlock, top-left at (col, row).
    pub fn stamp_padlock(&mut self, variant: Padlock, col: u32, row: u32, color: Rgba) {
        for (c, r) in padlock_cells(variant) {
            self.set(col + c, row + r, color);
        }
    }

    /// A horizontal run of `color` from (col0, row) to (col1, row), inclusive.
    pub fn hline(&mut self, row: u32, col0: u32, col1: u32, color: Rgba) {
        for c in col0..=col1 {
            self.set(c, row, color);
        }
    }

    /// A w×h rectangle outline of `color`, top-left at (col, row).
    pub fn frame(&mut self, col: u32, row: u32, w: u32, h: u32, color: Rgba) {
        self.hline(row, col, col + w - 1, color);
        self.hline(row + h - 1, col, col + w - 1, color);
        for r in row..row + h {
            self.set(col, r, color);
            self.set(col + w - 1, r, color);
        }
    }
}

/// Stamp the settled layout into a fresh 128-column cell map from `content`
/// (growing the row count for the detail rows), and return the placed
/// positions for hit-testing.
pub fn layout(content: &Panel) -> (CellMap, Placed) {
    // The settled layout, in cells (col, row) — the single source the stamps
    // below and the returned `Placed` hit-test rects both use.
    const PANEL_COLS: u32 = 128;
    const PANEL_ROWS: u32 = 80;
    const HEADING: Cell = (8, 7);
    const SEP_X: u32 = 8;
    const SEP_LEN: u32 = 120;
    const SEP_ROWS: [u32; 2] = [17, 49];
    const PADLOCK: Cell = (8, 21);
    const STATUS: [Cell; 3] = [(34, 22), (34, 31), (34, 40)];
    const BUTTON_ROW: u32 = 52;
    const BUTTON_COLS: [u32; 3] = [8, 38, 68];
    const BUTTON_W: u32 = 27; // 6n+3 for the 4-char labels: inner 6n−1 + 2-cell margins
    const BUTTON_H: u32 = 11;
    const DETAILS: (u32, u32, u32, u32) = (8, 68, 41, 7); // col, row, w, h
    const DETAIL_ORIGIN: Cell = (8, 78); // 2 blank rows under the DETAILS label
    const DETAIL_STEP: u32 = 9; // TEXT_ROWS + 2 blank dot-rows, like the status rows
    const TEXT_ROWS: u32 = 7; // the 5×7 font's row height

    let n = content.detail_rows.len() as u32;
    // The closed panel is the 128×80 grid (640×400 at pitch 5); detail rows
    // grow the grid with a 4-row bottom margin.
    let detail_end = if n > 0 {
        DETAIL_ORIGIN.1 + (n - 1) * DETAIL_STEP + TEXT_ROWS
    } else {
        0
    };
    let mut map = CellMap::new(PANEL_COLS, PANEL_ROWS.max(detail_end + 4));

    map.stamp_text("MONITOR LOCK", HEADING.0, HEADING.1, WHITE);
    map.hline(SEP_ROWS[0], SEP_X, SEP_LEN, DIML);
    map.stamp_padlock(content.padlock, PADLOCK.0, PADLOCK.1, content.padlock_color);
    // Row 1 carries the lock state and shares the padlock colour, so the
    // state reads in two places at once; rows 2 and 3 are the settled
    // white/grey.
    map.stamp_text(
        &content.status[0],
        STATUS[0].0,
        STATUS[0].1,
        content.padlock_color,
    );
    map.stamp_text(&content.status[1], STATUS[1].0, STATUS[1].1, WHITE);
    map.stamp_text(&content.status[2], STATUS[2].0, STATUS[2].1, GREY);
    map.hline(SEP_ROWS[1], SEP_X, SEP_LEN, DIML);
    for (label, bx) in [
        ("PICK", BUTTON_COLS[0]),
        ("AUTO", BUTTON_COLS[1]),
        ("EXIT", BUTTON_COLS[2]),
    ] {
        map.frame(bx, BUTTON_ROW, BUTTON_W, BUTTON_H, GREY);
        map.stamp_text(label, bx + 2, BUTTON_ROW + 2, WHITE);
    }
    map.stamp_text("DETAILS", DETAILS.0, DETAILS.1, SOFT);
    for (i, row) in content.detail_rows.iter().enumerate() {
        map.stamp_text(
            row,
            DETAIL_ORIGIN.0,
            DETAIL_ORIGIN.1 + i as u32 * DETAIL_STEP,
            SOFT,
        );
    }

    let placed = Placed {
        heading: HEADING,
        separators: SEP_ROWS,
        padlock: PADLOCK,
        status: STATUS,
        buttons: [
            ButtonRect {
                col: BUTTON_COLS[0],
                row: BUTTON_ROW,
                w: BUTTON_W,
                h: BUTTON_H,
            },
            ButtonRect {
                col: BUTTON_COLS[1],
                row: BUTTON_ROW,
                w: BUTTON_W,
                h: BUTTON_H,
            },
            ButtonRect {
                col: BUTTON_COLS[2],
                row: BUTTON_ROW,
                w: BUTTON_W,
                h: BUTTON_H,
            },
        ],
        details: ButtonRect {
            col: DETAILS.0,
            row: DETAILS.1,
            w: DETAILS.2,
            h: DETAILS.3,
        },
        detail_origin: DETAIL_ORIGIN,
        detail_step: DETAIL_STEP,
    };
    (map, placed)
}

/// Render a cell map into a `width`×`height` top-down RGBA buffer (4 bytes per
/// pixel) at `pitch` px per cell. Every pixel starts at BG; each cell is
/// stamped as an anti-aliased "real dot" of its colour (a superellipse
/// |x/a|⁴+|y/a|⁴ ≤ 1 with a = 0.39 × pitch, i.e. 0.78 of the pitch). The
/// panel is opaque (alpha 255); pixels beyond the grid stay BG.
pub fn render(map: &CellMap, pitch: u32, width: u32, height: u32) -> Vec<u8> {
    // Coverage LUT for this pitch: a pitch×pitch table, each entry the
    // fraction of the pixel inside the superellipse |dx/a|⁴ + |dy/a|⁴ ≤ 1,
    // 16×16 supersampled — the same make_lut the approved mockups were
    // rendered with, so output matches them at 1× DPI.
    const SS: u32 = 16;
    let a = pitch as f32 * DOT_RATIO / 2.0;
    let mut lut = vec![0.0f32; (pitch * pitch) as usize];
    for oy in 0..pitch {
        for ox in 0..pitch {
            let mut c = 0u32;
            for sy in 0..SS {
                let dy = (oy as f32 + (sy as f32 + 0.5) / SS as f32 - pitch as f32 / 2.0).abs() / a;
                let dyv = dy * dy * dy * dy;
                for sx in 0..SS {
                    let dx =
                        (ox as f32 + (sx as f32 + 0.5) / SS as f32 - pitch as f32 / 2.0).abs() / a;
                    if dx * dx * dx * dx + dyv <= 1.0 {
                        c += 1;
                    }
                }
            }
            lut[(oy * pitch + ox) as usize] = c as f32 / (SS * SS) as f32;
        }
    }

    let mut buf = BG.repeat((width * height) as usize);
    for r in 0..map.rows() {
        for c in 0..map.cols() {
            let x0 = c * pitch;
            let y0 = r * pitch;
            if x0 >= width || y0 >= height {
                continue;
            }
            let wspan = pitch.min(width - x0);
            let hspan = pitch.min(height - y0);
            let color = map.get(c, r);
            for oy in 0..hspan {
                for ox in 0..wspan {
                    let cov = lut[(oy * pitch + ox) as usize];
                    if cov > 0.0 {
                        let i = ((y0 + oy) * width + (x0 + ox)) as usize * 4;
                        // The prototype's int() blend, truncating toward zero
                        // (a floor here: the value stays in [0, 255]).
                        for k in 0..3 {
                            let v = (BG[k] as f32 + (color[k] as f32 - BG[k] as f32) * cov) as u32;
                            buf[i + k] = v.min(255) as u8;
                        }
                        buf[i + 3] = 255;
                    }
                }
            }
        }
    }
    buf
}

/// One-shot wave intensity at distance `d` (px from the padlock centre) and
/// progress `t` in [0,1] over the 250 ms one-shot:
/// I = exp(−(d−R)²/(2·13²)) · (1−t) · max(0, 1−d/140), R(t) = 140·t.
/// 1 at the padlock centre at t=0; 0 at t=1; 0 beyond the front's reach.
pub fn wave_intensity(d: f32, t: f32) -> f32 {
    // I = exp(−(d−R)²/(2·13²)) · (1−t) · max(0, 1−d/140), R(t) = 140·t.
    let g = d - 140.0 * t;
    let falloff = (1.0 - d / 140.0).max(0.0);
    (-g * g / (2.0 * 13.0 * 13.0)).exp() * (1.0 - t) * falloff
}

/// The picker ring's total arc length: a rounded rectangle inset 24 px from
/// `bounds`, corner radius 24 px.
pub fn ring_length(bounds: &crate::seam::Rect) -> f32 {
    let (x0, y0, x1, y1) = ring_box(bounds);
    let top = (x1 - RING_RADIUS) - (x0 + RING_RADIUS);
    let side = (y1 - RING_RADIUS) - (y0 + RING_RADIUS);
    2.0 * top + 2.0 * side + 2.0 * std::f32::consts::PI * RING_RADIUS
}

/// The ring's bounding box: `bounds` inset RING_INSET on every side.
fn ring_box(bounds: &crate::seam::Rect) -> (f32, f32, f32, f32) {
    (
        bounds.x as f32 + RING_INSET,
        bounds.y as f32 + RING_INSET,
        bounds.x as f32 + bounds.width as f32 - RING_INSET,
        bounds.y as f32 + bounds.height as f32 - RING_INSET,
    )
}

/// The point at arc length `s` in [0, L) on the picker ring (wraps mod L).
/// s=0 is the start of the top edge, walking clockwise.
pub fn ring_point(bounds: &crate::seam::Rect, s: f32) -> (f32, f32) {
    // A faithful port of ring_at in render_v2.py: s=0 is the start of the top
    // edge (after the top-left corner arc), walking clockwise.
    let (x0, y0, x1, y1) = ring_box(bounds);
    let r = RING_RADIUS;
    let top = (x1 - r) - (x0 + r);
    let side = (y1 - r) - (y0 + r);
    let arc = std::f32::consts::PI * r / 2.0;
    let l = 2.0 * top + 2.0 * side + 4.0 * arc;
    let mut s = s.rem_euclid(l);
    if s < top {
        return (x0 + r + s, y0);
    }
    s -= top;
    if s < arc {
        let a = -std::f32::consts::FRAC_PI_2 + s / r;
        return (x1 - r + r * a.cos(), y0 + r + r * a.sin());
    }
    s -= arc;
    if s < side {
        return (x1, y0 + r + s);
    }
    s -= side;
    if s < arc {
        let a = s / r;
        return (x1 - r + r * a.cos(), y1 - r + r * a.sin());
    }
    s -= arc;
    if s < top {
        return (x1 - r - s, y1);
    }
    s -= top;
    if s < arc {
        let a = std::f32::consts::FRAC_PI_2 + s / r;
        return (x0 + r + r * a.cos(), y1 - r + r * a.sin());
    }
    s -= arc;
    if s < side {
        return (x0, y1 - r - s);
    }
    s -= side;
    let a = std::f32::consts::PI + s / r;
    (x0 + r + r * a.cos(), y0 + r + r * a.sin())
}

/// The chaser orbit phase in [0,1) from `uptime_secs` on the current monitor:
/// one orbit per CHASER_PERIOD_SECS, so (uptime / period) mod 1. The uptime
/// (not the wall clock) is the phase origin, so it resets when the hovered
/// monitor changes (ticket #10).
pub fn chaser_phase(uptime_secs: f32) -> f32 {
    (uptime_secs / CHASER_PERIOD_SECS).rem_euclid(1.0)
}

/// How many ring dots close the ring at ~RING_DOT_SPACING arc spacing:
/// round(L / 14), so the last dot lands back at the start (ticket #10).
pub fn ring_dot_count(bounds: &crate::seam::Rect) -> u32 {
    (ring_length(bounds) / RING_DOT_SPACING).round().max(1.0) as u32
}

/// Stamp an anti-aliased "real dot" of radius `rad` centred at (cx, cy) with
/// `color` over an existing top-down RGBA buffer (alpha-composited "over", so
/// it works on the overlay's transparent buffer as well as opaque ones).
pub fn stamp_dot(buf: &mut [u8], width: u32, height: u32, cx: f32, cy: f32, rad: f32, color: Rgba) {
    // The pixel box the dot can touch (one pixel of slack each side), as in
    // the prototype's stamp_dot, 8×8 supersampled.
    const SS: u32 = 8;
    let x0 = (cx - rad - 1.0) as i32;
    let x1 = (cx + rad + 2.0) as i32;
    let y0 = (cy - rad - 1.0) as i32;
    let y1 = (cy + rad + 2.0) as i32;
    for y in (y0.max(0))..(y1.min(height as i32)) {
        for x in (x0.max(0))..(x1.min(width as i32)) {
            let mut c = 0u32;
            for sy in 0..SS {
                let dy = (y as f32 + (sy as f32 + 0.5) / SS as f32 - cy).abs() / rad;
                let dyv = dy * dy * dy * dy;
                for sx in 0..SS {
                    let dx = (x as f32 + (sx as f32 + 0.5) / SS as f32 - cx).abs() / rad;
                    if dx * dx * dx * dx + dyv <= 1.0 {
                        c += 1;
                    }
                }
            }
            let cov = c as f32 / (SS * SS) as f32;
            if cov > 0.0 {
                let i = ((y as u32) * width + (x as u32)) as usize * 4;
                // Alpha "over", so it works on the overlay's transparent
                // buffer as well as opaque ones.
                let sa = cov * color[3] as f32 / 255.0;
                let da = buf[i + 3] as f32 / 255.0;
                let out_a = sa + da * (1.0 - sa);
                if out_a > 0.0 {
                    for k in 0..3 {
                        let v =
                            (color[k] as f32 * sa + buf[i + k] as f32 * da * (1.0 - sa)) / out_a;
                        buf[i + k] = v.min(255.0) as u8;
                    }
                    buf[i + 3] = (out_a * 255.0).min(255.0) as u8;
                }
            }
        }
    }
}

/// Whether the local point (ox, oy) in a cell of pitch `pitch` (both in
/// [0, pitch), from the cell's top-left corner) is inside the "real dot": the
/// superellipse |x/a|⁴ + |y/a|⁴ ≤ 1, a = 0.39×pitch, centred in the cell.
/// The same inequality `render`'s LUT and `stamp_dot` supersample — the icon
/// counts 4×4 subsamples of it instead of building a per-pixel coverage LUT.
pub fn dot_at(ox: f32, oy: f32, pitch: f32) -> bool {
    let a = pitch * DOT_RATIO / 2.0;
    let dx = (ox - pitch / 2.0).abs() / a;
    let dy = (oy - pitch / 2.0).abs() / a;
    dx * dx * dx * dx + dy * dy * dy * dy <= 1.0
}

/// The effective state key the window renders: the padlock state, the
/// status rows, and the one-shot wave all key off it (any change starts a
/// fresh wave).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StateKey {
    /// The Lock is released and no pin is set: the cursor is free; the
    /// padlock is solid white.
    Released,
    /// A pinned monitor with the Lock released (the pin outlives a focus
    /// loss): the padlock is hollow orange.
    Suspended,
    /// The Lock is engaged: the cursor is confined to the target monitor;
    /// the padlock is solid orange.
    Engaged,
    /// The Monitor picker is active: the Lock is released while picking.
    Picking,
}

/// Derive the effective state key from a snapshot. Precedence: picking,
/// engaged, suspended, released — the same derivation the poll thread (the
/// engaged edge) and the old UI (the suspended hint) used, in one place.
pub fn state_key(snap: &crate::core::LockSnapshot) -> StateKey {
    use crate::core::LockState;
    if snap.picking {
        StateKey::Picking
    } else if matches!(snap.lock, LockState::Engaged { .. }) {
        StateKey::Engaged
    } else if snap.pinned.is_some() {
        StateKey::Suspended
    } else {
        StateKey::Released
    }
}

/// The window panel content for one state: the padlock glyph and colour, the
/// three status rows (fit to the 94 cells from col 34), and the detail rows
/// (fit to the 120 cells from col 8, only while DETAILS is open). The dot
/// font renders uppercase regardless, so no case change here.
pub fn panel_content(snap: &crate::core::LockSnapshot, key: StateKey, details_open: bool) -> Panel {
    const STATUS_CELLS: u32 = 94; // col 34 of the 128-col grid
    const DETAIL_CELLS: u32 = 120; // col 8 of the 128-col grid

    let (padlock, padlock_color, rows3) = match key {
        StateKey::Picking => (
            Padlock::Solid,
            WHITE,
            vec![
                "PICKING MONITOR".to_string(),
                "CLICK TO PIN".to_string(),
                "ESC TO CANCEL".to_string(),
            ],
        ),
        StateKey::Engaged => (
            Padlock::Solid,
            ORANGE,
            vec![
                "LOCK: ENGAGED".to_string(),
                foreground_line(snap),
                target_line(snap),
            ],
        ),
        StateKey::Suspended => (
            Padlock::Hollow,
            ORANGE,
            vec![
                "LOCK: SUSPENDED".to_string(),
                foreground_line(snap),
                target_line(snap),
            ],
        ),
        StateKey::Released => (
            Padlock::Solid,
            WHITE,
            vec![
                "LOCK: RELEASED".to_string(),
                // No Game window: the cursor is simply free.
                if snap.game_window.is_some() {
                    foreground_line(snap)
                } else {
                    "CURSOR FREE".to_string()
                },
                target_line(snap),
            ],
        ),
    };
    let status = [
        fit_width(&rows3[0], STATUS_CELLS),
        fit_width(&rows3[1], STATUS_CELLS),
        fit_width(&rows3[2], STATUS_CELLS),
    ];
    let detail_rows = if details_open {
        detail_row_list(snap)
            .into_iter()
            .map(|r| fit_width(&r, DETAIL_CELLS))
            .collect()
    } else {
        Vec::new()
    };
    Panel {
        padlock,
        padlock_color,
        status,
        detail_rows,
    }
}

/// The foreground window's title (status row 2, detail FG line): the title,
/// "NO TITLE" for a window that reports none, "NONE" when there is no
/// foreground window.
pub fn foreground_line(snap: &crate::core::LockSnapshot) -> String {
    match &snap.foreground {
        None => "NONE".to_string(),
        Some(w) if w.title.is_empty() => "NO TITLE".to_string(),
        Some(w) => w.title.clone(),
    }
}

/// The target monitor's id (status row 3): the pinned monitor while an
/// override is active (a pin outlives a focus loss), else the Game monitor;
/// "TARGET: NONE" without either.
pub fn target_line(snap: &crate::core::LockSnapshot) -> String {
    let id = snap
        .pinned
        .as_ref()
        .map(|p| p.monitor_id.clone())
        .or_else(|| snap.game_window.as_ref().map(|g| g.game_monitor_id.clone()));
    match id {
        Some(id) => format!("TARGET: {}", id),
        None => "TARGET: NONE".to_string(),
    }
}

/// The DETAILS diagnostic rows (dot-friendly): foreground, game, the
/// monitor list, and the clip re-asserts (only while nonzero).
pub fn detail_row_list(snap: &crate::core::LockSnapshot) -> Vec<String> {
    let mut rows = vec![
        format!("FG: {}", foreground_line(snap)),
        format!(
            "GAME: {}",
            match &snap.game_window {
                None => "NONE".to_string(),
                Some(g) if g.title.is_empty() => "NO TITLE".to_string(),
                Some(g) => g.title.clone(),
            }
        ),
        format!("MONITORS: {}", snap.monitors.len()),
    ];
    for m in &snap.monitors {
        rows.push(format!("{} {}x{}", m.id, m.bounds.width, m.bounds.height));
    }
    if snap.clip_reasserts > 0 {
        rows.push(format!("CLIP REASSERTS: {}", snap.clip_reasserts));
    }
    rows
}

/// Blend `a` toward `b` by `t` in [0,1] (the wave; the PICK dimming).
pub fn blend_toward(a: Rgba, b: Rgba, t: f32) -> Rgba {
    let mut out = a;
    for k in 0..3 {
        out[k] = (a[k] as f32 + (b[k] as f32 - a[k] as f32) * t)
            .round()
            .min(255.0) as u8;
    }
    out
}

/// The PICK button's dim while picking: 60% toward the OFF dot colour.
pub fn dim(c: Rgba) -> Rgba {
    blend_toward(c, OFF, 0.6)
}

/// Truncate `text` so it fits `max_cells` cells of the 5×7 font: n
/// characters occupy 6n − 1 cells (5 wide + a 1-cell gap per character, the
/// last one without a gap), so at most (max_cells + 1) / 6 characters fit.
/// The dot font renders uppercase regardless, so no case change here.
pub fn fit_width(text: &str, max_cells: u32) -> String {
    let max_chars = (max_cells + 1) / 6;
    text.chars().take(max_chars as usize).collect()
}

/// A pointer position in panel points (the panel's top-left is (0,0)) to
/// its cell, None outside the grid. `pitch_x` / `pitch_y` are the drawn
/// cell pitches in points (the client area divided by the grid's
/// cols/rows).
pub fn cell_at(x: f32, y: f32, pitch_x: f32, pitch_y: f32, cols: u32, rows: u32) -> Option<Cell> {
    if pitch_x.is_nan()
        || pitch_y.is_nan()
        || pitch_x <= 0.0
        || pitch_y <= 0.0
        || x < 0.0
        || y < 0.0
    {
        return None;
    }
    let c = (x / pitch_x).floor() as i64;
    let r = (y / pitch_y).floor() as i64;
    if c < 0 || r < 0 || c >= cols as i64 || r >= rows as i64 {
        return None;
    }
    Some((c as u32, r as u32))
}

/// Whether a cell is inside a button rect (the right/bottom edges are
/// exclusive, like `Rect::contains`).
pub fn cell_in_rect(cell: Cell, b: &ButtonRect) -> bool {
    cell.0 >= b.col && cell.0 < b.col + b.w && cell.1 >= b.row && cell.1 < b.row + b.h
}

/// The one-shot state-change wave's duration in ms (spec: ≤300 ms; the
/// settled prototype uses 250 ms).
pub const WAVE_MS: u128 = 250;

/// The wave's progress t in [0,1) from its elapsed time, None once the
/// one-shot has finished (the panel is static again).
pub fn wave_progress(elapsed_ms: u128, duration_ms: u128) -> Option<f32> {
    if duration_ms == 0 || elapsed_ms >= duration_ms {
        return None;
    }
    Some((elapsed_ms as f32 / duration_ms as f32).min(1.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seam::Rect;

    /// The approved 5×7 font, transcribed independently from render_v2.py —
    /// the specification the implementation must match.
    fn expected_glyph(ch: char) -> &'static [&'static str; 7] {
        match ch {
            'A' => &[
                "01110", "10001", "10001", "11111", "10001", "10001", "10001",
            ],
            'B' => &[
                "11110", "10001", "10001", "11110", "10001", "10001", "11110",
            ],
            'C' => &[
                "01111", "10000", "10000", "10000", "10000", "10000", "01111",
            ],
            'D' => &[
                "11100", "10010", "10001", "10001", "10001", "10010", "11100",
            ],
            'E' => &[
                "11111", "10000", "10000", "11110", "10000", "10000", "11111",
            ],
            'F' => &[
                "11111", "10000", "10000", "11110", "10000", "10000", "10000",
            ],
            'G' => &[
                "01111", "10000", "10000", "10111", "10001", "10001", "01110",
            ],
            'H' => &[
                "10001", "10001", "10001", "11111", "10001", "10001", "10001",
            ],
            'I' => &[
                "01110", "00100", "00100", "00100", "00100", "00100", "01110",
            ],
            'J' => &[
                "00111", "00010", "00010", "00010", "00010", "10010", "01100",
            ],
            'K' => &[
                "10001", "10010", "10100", "11000", "10100", "10010", "10001",
            ],
            'L' => &[
                "10000", "10000", "10000", "10000", "10000", "10000", "11111",
            ],
            'M' => &[
                "10001", "11011", "10101", "10101", "10001", "10001", "10001",
            ],
            'N' => &[
                "10001", "11001", "10101", "10011", "10001", "10001", "10001",
            ],
            'O' => &[
                "01110", "10001", "10001", "10001", "10001", "10001", "01110",
            ],
            'P' => &[
                "11110", "10001", "10001", "11110", "10000", "10000", "10000",
            ],
            'Q' => &[
                "01110", "10001", "10001", "10001", "10101", "10010", "01101",
            ],
            'R' => &[
                "11110", "10001", "10001", "11110", "10100", "10010", "10001",
            ],
            'S' => &[
                "01111", "10000", "10000", "01110", "00001", "00001", "11110",
            ],
            'T' => &[
                "11111", "00100", "00100", "00100", "00100", "00100", "00100",
            ],
            'U' => &[
                "10001", "10001", "10001", "10001", "10001", "10001", "01110",
            ],
            'V' => &[
                "10001", "10001", "10001", "10001", "10001", "01010", "00100",
            ],
            'W' => &[
                "10001", "10001", "10001", "10101", "10101", "11011", "10001",
            ],
            'X' => &[
                "10001", "10001", "01010", "00100", "01010", "10001", "10001",
            ],
            'Y' => &[
                "10001", "10001", "01010", "00100", "00100", "00100", "00100",
            ],
            'Z' => &[
                "11111", "00001", "00010", "00100", "01000", "10000", "11111",
            ],
            '0' => &[
                "01110", "10001", "10011", "10101", "11001", "10001", "01110",
            ],
            '1' => &[
                "00100", "01100", "00100", "00100", "00100", "00100", "01110",
            ],
            '2' => &[
                "01110", "10001", "00001", "00110", "01000", "10000", "11111",
            ],
            '3' => &[
                "11110", "00001", "00001", "01110", "00001", "00001", "11110",
            ],
            '4' => &[
                "00010", "00110", "01010", "10010", "11111", "00010", "00010",
            ],
            '5' => &[
                "11111", "10000", "11110", "00001", "00001", "10001", "01110",
            ],
            '6' => &[
                "01110", "10000", "10000", "11110", "10001", "10001", "01110",
            ],
            '7' => &[
                "11111", "00001", "00010", "00100", "01000", "01000", "01000",
            ],
            '8' => &[
                "01110", "10001", "10001", "01110", "10001", "10001", "01110",
            ],
            '9' => &[
                "01110", "10001", "10001", "01111", "00001", "00001", "01110",
            ],
            ' ' => &[
                "00000", "00000", "00000", "00000", "00000", "00000", "00000",
            ],
            ':' => &[
                "00000", "00100", "00000", "00000", "00000", "00100", "00000",
            ],
            '-' => &[
                "00000", "00000", "00000", "01110", "00000", "00000", "00000",
            ],
            '(' => &[
                "00010", "00100", "01000", "01000", "01000", "00100", "00010",
            ],
            ')' => &[
                "01000", "00100", "00010", "00100", "00100", "00100", "01000",
            ],
            '.' => &[
                "00000", "00000", "00000", "00000", "00000", "00000", "00100",
            ],
            _ => &[
                "00000", "00000", "00000", "00000", "00000", "00000", "00000",
            ],
        }
    }

    /// The (col, row) cells a set of row strings lights.
    fn cells_from_rows(rows: &[&str; 7]) -> Vec<Cell> {
        let mut cells = Vec::new();
        for (r, row) in rows.iter().enumerate() {
            for (c, bit) in row.bytes().enumerate() {
                if bit == b'1' {
                    cells.push((c as u32, r as u32));
                }
            }
        }
        // Row-major scan order, matching glyph_cells and the hand-checked list.
        cells
    }

    #[test]
    fn every_glyph_matches_the_approved_font() {
        // Every character the UI text uses (and the rest of the font).
        let chars: Vec<char> = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 :-.()"
            .chars()
            .collect();
        for ch in chars {
            assert_eq!(
                glyph_cells(ch),
                cells_from_rows(expected_glyph(ch)),
                "glyph {ch:?} diverges from the approved font"
            );
        }
    }

    #[test]
    fn glyph_a_is_the_hand_checked_cell_set() {
        // Worked by hand from "01110 10001 10001 11111 10001 10001 10001".
        let want: Vec<Cell> = vec![
            (1, 0),
            (2, 0),
            (3, 0),
            (0, 1),
            (4, 1),
            (0, 2),
            (4, 2),
            (0, 3),
            (1, 3),
            (2, 3),
            (3, 3),
            (4, 3),
            (0, 4),
            (4, 4),
            (0, 5),
            (4, 5),
            (0, 6),
            (4, 6),
        ];
        assert_eq!(glyph_cells('A'), want);
    }

    #[test]
    fn unknown_characters_fall_back_to_blank() {
        assert!(glyph_cells('?').is_empty());
        assert!(glyph_cells('é').is_empty());
        // Lowercase is uppercased before the lookup.
        assert_eq!(glyph_cells('a'), glyph_cells('A'));
    }

    #[test]
    fn padlock_solid_matches_the_approved_shape() {
        // Independent transcription of render_v2.py padlock_solid().
        let mut want = std::collections::BTreeSet::new();
        for r in [2u32, 3] {
            for c in 5..13 {
                want.insert((c, r));
            }
        }
        for r in 4..9 {
            for c in [5u32, 6, 11, 12] {
                want.insert((c, r));
            }
        }
        for r in 9..17 {
            for c in 3..15 {
                want.insert((c, r));
            }
        }
        for (c, r) in [(8, 11), (9, 11), (8, 12), (9, 12)] {
            want.remove(&(c, r));
        }
        let have = padlock_cells(Padlock::Solid);
        let have_set: std::collections::BTreeSet<_> = have.iter().copied().collect();
        assert_eq!(
            have_set, want,
            "solid padlock diverges from the approved shape"
        );
        assert_eq!(have.len(), 128, "solid padlock cell count");
        // The keyhole is cut out of the solid body.
        for (c, r) in [(8, 11), (9, 11), (8, 12), (9, 12)] {
            assert!(!have_set.contains(&(c, r)), "keyhole ({c},{r}) must be cut");
        }
    }

    #[test]
    fn padlock_hollow_matches_the_approved_shape() {
        // Independent transcription of render_v2.py padlock_hollow().
        let mut want = std::collections::BTreeSet::new();
        for c in 5..13 {
            want.insert((c, 2u32));
        }
        for r in 3..9 {
            want.insert((5, r));
            want.insert((12, r));
        }
        for c in 3..15 {
            want.insert((c, 9u32));
            want.insert((c, 16u32));
        }
        for r in 10..16 {
            want.insert((3, r));
            want.insert((14, r));
        }
        for (c, r) in [(8, 11), (9, 11), (8, 12), (9, 12)] {
            want.insert((c, r));
        }
        let have = padlock_cells(Padlock::Hollow);
        let have_set: std::collections::BTreeSet<_> = have.iter().copied().collect();
        assert_eq!(
            have_set, want,
            "hollow padlock diverges from the approved shape"
        );
        assert_eq!(have.len(), 60, "hollow padlock cell count");
        // The keyhole is lit in the hollow variant.
        for (c, r) in [(8, 11), (9, 11), (8, 12), (9, 12)] {
            assert!(have_set.contains(&(c, r)), "keyhole ({c},{r}) must be lit");
        }
    }

    fn engaged_panel(detail_rows: Vec<&str>) -> Panel {
        Panel {
            padlock: Padlock::Solid,
            padlock_color: ORANGE,
            status: [
                "LOCK: ENGAGED".into(),
                "MY GAME".into(),
                "TARGET: MON 1".into(),
            ],
            detail_rows: detail_rows.into_iter().map(String::from).collect(),
        }
    }

    #[test]
    fn layout_places_the_settled_chrome() {
        let (map, placed) = layout(&engaged_panel(vec![]));
        // The settled positions.
        assert_eq!(placed.heading, (8, 7));
        assert_eq!(placed.separators, [17, 49]);
        assert_eq!(placed.padlock, (8, 21));
        assert_eq!(placed.status, [(34, 22), (34, 31), (34, 40)]);
        assert_eq!(
            placed.buttons,
            [
                ButtonRect {
                    col: 8,
                    row: 52,
                    w: 27,
                    h: 11
                },
                ButtonRect {
                    col: 38,
                    row: 52,
                    w: 27,
                    h: 11
                },
                ButtonRect {
                    col: 68,
                    row: 52,
                    w: 27,
                    h: 11
                },
            ]
        );
        assert_eq!(placed.details.row, 68);
        assert_eq!(placed.details.col, 8);
        assert_eq!(placed.detail_origin, (8, 78));
        assert_eq!(placed.detail_step, 9);
        // The closed window is the 128×80 grid (640×400 at pitch 5).
        assert_eq!((map.cols(), map.rows()), (128, 80));
        // Heading 'M' at (8,7): row 0 "10001" lights cols 8 and 12, white.
        assert_eq!(map.get(8, 7), WHITE);
        assert_eq!(map.get(12, 7), WHITE);
        // Separator row 17 spans cols 8..=120.
        assert_eq!(map.get(8, 17), DIML);
        assert_eq!(map.get(120, 17), DIML);
        assert_eq!(map.get(7, 17), OFF, "left of the separator stays off");
        assert_eq!(map.get(121, 17), OFF, "right of the separator stays off");
        // Separator row 49.
        assert_eq!(map.get(8, 49), DIML);
        // Padlock at (8,21): shackle top-left (r=2,c=5) is orange; the
        // keyhole (r=11,c=8) is cut out of the solid body.
        assert_eq!(map.get(8 + 5, 21 + 2), ORANGE);
        assert_eq!(map.get(8 + 8, 21 + 11), OFF, "the solid keyhole is cut");
        // Status row 1 'L' at (34,22): row 0 "10000" lights col 34, orange.
        assert_eq!(map.get(34, 22), ORANGE);
        // Status rows 2 and 3 use the settled white/gray.
        assert_eq!(map.get(34, 31), WHITE);
        assert_eq!(map.get(34, 40), GREY);
        // Button frame corner (grey) and the PICK label 'P' at (10,54).
        assert_eq!(map.get(8, 52), GREY);
        assert_eq!(map.get(8 + 23 + 3, 52), GREY, "frame right edge");
        assert_eq!(map.get(10, 54), WHITE);
        // DETAILS 'D' at (8,68): row 0 "11100" lights col 8, soft.
        assert_eq!(map.get(8, 68), SOFT);
    }

    #[test]
    fn layout_places_the_detail_rows() {
        let (map, placed) = layout(&engaged_panel(vec!["FG: GAME", "REASSERTS: 3"]));
        // Detail rows from (8,78) stepping 9 (7 glyph rows + 2 blank).
        assert_eq!(placed.detail_origin, (8, 78));
        assert_eq!(placed.detail_step, 9);
        assert_eq!(map.get(8, 78), SOFT, "first detail row 'F'");
        assert_eq!(map.get(8, 87), SOFT, "second detail row 'R'");
        assert_eq!(map.get(8, 85), OFF, "blank dot-row between the lines");
        assert_eq!(map.get(8, 86), OFF, "blank dot-row between the lines");
        // The grid grows to fit the detail rows: last row ends at 87+7,
        // plus the 4-row bottom margin.
        assert_eq!(map.rows(), 87 + 7 + 4);
    }

    #[test]
    fn layout_grown_height_follows_the_detail_count() {
        let rows = |n: usize| layout(&engaged_panel(vec!["FG: A"; n])).0.rows();
        assert_eq!(rows(0), 80, "closed grid unchanged");
        assert_eq!(rows(1), 78 + 7 + 4);
        assert_eq!(rows(7), 78 + 6 * 9 + 7 + 4); // 143 rows = 715 pt
    }

    #[test]
    fn layout_detail_lines_never_overlap_and_are_not_clipped() {
        // Longest realistic rows: 20 widest glyphs each, 7 lines.
        let long = "W".repeat(40);
        let rows: Vec<String> = (0..7).map(|_| long.clone()).collect();
        let snap_rows: Vec<String> = rows.iter().map(|r| fit_width(r, 120)).collect();
        let panel = Panel {
            padlock: Padlock::Solid,
            padlock_color: ORANGE,
            status: ["A".into(), "B".into(), "C".into()],
            detail_rows: snap_rows,
        };
        let (map, placed) = layout(&panel);
        // At least one blank dot-row between consecutive lines.
        assert!(placed.detail_step > 7, "lines need a blank row between");
        // Every line's last glyph column and last glyph row are inside the
        // grid (640 pt wide = 128 cols; no clipping at right or bottom).
        assert_eq!(map.cols(), 128);
        let last_col = placed.detail_origin.0 + 6 * 19 + 4;
        assert!(last_col < map.cols(), "longest row clipped at the right");
        let last_row = placed.detail_origin.1 + 6 * placed.detail_step + 6;
        assert!(last_row < map.rows(), "last row clipped at the bottom");
        // The rightmost cell of the longest row's last glyph is lit.
        assert_ne!(map.get(last_col, placed.detail_origin.1 + 6), OFF);
        // Between lines, the gap rows stay dark.
        for i in 0..6 {
            let gap = placed.detail_origin.1 + i * placed.detail_step + 7;
            for c in 8..128 {
                assert_eq!(map.get(c, gap), OFF, "gap row {gap} col {c}");
            }
        }
    }

    #[test]
    fn layout_rows_do_not_overlap() {
        let (_, placed) = layout(&engaged_panel(vec!["FG: GAME", "REASSERTS: 3"]));
        // The 7-tall status rows keep the settled 2-empty-row gap.
        for i in 0..placed.status.len() - 1 {
            let (a, b) = (placed.status[i], placed.status[i + 1]);
            assert_eq!(b.1 - a.1, 9, "status rows must keep the 9-cell step");
            assert!(a.1 + 7 <= b.1, "status rows overlap: {a:?} {b:?}");
        }
        // DETAILS (7 tall at 68) does not overlap the first detail row (76).
        assert!(placed.details.row + placed.details.h <= placed.detail_origin.1);
        // The padlock (cols 8..25) sits clear of the status rows (col 34+).
        assert!(placed.padlock.0 + 18 <= placed.status[0].0);
        // The button frames do not overlap each other.
        for i in 0..placed.buttons.len() - 1 {
            let (a, b) = (placed.buttons[i], placed.buttons[i + 1]);
            assert!(a.col + a.w <= b.col, "button frames overlap: {a:?} {b:?}");
        }
        // The separators clear the heading (rows 7-13) and the padlock (21+).
        assert!(placed.separators[0] > 13 && placed.separators[0] < 21);
        assert!(placed.separators[1] > 46 && placed.separators[1] < 52);
    }

    #[test]
    fn layout_hollow_padlock_lights_the_keyhole() {
        let mut panel = engaged_panel(vec![]);
        panel.padlock = Padlock::Hollow;
        let (map, _) = layout(&panel);
        // Hollow: the keyhole (r=11,c=8) is lit, the body interior is not.
        assert_eq!(map.get(8 + 8, 21 + 11), ORANGE, "hollow keyhole is lit");
        assert_eq!(map.get(8 + 8, 21 + 10), OFF, "hollow body interior is dark");
        // The 1-dot outline: shackle top (r=2) and the 1-wide side (r=3, c=5)
        // are lit; the shackle interior (r=3, c=8) is dark.
        assert_eq!(map.get(8 + 5, 21 + 2), ORANGE);
        assert_eq!(map.get(8 + 5, 21 + 3), ORANGE, "hollow shackle side");
        assert_eq!(
            map.get(8 + 8, 21 + 3),
            OFF,
            "hollow shackle interior is dark"
        );
    }

    fn px(buf: &[u8], width: u32, x: u32, y: u32) -> Rgba {
        let i = ((y * width + x) * 4) as usize;
        [buf[i], buf[i + 1], buf[i + 2], buf[i + 3]]
    }

    #[test]
    fn renderer_paints_real_dots_with_antialiasing() {
        // A 2×2 cell map at pitch 10 into a 40×40 buffer: cell (0,0) lit
        // ORANGE, the rest OFF. The buffer extends past the grid, so far
        // pixels are pure BG.
        let mut map = CellMap::new(2, 2);
        map.set(0, 0, ORANGE);
        let buf = render(&map, 10, 40, 40);
        assert_eq!(buf.len(), (40 * 40 * 4) as usize);
        // The dot centre pixel is fully the lit colour.
        assert_eq!(px(&buf, 40, 5, 5), ORANGE, "lit dot centre");
        // An off-dot centre is fully the off colour.
        assert_eq!(px(&buf, 40, 15, 15), OFF, "off dot centre");
        // Far outside the grid: pure BG.
        assert_eq!(px(&buf, 40, 30, 30), BG, "no dot means BG");
        // A pixel on the dot boundary is a partial blend — anti-aliased,
        // strictly between BG and the lit colour.
        let edge = px(&buf, 40, 8, 5);
        assert_ne!(edge, ORANGE, "boundary pixel must not be fully lit");
        assert_ne!(edge, BG, "boundary pixel must not be fully BG");
        assert!(
            edge[0] > BG[0] && edge[0] < ORANGE[0],
            "boundary red {edge:?} between BG and ORANGE"
        );
    }

    #[test]
    fn renderer_paints_every_off_dot_across_the_panel() {
        // An all-off 3×3 map: every cell paints its faint dot, the gaps stay
        // BG-leaning, and nothing is lit.
        let map = CellMap::new(3, 3);
        let buf = render(&map, 10, 30, 30);
        for r in 0..3u32 {
            for c in 0..3u32 {
                assert_eq!(
                    px(&buf, 30, c * 10 + 5, r * 10 + 5),
                    OFF,
                    "off dot at ({c},{r})"
                );
            }
        }
        // Between two off-dots the blend never lights anything.
        let mid = px(&buf, 30, 10, 5);
        assert!(
            mid[0] <= OFF[0].saturating_add(1),
            "gap stays at or under off: {mid:?}"
        );
    }

    #[test]
    fn stamp_dot_composits_a_real_dot() {
        // A transparent buffer; stamp one dot and check the centre is the
        // colour and the far corner stays transparent.
        let mut buf = vec![0u8; (20 * 20 * 4) as usize];
        // radius 4.5 so the dot boundary (x=14.5) cuts through pixel 14.
        stamp_dot(&mut buf, 20, 20, 10.0, 10.0, 4.5, ORANGE);
        let c = px(&buf, 20, 10, 10);
        assert_eq!(c, ORANGE, "dot centre is the colour");
        assert_eq!(
            px(&buf, 20, 0, 0),
            [0, 0, 0, 0],
            "far corner stays transparent"
        );
        // The boundary is a partial alpha blend.
        let edge = px(&buf, 20, 14, 10);
        assert!(
            edge[3] > 0 && edge[3] < 255,
            "boundary is partial: {edge:?}"
        );
    }

    #[test]
    fn dot_at_matches_the_real_dot_shape() {
        // The cell centre is inside; the cell corner is outside.
        let p = 10.0;
        assert!(dot_at(p / 2.0, p / 2.0, p), "centre");
        assert!(!dot_at(0.0, 0.0, p), "corner");
        // The on-axis half-width is a = 0.39×pitch (the settled constant).
        assert!(
            dot_at(p / 2.0 + 0.389 * p, p / 2.0, p),
            "within 0.389×pitch"
        );
        assert!(
            !dot_at(p / 2.0 + 0.391 * p, p / 2.0, p),
            "beyond 0.391×pitch"
        );
        // On the diagonal the superellipse edge (2^(−1/4)·a) reaches further
        // than a circle of radius a (0.707·a): 0.8408·a is inside, 0.9·a is
        // outside (2·0.9⁴ > 1).
        let a = 0.39 * p;
        assert!(
            dot_at(p / 2.0 + 0.8408 * a, p / 2.0 + 0.8408 * a, p),
            "diagonal near edge"
        );
        assert!(
            !dot_at(p / 2.0 + 0.9 * a, p / 2.0 + 0.9 * a, p),
            "diagonal beyond edge"
        );
    }

    #[test]
    fn wave_intensity_matches_the_settled_formula() {
        // 1 at the padlock centre at t=0.
        assert!(
            (wave_intensity(0.0, 0.0) - 1.0).abs() < 1e-6,
            "centre at t=0 is 1"
        );
        // 0 at t=1, everywhere.
        for d in [0.0, 30.0, 70.0, 139.0] {
            assert!(wave_intensity(d, 1.0).abs() < 1e-6, "t=1 is 0 at d={d}");
        }
        // 0 beyond the front's reach (d > 140).
        assert!(wave_intensity(141.0, 0.5).abs() < 1e-6);
        assert!(wave_intensity(300.0, 0.9).abs() < 1e-6);
        // The peak sits near the moving front R(t)=140t.
        let t = 0.5;
        let front = 140.0 * t;
        let mut best_d = 0.0;
        let mut best_i = f32::MIN;
        let mut d = 0.0;
        while d <= 140.0 {
            let i = wave_intensity(d, t);
            if i > best_i {
                best_i = i;
                best_d = d;
            }
            d += 0.5;
        }
        assert!(
            (best_d - front).abs() <= 20.0,
            "peak at {best_d}, front at {front}"
        );
        // It beats points well behind and ahead of the front, and stays in
        // [0, 1].
        assert!(best_i > wave_intensity(20.0, t));
        assert!(best_i > wave_intensity(130.0, t));
        for d in [0.0, 10.0, 35.0, 70.0, 100.0, 139.0] {
            let i = wave_intensity(d, t);
            assert!((0.0..=1.0 + 1e-6).contains(&i), "d={d}: {i} out of [0,1]");
        }
    }

    fn ring_bounds() -> Rect {
        Rect {
            x: 0,
            y: 0,
            width: 400,
            height: 300,
        }
    }

    #[test]
    fn ring_length_matches_the_rounded_rect_formula() {
        const INSET: f32 = 24.0;
        const R: f32 = 24.0;
        let b = ring_bounds();
        // The ring's bounding box: the monitor bounds inset 24 on every side.
        let w = b.width as f32 - 2.0 * INSET;
        let h = b.height as f32 - 2.0 * INSET;
        // Independent expectation: two horizontal edges, two vertical edges,
        // and four quarter arcs (one full circle).
        let expected = 2.0 * (w - 2.0 * R) + 2.0 * (h - 2.0 * R) + 2.0 * std::f32::consts::PI * R;
        let have = ring_length(&b);
        assert!(
            (have - expected).abs() < 1e-3,
            "ring length {have} != rounded-rect expectation {expected}"
        );
    }

    #[test]
    fn ring_point_is_periodic_and_opposite_sides() {
        let b = ring_bounds();
        let l = ring_length(&b);
        // Periodic: s and s+L are the same point.
        for s in [0.0, 100.0, l * 0.3, l * 0.7] {
            let (x1, y1) = ring_point(&b, s);
            let (x2, y2) = ring_point(&b, s + l);
            assert!(
                (x1 - x2).abs() < 1e-2 && (y1 - y2).abs() < 1e-2,
                "not periodic at s={s}"
            );
        }
        // s and s+L/2 are on opposite sides of the shape (the rounded rect is
        // point-symmetric, so antipodal in arc length is antipodal in space).
        let (cx, cy) = ((b.x + b.width / 2) as f32, (b.y + b.height / 2) as f32);
        for s in [0.0, 50.0, l * 0.25, l * 0.6] {
            let (x1, y1) = ring_point(&b, s);
            let (x2, y2) = ring_point(&b, s + l / 2.0);
            let dot = (x1 - cx) * (x2 - cx) + (y1 - cy) * (y2 - cy);
            assert!(dot < 0.0, "s={s}: not opposite sides (dot={dot})");
        }
    }

    #[test]
    fn ring_point_starts_at_the_top_edge_and_walks_clockwise() {
        let b = ring_bounds();
        let l = ring_length(&b);
        // Inset 24 and radius 24: the ring box is (24,24)-(376,276), so the
        // top edge starts at (x0+24, y0) = (48, 24).
        let (x, y) = ring_point(&b, 0.0);
        assert!(
            (x - 48.0).abs() < 1e-3 && (y - 24.0).abs() < 1e-3,
            "s=0 is ({x},{y})"
        );
        // A little along s: still on the top edge, moving right.
        let (x, y) = ring_point(&b, 10.0);
        assert!(
            (x - 58.0).abs() < 1e-3 && (y - 24.0).abs() < 1e-3,
            "s=10 is ({x},{y})"
        );
        // Three quarters around: with the 400×300 bounds (top=304, side=204,
        // arc≈37.7, L≈1166.8) that lands on the bottom edge walking left:
        // y = 276, x between the bottom corners.
        let (x, y) = ring_point(&b, 3.0 * l / 4.0);
        assert!(
            (y - 276.0).abs() < 1e-3,
            "3/4 around is on the bottom edge, y={y}"
        );
        assert!(
            x > 48.0 && x < 352.0,
            "3/4 around is between the bottom corners, x={x}"
        );
    }

    #[test]
    fn state_key_precedence_is_picking_over_engaged_over_suspended_over_released() {
        use crate::core;
        let mut snap = core::LockSnapshot::default();
        assert_eq!(state_key(&snap), StateKey::Released);

        // A pin with the Lock released is suspended (the pin outlives a
        // focus loss).
        snap.pinned = Some(core::MonitorTarget {
            monitor_id: r"\\.\DISPLAY1".into(),
            monitor_bounds: Rect {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
        });
        assert_eq!(state_key(&snap), StateKey::Suspended);

        // An engaged Lock beats the pin (start_picker keeps the pin).
        snap.lock = core::LockState::Engaged {
            window_identity: 1,
            window_title: "My Game".into(),
            monitor_id: r"\\.\DISPLAY1".into(),
            monitor_bounds: Rect {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
        };
        assert_eq!(state_key(&snap), StateKey::Engaged);

        // Picking beats everything: the Lock is released while picking, but
        // the key is still Picking (the padlock renders released-white).
        snap.picking = true;
        assert_eq!(state_key(&snap), StateKey::Picking);
    }

    fn window(title: &str) -> crate::seam::WindowInfo {
        crate::seam::WindowInfo {
            identity: 1,
            title: title.into(),
            bounds: Rect {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
        }
    }

    fn game(title: &str, monitor_id: &str) -> crate::core::DetectedGameWindow {
        crate::core::DetectedGameWindow {
            identity: 1,
            title: title.into(),
            game_monitor_id: monitor_id.into(),
            game_monitor_bounds: Rect {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
        }
    }

    fn monitor(id: &str, w: i32, h: i32) -> crate::seam::MonitorInfo {
        crate::seam::MonitorInfo {
            id: id.into(),
            bounds: Rect {
                x: 0,
                y: 0,
                width: w,
                height: h,
            },
        }
    }

    #[test]
    fn panel_content_gives_the_status_lines_per_state() {
        use crate::core;
        let snap = core::LockSnapshot {
            foreground: Some(window("My Game")),
            game_window: Some(game("My Game", "MON1")),
            ..Default::default()
        };

        // Titles keep their case here; the dot font uppercases at render.
        let p = panel_content(&snap, StateKey::Released, false);
        assert_eq!(p.padlock, Padlock::Solid);
        assert_eq!(p.padlock_color, WHITE);
        assert_eq!(p.status, ["LOCK: RELEASED", "My Game", "TARGET: MON1"]);
        assert!(p.detail_rows.is_empty());

        let p = panel_content(&snap, StateKey::Engaged, false);
        assert_eq!(p.padlock, Padlock::Solid);
        assert_eq!(p.padlock_color, ORANGE);
        assert_eq!(p.status, ["LOCK: ENGAGED", "My Game", "TARGET: MON1"]);

        let p = panel_content(&snap, StateKey::Suspended, false);
        assert_eq!(p.padlock, Padlock::Hollow);
        assert_eq!(p.padlock_color, ORANGE);
        assert_eq!(p.status, ["LOCK: SUSPENDED", "My Game", "TARGET: MON1"]);

        let p = panel_content(&snap, StateKey::Picking, false);
        assert_eq!(p.padlock, Padlock::Solid);
        assert_eq!(p.padlock_color, WHITE);
        assert_eq!(
            p.status,
            ["PICKING MONITOR", "CLICK TO PIN", "ESC TO CANCEL"]
        );
    }

    #[test]
    fn panel_content_without_a_game_window_says_cursor_free() {
        // No Game window, no pin: the cursor is simply free, the target is
        // none.
        let snap = crate::core::LockSnapshot::default();
        let p = panel_content(&snap, StateKey::Released, false);
        assert_eq!(p.status, ["LOCK: RELEASED", "CURSOR FREE", "TARGET: NONE"]);
    }

    #[test]
    fn panel_content_detail_rows_only_while_open_and_in_order() {
        use crate::core;
        let mut snap = core::LockSnapshot {
            foreground: Some(window("My Game")),
            game_window: Some(game("My Game", "MON1")),
            monitors: vec![monitor("MON1", 1920, 1080), monitor("MON2", 1280, 1024)],
            clip_reasserts: 3,
            ..Default::default()
        };

        let p = panel_content(&snap, StateKey::Engaged, false);
        assert!(p.detail_rows.is_empty(), "DETAILS closed: no rows");

        let p = panel_content(&snap, StateKey::Engaged, true);
        assert_eq!(
            p.detail_rows,
            [
                "FG: My Game",
                "GAME: My Game",
                "MONITORS: 2",
                "MON1 1920x1080",
                "MON2 1280x1024",
                "CLIP REASSERTS: 3",
            ]
        );

        // Zero re-asserts: no row for them.
        snap.clip_reasserts = 0;
        let p = panel_content(&snap, StateKey::Engaged, true);
        assert_eq!(p.detail_rows.len(), 5);
        assert!(!p.detail_rows.iter().any(|r| r.contains("REASSERTS")));
    }

    #[test]
    fn panel_content_fits_the_rows_to_the_cell_budget() {
        use crate::core;
        let snap = core::LockSnapshot {
            foreground: Some(window("ABCDEFGHIJKLMNOPQRSTUVWXYZ")),
            game_window: Some(game("Game", "MON1")),
            ..Default::default()
        };
        // The 94-cell status row (col 34): 15 chars fit.
        let p = panel_content(&snap, StateKey::Engaged, false);
        assert_eq!(p.status[1], "ABCDEFGHIJKLMNO");
        // The 120-cell detail row (col 8): 20 chars fit.
        let p = panel_content(&snap, StateKey::Engaged, true);
        assert_eq!(p.detail_rows[0], "FG: ABCDEFGHIJKLMNOP");
    }

    #[test]
    fn foreground_line_reports_the_title_or_a_placeholder() {
        use crate::core;
        let none = core::LockSnapshot::default();
        assert_eq!(foreground_line(&none), "NONE");
        let empty = core::LockSnapshot {
            foreground: Some(window("")),
            ..Default::default()
        };
        assert_eq!(foreground_line(&empty), "NO TITLE");
        let titled = core::LockSnapshot {
            foreground: Some(window("My Game")),
            ..Default::default()
        };
        assert_eq!(foreground_line(&titled), "My Game");
    }

    #[test]
    fn target_line_prefers_the_pin_over_the_game_monitor() {
        use crate::core;
        let mut snap = core::LockSnapshot::default();
        assert_eq!(target_line(&snap), "TARGET: NONE");
        snap.game_window = Some(game("Game", "MON1"));
        assert_eq!(target_line(&snap), "TARGET: MON1");
        // The pin outlives a focus loss: while armed it names the target.
        snap.pinned = Some(core::MonitorTarget {
            monitor_id: "MON2".into(),
            monitor_bounds: Rect {
                x: 0,
                y: 0,
                width: 1280,
                height: 1024,
            },
        });
        assert_eq!(target_line(&snap), "TARGET: MON2");
    }

    #[test]
    fn blend_toward_interpolates_rgb_and_keeps_the_source_alpha() {
        let a: Rgba = [10, 20, 30, 255];
        let b: Rgba = [110, 120, 230, 1];
        assert_eq!(
            blend_toward(a, b, 0.0),
            [10, 20, 30, 255],
            "t=0 is the source"
        );
        assert_eq!(
            blend_toward(a, b, 1.0),
            [110, 120, 230, 255],
            "t=1 is the target's rgb, the source's alpha"
        );
        assert_eq!(blend_toward(a, b, 0.5), [60, 70, 130, 255], "the midpoint");
    }

    #[test]
    fn dim_is_sixty_percent_toward_the_off_dot() {
        // 60% of the way WHITE -> OFF: 232->116, 232->117, 236->122.
        assert_eq!(dim(WHITE), [116, 117, 122, 255]);
        // And from GREY: 136->77/78/82.
        assert_eq!(dim(GREY), [77, 78, 82, 255]);
        // Already at the target: unchanged.
        assert_eq!(dim(OFF), OFF);
    }

    #[test]
    fn fit_width_truncates_to_the_cell_budget() {
        // n characters occupy 6n-1 cells: 1->5, 2->11, 3->17.
        assert_eq!(fit_width("AB", 5), "A");
        assert_eq!(fit_width("AB", 10), "A"); // 2 chars need 11
        assert_eq!(fit_width("ABC", 11), "AB"); // exactly 2 fit in 11
        assert_eq!(fit_width("ABCD", 17), "ABC"); // exactly 3 fit in 17
        assert_eq!(fit_width("ABCD", 16), "AB"); // 16 falls short of 17
        assert_eq!(fit_width("", 100), "");
        // The 94-cell status row (col 34 of the 128-col grid): 15 chars fit
        // (6·15−1 = 89); 16 would need 95.
        assert_eq!(fit_width("TARGET: DISPLAY1", 94), "TARGET: DISPLAY");
        // The 120-cell detail row (col 8): exactly 20 chars fit (6·20−1 =
        // 119).
        assert_eq!(
            fit_width("ABCDEFGHIJKLMNOPQRST", 120),
            "ABCDEFGHIJKLMNOPQRST"
        );
        assert_eq!(
            fit_width("ABCDEFGHIJKLMNOPQRSTUVWXYZ", 120),
            "ABCDEFGHIJKLMNOPQRST"
        );
    }

    #[test]
    fn cell_at_maps_pointer_points_to_grid_cells() {
        let p = 5.0;
        assert_eq!(cell_at(0.0, 0.0, p, p, 128, 80), Some((0, 0)));
        assert_eq!(cell_at(2.5 * p, 0.5 * p, p, p, 128, 80), Some((2, 0)));
        assert_eq!(cell_at(2.99 * p, 79.99 * p, p, p, 128, 80), Some((2, 79)));
        // The grid's right/bottom edges are outside.
        assert_eq!(cell_at(128.0 * p, 0.0, p, p, 128, 80), None);
        assert_eq!(cell_at(0.0, 80.0 * p, p, p, 128, 80), None);
        // Negative (outside the panel's top-left) is outside.
        assert_eq!(cell_at(-0.1, 0.0, p, p, 128, 80), None);
        assert_eq!(cell_at(0.0, -0.1, p, p, 128, 80), None);
    }

    #[test]
    fn cell_in_rect_matches_the_button_rects() {
        let pick = ButtonRect {
            col: 8,
            row: 52,
            w: 27,
            h: 11,
        };
        assert!(cell_in_rect((8, 52), &pick));
        assert!(cell_in_rect((8 + 26, 52 + 10), &pick));
        assert!(!cell_in_rect((8 + 27, 52), &pick)); // right edge exclusive
        assert!(!cell_in_rect((8, 52 + 11), &pick)); // bottom edge exclusive
        assert!(!cell_in_rect((7, 52), &pick));
        assert!(!cell_in_rect((35, 52), &pick)); // the gap between the frames
    }

    #[test]
    fn wave_progress_counts_to_the_one_shot_end() {
        assert_eq!(wave_progress(0, WAVE_MS), Some(0.0));
        assert_eq!(wave_progress(125, WAVE_MS), Some(0.5));
        assert_eq!(wave_progress(WAVE_MS, WAVE_MS), None); // finished at t=1
        assert_eq!(wave_progress(WAVE_MS + 50, WAVE_MS), None);
    }

    #[test]
    fn chaser_phase_wraps_on_the_6_second_orbit() {
        // Ticket #10: the chasers orbit with a 6 s period, so the phase is
        // (uptime / 6) mod 1, in [0, 1), from uptime on the current monitor.
        assert!((chaser_phase(0.0) - 0.0).abs() < 1e-6, "t=0 is phase 0");
        assert!(
            (chaser_phase(3.0) - 0.5).abs() < 1e-6,
            "half a period is 0.5"
        );
        assert!(
            (chaser_phase(6.0) - 0.0).abs() < 1e-6,
            "one full period wraps to 0"
        );
        assert!((chaser_phase(9.0) - 0.5).abs() < 1e-6, "1.5 periods is 0.5");
        // A long picker session must not drift: 1000 orbits plus half.
        assert!((chaser_phase(6003.0) - 0.5).abs() < 1e-4, "6003 s is 0.5");
        // Always in [0, 1), increasing within one period.
        for i in 0..12 {
            let p = chaser_phase(i as f32);
            assert!((0.0..1.0).contains(&p), "t={i}: phase {p} out of range");
        }
        assert!(
            chaser_phase(1.0) < chaser_phase(2.0) && chaser_phase(2.0) < chaser_phase(5.0),
            "phase must increase within a period"
        );
    }

    #[test]
    fn ring_dot_count_closes_the_ring_at_14_px_spacing() {
        // 400x300 bounds: ring box (24,24)-(376,276), L = 2*304 + 2*204 +
        // 2*pi*24 ~= 1166.796, so L/14 ~= 83.34 -> 83 dots.
        let b = ring_bounds();
        let n = ring_dot_count(&b);
        assert_eq!(n, 83, "400x300 bounds");
        let achieved = ring_length(&b) / n as f32;
        assert!((achieved - 14.0).abs() < 0.2, "achieved spacing {achieved}");
        // A real monitor: 1920x1080 -> top=1824, side=984, L ~= 5766.796,
        // L/14 ~= 411.91 -> 412 dots.
        let m = Rect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        };
        let nm = ring_dot_count(&m);
        assert_eq!(nm, 412, "1920x1080 bounds");
        let achieved = ring_length(&m) / nm as f32;
        assert!((achieved - 14.0).abs() < 0.2, "achieved spacing {achieved}");
    }
}
