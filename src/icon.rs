//! The app icon: the dot-matrix padlock panel and nothing else — a dark
//! rounded tile of 18×18 dots with the padlock lit: solid orange when
//! engaged, solid white when released, hollow orange when suspended. No monitor silhouette. Pure pixel art (no Win32), so it is drawn at any size and
//! unit-tested.
//!
//! One source for every place the icon appears: the tray (swapped on state
//! change), the window's title bar/taskbar icon, and res/icon.ico (the exe
//! icon, embedded by build.rs — the engaged art). The dot panel reuses the
//! design language from dotmatrix.rs — the same 18×18 padlock cells and the
//! same "real dot" superellipse the window renders — so the icon can never
//! drift from the window. The test `ico_file_matches_the_art` fails if
//! res/icon.ico drifts from this code; regenerate it with
//! `MONITOR_LOCK_WRITE_ICO=1 cargo test ico_file`.

use crate::dotmatrix;

/// Panel background and off-dot, from the one source (dotmatrix.rs) — the
/// same palette the window renders.
const PANEL_BG: [u8; 3] = [dotmatrix::BG[0], dotmatrix::BG[1], dotmatrix::BG[2]];
const PANEL_OFF: [u8; 3] = [dotmatrix::OFF[0], dotmatrix::OFF[1], dotmatrix::OFF[2]];
/// The padlock, engaged (locked orange).
const LOCKED: [u8; 3] = [
    dotmatrix::ORANGE[0],
    dotmatrix::ORANGE[1],
    dotmatrix::ORANGE[2],
];
/// The padlock, released (white).
const FREE: [u8; 3] = [
    dotmatrix::WHITE[0],
    dotmatrix::WHITE[1],
    dotmatrix::WHITE[2],
];
/// The tile: the dark panel covering the whole icon, rounded corners
/// (transparent outside). The 18×18 dot grid is a centred square inside it.
const TILE_RADIUS: f32 = 0.14;
const PANEL_SIDE: f32 = 0.88; // the grid's side
const PANEL_LEFT: f32 = 0.5 - PANEL_SIDE / 2.0; // centred
const PANEL_TOP: f32 = PANEL_LEFT;

/// The sizes packed into res/icon.ico (Explorer, taskbar and Alt+Tab pick
/// the best fit per DPI and view).
#[cfg(test)]
pub const ICO_SIZES: [u32; 6] = [16, 24, 32, 48, 64, 256];

/// The three icon arts: the same tile and off-dots, only the padlock differs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Art {
    /// Solid orange padlock (the exe icon too).
    Engaged,
    /// Solid white padlock.
    Released,
    /// Hollow orange padlock: a pin is set but the Lock is released.
    Suspended,
}

impl Art {
    /// Every art, in `index` order.
    pub const ALL: [Art; 3] = [Art::Engaged, Art::Released, Art::Suspended];

    /// The icon for a window state. Picking paints the window's padlock solid
    /// white (dotmatrix::panel_content), so it shares the released art.
    pub fn for_state(key: dotmatrix::StateKey) -> Art {
        use dotmatrix::StateKey;
        match key {
            StateKey::Engaged => Art::Engaged,
            StateKey::Suspended => Art::Suspended,
            StateKey::Released | StateKey::Picking => Art::Released,
        }
    }

    /// Position in `ALL`, for per-art caches.
    pub fn index(self) -> usize {
        self as usize
    }
}

/// The icon as `size`x`size` top-down RGBA pixels, transparent background,
/// anti-aliased by 4x4 supersampling.
pub fn rgba(size: u32, art: Art) -> Vec<u8> {
    const SS: u32 = 4;
    let panel = panel_cells(art);
    let mut out = vec![0u8; (size * size * 4) as usize];
    for y in 0..size {
        for x in 0..size {
            // Premultiplied sum over the subsamples.
            let (mut r, mut g, mut b, mut a) = (0u32, 0u32, 0u32, 0u32);
            for sy in 0..SS {
                for sx in 0..SS {
                    let u = (x as f32 + (sx as f32 + 0.5) / SS as f32) / size as f32;
                    let v = (y as f32 + (sy as f32 + 0.5) / SS as f32) / size as f32;
                    if let Some(c) = color_at(u, v, &panel) {
                        r += u32::from(c[0]);
                        g += u32::from(c[1]);
                        b += u32::from(c[2]);
                        a += 1;
                    }
                }
            }
            if a == 0 {
                continue; // fully transparent
            }
            let i = ((y * size + x) * 4) as usize;
            out[i] = (r / a) as u8;
            out[i + 1] = (g / a) as u8;
            out[i + 2] = (b / a) as u8;
            out[i + 3] = ((a * 255 + SS * SS / 2) / (SS * SS)) as u8;
        }
    }
    out
}

