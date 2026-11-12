//! The app icon: a monitor whose screen shows the Lock state. Released =
//! dark gray screen; engaged = amber screen with a padlock. Pure pixel art
//! (no Win32), so it is drawn at any size and unit-tested.
//!
//! One source for every place the icon appears: the tray (swapped on state
//! change), the window's title bar/taskbar icon, and res/icon.ico (the exe
//! icon, embedded by build.rs — the engaged art). The test
//! `ico_file_matches_the_art` fails if res/icon.ico drifts from this code;
//! regenerate it with `MONITOR_LOCK_WRITE_ICO=1 cargo test ico_file`.

/// Amber: the engaged screen (same amber the UI uses for pin hints).
const AMBER: [u8; 3] = [255, 191, 64];
/// Dark gray: the released screen.
const SCREEN_OFF: [u8; 3] = [52, 53, 60];
/// The bezel and the padlock body: dark slate.
const BEZEL: [u8; 3] = [88, 90, 110];
/// The stand's neck, a shade lighter than the bezel.
const NECK: [u8; 3] = [104, 106, 128];
/// The padlock's shackle and keyhole: light gray.
const LIGHT: [u8; 3] = [236, 236, 240];

/// The sizes packed into res/icon.ico (Explorer, taskbar and Alt+Tab pick
/// the best fit per DPI and view).
#[cfg(test)]
pub const ICO_SIZES: [u32; 6] = [16, 24, 32, 48, 64, 256];

/// The icon as `size`x`size` top-down RGBA pixels, transparent background,
/// anti-aliased by 4x4 supersampling.
pub fn rgba(size: u32, engaged: bool) -> Vec<u8> {
    const SS: u32 = 4;
    let mut out = vec![0u8; (size * size * 4) as usize];
    for y in 0..size {
        for x in 0..size {
            // Premultiplied sum over the subsamples.
            let (mut r, mut g, mut b, mut a) = (0u32, 0u32, 0u32, 0u32);
            for sy in 0..SS {
                for sx in 0..SS {
                    let u = (x as f32 + (sx as f32 + 0.5) / SS as f32) / size as f32;
                    let v = (y as f32 + (sy as f32 + 0.5) / SS as f32) / size as f32;
                    if let Some(c) = color_at(u, v, engaged) {
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

/// The color at normalized point (u, v) in 0..1, or None (transparent).
/// Shapes are painted back to front: the topmost hit wins.
fn color_at(u: f32, v: f32, engaged: bool) -> Option<[u8; 3]> {
    // Screen area (inside the bezel).
    if in_round_rect(u, v, 0.10, 0.15, 0.90, 0.63, 0.015) {
        if engaged {
            return Some(padlock_at(u, v).unwrap_or(AMBER));
        }
        return Some(SCREEN_OFF);
    }
    // Bezel.
    if in_round_rect(u, v, 0.02, 0.07, 0.98, 0.76, 0.07) {
        return Some(BEZEL);
    }
    // Neck: a trapezoid widening downward.
    if (0.76..0.87).contains(&v) {
        let t = (v - 0.76) / 0.11;
        if (u - 0.5).abs() < 0.10 + 0.04 * t {
            return Some(NECK);
        }
    }
    // Base.
    if in_round_rect(u, v, 0.26, 0.86, 0.74, 0.93, 0.02) {
        return Some(BEZEL);
    }
    None
}

/// The padlock on the engaged screen, centered in it.
fn padlock_at(u: f32, v: f32) -> Option<[u8; 3]> {
    // Keyhole: a dot and a slot, cut out of the body (shows the screen).
    let keyhole = dist(u, v, 0.5, 0.465) < 0.028
        || ((u - 0.5).abs() < 0.013 && (0.465..0.52).contains(&v));
    if in_round_rect(u, v, 0.37, 0.385, 0.63, 0.575, 0.025) {
        return if keyhole { Some(LIGHT) } else { Some(BEZEL) };
    }
    // Shackle: an arch over the body, with short straight legs.
    let (cx, cy, r_out, r_in) = (0.5, 0.30, 0.10, 0.058);
    let shackle = if v < cy {
        (r_in..r_out).contains(&dist(u, v, cx, cy))
    } else {
        v < 0.39 && (r_in..r_out).contains(&(u - cx).abs())
    };
    shackle.then_some(LIGHT)
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
        .map(|&s| (s, bmp_image(s, &rgba(s, true))))
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
    fn the_states_differ_only_on_the_screen() {
        let (on, off) = (rgba(32, true), rgba(32, false));
        assert_ne!(on, off);
        // The outline (alpha) is the same monitor in both states.
        let alpha = |p: &[u8]| p.chunks(4).map(|c| c[3]).collect::<Vec<_>>();
        assert_eq!(alpha(&on), alpha(&off));
    }
}