/// The color at normalized point (u, v) in 0..1, or None (transparent
/// outside the tile's rounded corners).
fn color_at(u: f32, v: f32, panel: &[[u8; 3]; 18 * 18]) -> Option<[u8; 3]> {
    if in_round_rect(u, v, 0.0, 0.0, 1.0, 1.0, TILE_RADIUS) {
        return Some(panel_at(u, v, panel));
    }
    None
}

/// The 18×18 panel's cell colours, row-major: the padlock lit (solid orange
/// engaged, solid white released, hollow orange suspended), every other cell the off-dot colour. The padlock
/// cells come from the one source (dotmatrix.rs) — the same glyph the window
/// stamps.
fn panel_cells(art: Art) -> [[u8; 3]; 18 * 18] {
    let (lit, glyph) = match art {
        Art::Engaged => (LOCKED, dotmatrix::Padlock::Solid),
        Art::Released => (FREE, dotmatrix::Padlock::Solid),
        Art::Suspended => (LOCKED, dotmatrix::Padlock::Hollow),
    };
    let mut cells = [PANEL_OFF; 18 * 18];
    for (col, row) in dotmatrix::padlock_cells(glyph) {
        cells[(row * 18 + col) as usize] = lit;
    }
    cells
}

/// The tile's colour at (u, v): the panel background with an 18×18 grid of
/// "real dots" (the same superellipse dotmatrix.rs renders) — the padlock
/// lit, every other dot off. The grid is a centred square.
fn panel_at(u: f32, v: f32, panel: &[[u8; 3]; 18 * 18]) -> [u8; 3] {
    const N: u32 = 18;
    if u < PANEL_LEFT
        || u >= PANEL_LEFT + PANEL_SIDE
        || v < PANEL_TOP
        || v >= PANEL_TOP + PANEL_SIDE
    {
        return PANEL_BG; // the tile margin around the grid
    }
    let pitch = PANEL_SIDE / N as f32;
    let col = (((u - PANEL_LEFT) / pitch) as u32).min(N - 1);
    let row = (((v - PANEL_TOP) / pitch) as u32).min(N - 1);
    let ox = u - (PANEL_LEFT + col as f32 * pitch);
    let oy = v - (PANEL_TOP + row as f32 * pitch);
    if dotmatrix::dot_at(ox, oy, pitch) {
        panel[(row * N + col) as usize]
    } else {
        PANEL_BG
    }
}

fn dist(u: f32, v: f32, cx: f32, cy: f32) -> f32 {
    ((u - cx) * (u - cx) + (v - cy) * (v - cy)).sqrt()
}

/// Inside the rect (x0, y0)-(x1, y1) with corner radius r.
fn in_round_rect(u: f32, v: f32, x0: f32, y0: f32, x1: f32, y1: f32, r: f32) -> bool {
    if u < x0 || u > x1 || v < y0 || v > y1 {
        return false;
    }
    let cx = u.clamp(x0 + r, x1 - r);
    let cy = v.clamp(y0 + r, y1 - r);
    dist(u, v, cx, cy) <= r
}

/// res/icon.ico's bytes: every `ICO_SIZES` image of the engaged art, each an
/// uncompressed 32bpp BMP (bottom-up BGRA + an all-zero AND mask).
#[cfg(test)]
pub fn ico_bytes() -> Vec<u8> {
    let images: Vec<(u32, Vec<u8>)> = ICO_SIZES
        .iter()
        .map(|&s| (s, bmp_image(s, &rgba(s, Art::Engaged))))
        .collect();
    let mut out = Vec::new();
    // ICONDIR: reserved, type 1 (icon), count.
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&(images.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * images.len() as u32;
    for (s, data) in &images {
        let dim = if *s >= 256 { 0 } else { *s as u8 }; // 0 means 256
        out.extend_from_slice(&[dim, dim, 0, 0]);
        out.extend_from_slice(&1u16.to_le_bytes()); // planes
        out.extend_from_slice(&32u16.to_le_bytes()); // bpp
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&offset.to_le_bytes());
        offset += data.len() as u32;
    }
    for (_, data) in &images {
        out.extend_from_slice(data);
    }
    out
}

#[cfg(test)]
fn bmp_image(s: u32, rgba: &[u8]) -> Vec<u8> {
    let mask_row = s.div_ceil(32) * 4; // 1bpp rows, padded to 4 bytes
    let mut out = Vec::new();
    // BITMAPINFOHEADER (height doubled: XOR image + AND mask).
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&(s as i32).to_le_bytes());
    out.extend_from_slice(&(2 * s as i32).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&32u16.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // BI_RGB
    out.extend_from_slice(&(s * s * 4 + mask_row * s).to_le_bytes());
    out.extend_from_slice(&[0u8; 16]); // ppm x/y, colors used/important
    for y in (0..s).rev() {
        for x in 0..s {
            let i = ((y * s + x) * 4) as usize;
            out.extend_from_slice(&[rgba[i + 2], rgba[i + 1], rgba[i], rgba[i + 3]]);
        }
    }
    out.extend(std::iter::repeat_n(0u8, (mask_row * s) as usize));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ico_file_matches_the_art() {
        let want = ico_bytes();
        if std::env::var_os("MONITOR_LOCK_WRITE_ICO").is_some() {
            std::fs::write("res/icon.ico", &want).expect("write res/icon.ico");
        }
        let have = std::fs::read("res/icon.ico").expect("res/icon.ico is checked in");
        assert!(
            have == want,
            "res/icon.ico is out of date: run `MONITOR_LOCK_WRITE_ICO=1 cargo test ico_file`"
        );
    }

    #[test]
    fn the_icon_art_follows_the_full_state() {
        use crate::dotmatrix::StateKey;
        assert_eq!(Art::for_state(StateKey::Engaged), Art::Engaged);
        assert_eq!(Art::for_state(StateKey::Suspended), Art::Suspended);
        assert_eq!(Art::for_state(StateKey::Released), Art::Released);
        // Picking paints the window's padlock solid white: the icon matches.
        assert_eq!(Art::for_state(StateKey::Picking), Art::Released);
    }

    #[test]
    fn suspended_is_the_hollow_orange_padlock_on_the_same_tile() {
        const S: u32 = 256;
        const N: u32 = 18;
        let pitch = PANEL_SIDE / N as f32;
        let (eng, rel, sus) = (
            rgba(S, Art::Engaged),
            rgba(S, Art::Released),
            rgba(S, Art::Suspended),
        );
        let px = |(col, row): (u32, u32), img: &[u8]| -> [u8; 3] {
            let x = ((PANEL_LEFT + (col as f32 + 0.5) * pitch) * S as f32) as u32;
            let y = ((PANEL_TOP + (row as f32 + 0.5) * pitch) * S as f32) as u32;
            let i = ((y * S + x) * 4) as usize;
            [img[i], img[i + 1], img[i + 2]]
        };
        let alpha = |p: &[u8]| p.chunks(4).map(|c| c[3]).collect::<Vec<_>>();
        assert_eq!(alpha(&sus), alpha(&eng), "same tile outline");
        let hollow = dotmatrix::padlock_cells(dotmatrix::Padlock::Hollow);
        let solid = dotmatrix::padlock_cells(dotmatrix::Padlock::Solid);
        // An outline cell is lit orange (also in the solid art).
        let outline = *hollow.iter().find(|c| solid.contains(c)).unwrap();
        assert_eq!(px(outline, &sus), [255, 179, 71]);
        // A body cell inside the outline is unlit in suspended, lit when engaged.
        let inside = *solid.iter().find(|c| !hollow.contains(c)).unwrap();
        assert_eq!(px(inside, &sus), [38, 40, 46], "hollow: interior is off");
        assert_eq!(px(inside, &eng), [255, 179, 71]);
        assert_eq!(px(inside, &rel), [232, 232, 236]);
        // Off-dots and margin identical to the other states.
        assert_eq!(px((0, 0), &sus), [38, 40, 46]);
        assert_ne!(sus, eng);
        assert_ne!(sus, rel);
    }

    #[test]
    fn the_states_share_one_outline() {
        let (on, off) = (rgba(32, Art::Engaged), rgba(32, Art::Released));
        assert_ne!(on, off);
        // The tile's outline (alpha) is the same in both states.
        let alpha = |p: &[u8]| p.chunks(4).map(|c| c[3]).collect::<Vec<_>>();
        assert_eq!(alpha(&on), alpha(&off));
    }

    #[test]
    fn the_states_differ_only_in_the_padlock_colour() {
        // Panel background and off-dots paint identically in both states;
        // every differing pixel is a lit padlock dot (orange vs white).
        const S: u32 = 256;
        let (on, off) = (rgba(S, Art::Engaged), rgba(S, Art::Released));
        let mut differing = 0;
        for i in (0..on.len()).step_by(4) {
            if on[i..i + 4] != off[i..i + 4] {
                differing += 1;
                // Dot edges blend with the tile, so compare channels: orange
                // is redder and less blue than white at the same coverage.
                assert!(on[i] > off[i], "engaged padlock is orange, released white");
                assert!(
                    on[i + 2] < off[i + 2],
                    "engaged padlock is orange, released white"
                );
            }
        }
        assert!(differing > 0);
    }

    #[test]
    fn the_icon_is_the_dot_panel_tile_with_no_monitor() {
        // The dot panel fills the whole icon: a dark tile with rounded
        // corners (transparent outside), no bezel, neck or base.
        const S: u32 = 256;
        let img = rgba(S, Art::Engaged);
        let at = |u: f32, v: f32| {
            let i = (((v * S as f32) as u32 * S + (u * S as f32) as u32) * 4) as usize;
            [img[i], img[i + 1], img[i + 2], img[i + 3]]
        };
        assert_eq!(at(0.0, 0.0)[3], 0, "rounded corner is transparent");
        assert_eq!(at(0.995, 0.995)[3], 0, "rounded corner is transparent");
        // Where the old monitor had bezel, neck, base and the gap beside them.
        assert_eq!(at(0.03, 0.5), [14, 15, 19, 255], "tile margin, not bezel");
        assert_eq!(at(0.5, 0.995 - 0.01), [14, 15, 19, 255], "tile, not base");
        assert_eq!(at(0.1, 0.95), [14, 15, 19, 255], "tile, not transparent");
        // No slate silhouette: the old bezel and neck leaned blue; tile,
        // off-dots and the orange padlock (and their blends) never do.
        for p in img.chunks(4).filter(|p| p[3] > 0) {
            assert!(
                u16::from(p[2]) <= u16::from(p[0]) + 8,
                "bluish pixel {:?} is a leftover bezel",
                &p[..3]
            );
        }
    }

    #[test]
    fn the_icon_is_the_18x18_dot_panel() {
        // The panel is a centred square filling the tile, 18×18 cells.
        const S: u32 = 256;
        const N: u32 = 18;
        let pitch = PANEL_SIDE / N as f32;
        let (engaged, released) = (rgba(S, Art::Engaged), rgba(S, Art::Released));
        let px = |u: f32, v: f32, img: &[u8]| -> [u8; 3] {
            let x = (u * S as f32) as u32;
            let y = (v * S as f32) as u32;
            let i = ((y * S + x) * 4) as usize;
            [img[i], img[i + 1], img[i + 2]]
        };
        let centre = |row: u32, col: u32| {
            (
                PANEL_LEFT + (col as f32 + 0.5) * pitch,
                PANEL_TOP + (row as f32 + 0.5) * pitch,
            )
        };
        // A padlock body cell (not the keyhole): the solid padlock lit
        // locked-orange engaged, white released.
        let (u, v) = centre(13, 8);
        assert_eq!(
            px(u, v, &engaged),
            [255, 179, 71],
            "padlock lit orange when engaged"
        );
        assert_eq!(
            px(u, v, &released),
            [232, 232, 236],
            "padlock lit white when released"
        );
        // An off-dot cell (the padlock starts at row 2): the off-dot colour in
        // both states.
        let (u, v) = centre(0, 0);
        assert_eq!(px(u, v, &engaged), [38, 40, 46]);
        assert_eq!(px(u, v, &released), [38, 40, 46]);
        // The keyhole is cut out of the padlock: off, not lit.
        let (u, v) = centre(11, 8);
        assert_eq!(px(u, v, &engaged), [38, 40, 46], "keyhole is cut out");
        // The panel background: the tile margin around the grid, and the gap
        // between two dots.
        assert_eq!(px(0.02, 0.5, &engaged), [14, 15, 19], "tile margin");
        let (u, v) = (PANEL_LEFT + pitch, PANEL_TOP + 0.5 * pitch);
        assert_eq!(px(u, v, &engaged), [14, 15, 19], "gap between dots");
    }
}
