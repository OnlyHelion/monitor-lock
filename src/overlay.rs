//! The Monitor picker's overlay: a hand-rolled Win32 layered window that
//! draws the dot-matrix picker ring around the FULL bounds (rcMonitor) of
//! the monitor under the cursor: a dotted #FFB347 ring (inset 24 px, corner
//! radius 24 px, dots radius 5 px on 14 px arc spacing) with two chasers
//! (radius 7 px) orbiting on opposite sides, each with two fading tail dots
//! 14/28 px behind at alpha 102/51 (ticket #10).
//!
//! Transparent, topmost, and input-transparent (WS_EX_LAYERED +
//! WS_EX_TRANSPARENT + WS_EX_TOPMOST + WS_EX_TOOLWINDOW, WS_POPUP,
//! SWP_NOACTIVATE): it never steals a click or focus — the picker's click is
//! resolved from the cursor position, not through the overlay (spec: Monitor
//! picker). It is driven from the poll thread, not from egui.
//!
//! The chasers orbit with a 6 s period, quantized to the 50 ms picker tick
//! (ADR 0001); the phase runs from the uptime on the CURRENT monitor and
//! resets when the hovered monitor changes. The DIB buffer is kept across
//! ticks and recreated only when the monitor changes (ticket #10).
//!
//! Thin layer — no unit tests (the ring/chaser math is tested in
//! dotmatrix); verified by the user's in-game hand-check (ticket #10).

use std::time::Instant;

use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    AC_SRC_ALPHA, AC_SRC_OVER, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION, CreateCompatibleDC,
    CreateDIBSection, DIB_RGB_COLORS, DeleteDC, DeleteObject, HBITMAP, HDC, HGDIOBJ, SelectObject,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, HWND_TOPMOST, RegisterClassW, SWP_NOACTIVATE,
    SWP_SHOWWINDOW, SetWindowPos, ULW_ALPHA, UpdateLayeredWindow, WNDCLASSW, WS_EX_LAYERED,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};
use windows::core::PCWSTR;

use crate::dotmatrix;
use crate::seam::Rect;

const CLASS_NAME: &str = "MonitorLockPickerOverlay";

static CLASS_REGISTERED: std::sync::Once = std::sync::Once::new();

/// The overlay window, its kept DIB buffer, and the chaser phase origin.
/// Created on the first `outline`, destroyed on picker end or exit.
pub struct PickerOverlay {
    hwnd: HWND,
    /// The bounds the ring is currently drawn around; None = idle (no window).
    current: Option<Rect>,
    /// The DIB the ring is painted into: kept across ticks, recreated only
    /// when the hovered monitor changes. GDI-allocated — freed only via
    /// DeleteObject/DeleteDC, never a Rust deallocator.
    dc: HDC,
    bitmap: HBITMAP,
    pixels: *mut u8,
    w: i32,
    h: i32,
    /// When the overlay started on the CURRENT monitor: the chaser phase
    /// origin (uptime on this monitor, not the wall clock).
    up_since: Option<Instant>,
}

impl PickerOverlay {
    pub fn new() -> Self {
        Self {
            hwnd: HWND::default(),
            current: None,
            dc: HDC::default(),
            bitmap: HBITMAP::default(),
            pixels: core::ptr::null_mut(),
            w: 0,
            h: 0,
            up_since: None,
        }
    }

    /// Draw the dotted ring + chasers around the monitor's FULL bounds.
    /// Repaints on EVERY call (the chasers orbit, driven at the 50 ms picker
    /// tick); the window moves and the DIB is recreated only when the bounds
    /// change, and the chaser phase origin resets then too.
    pub fn outline(&mut self, bounds: &Rect) {
        let changed = self.current != Some(*bounds) || self.hwnd.is_invalid();
        if changed {
            if self.hwnd.is_invalid() {
                self.hwnd = create(bounds);
            } else {
                let _ = unsafe {
                    SetWindowPos(
                        self.hwnd,
                        Some(HWND_TOPMOST),
                        bounds.x,
                        bounds.y,
                        bounds.width,
                        bounds.height,
                        SWP_NOACTIVATE | SWP_SHOWWINDOW,
                    )
                };
            }
            self.recreate_dib(bounds);
            self.up_since = Some(Instant::now());
            self.current = Some(*bounds);
        }
        self.paint(bounds);
    }

    /// Destroy the overlay window and free the DIB (picker end or exit).
    /// Idempotent and cheap when idle — the poll loop calls it every tick
    /// while the picker is not active. (No UnregisterClassW: at exit the
    /// process dies, and the class table goes with it.)
    pub fn destroy(&mut self) {
        if !self.hwnd.is_invalid() {
            unsafe {
                let _ = DestroyWindow(self.hwnd);
            }
            self.hwnd = HWND::default();
        }
        self.free_dib();
        self.current = None;
        self.up_since = None;
    }

    /// Free the DIB (GDI memory — DeleteObject/DeleteDC only).
    fn free_dib(&mut self) {
        if !self.pixels.is_null() {
            unsafe {
                let _ = DeleteObject(HGDIOBJ(self.bitmap.0));
                let _ = DeleteDC(self.dc);
            }
            self.dc = HDC::default();
            self.bitmap = HBITMAP::default();
            self.pixels = core::ptr::null_mut();
            self.w = 0;
            self.h = 0;
        }
    }

    /// (Re)create the DIB at the monitor's full-bounds size and select it
    /// into its DC for UpdateLayeredWindow.
    fn recreate_dib(&mut self, bounds: &Rect) {
        self.free_dib();
        let w = bounds.width;
        let h = bounds.height;
        if w <= 0 || h <= 0 {
            return;
        }
        unsafe {
            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: core::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: w,
                    biHeight: -h, // negative = top-down
                    biPlanes: 1,
                    biBitCount: 32,
                    ..Default::default()
                },
                ..Default::default()
            };
            let dc = CreateCompatibleDC(None);
            let mut pixels: *mut core::ffi::c_void = core::ptr::null_mut();
            let bitmap = CreateDIBSection(
                Some(dc),
                &bmi as *const BITMAPINFO,
                DIB_RGB_COLORS,
                &mut pixels as *mut *mut core::ffi::c_void,
                None,
                0,
            )
            .expect("the overlay's ARGB bitmap is created");
            let _ = SelectObject(dc, HGDIOBJ(bitmap.0));
            self.dc = dc;
            self.bitmap = bitmap;
            self.pixels = pixels as *mut u8;
            self.w = w;
            self.h = h;
        }
    }

    /// Paint the current frame into the kept DIB and push it to the layered
    /// window. The buffer is cleared to fully transparent, the dotted ring is
    /// stamped, then the two chasers with their fading tails (dotmatrix does
    /// the geometry).
    fn paint(&mut self, bounds: &Rect) {
        if self.pixels.is_null() || self.w <= 0 || self.h <= 0 {
            return;
        }
        let (w, h) = (self.w, self.h);
        let l = dotmatrix::ring_length(bounds);
        let uptime = self.up_since.map_or(0.0, |since| since.elapsed().as_secs_f32());
        let phase = dotmatrix::chaser_phase(uptime);
        let buf = unsafe {
            core::slice::from_raw_parts_mut(self.pixels, (w as usize) * (h as usize) * 4)
        };
        buf.fill(0); // fully transparent
        // The dotted ring: n dots closing the arc at ~14 px spacing.
        let n = dotmatrix::ring_dot_count(bounds);
        for i in 0..n {
            let (x, y) = dotmatrix::ring_point(bounds, l * i as f32 / n as f32);
            dotmatrix::stamp_dot(
                buf,
                w as u32,
                h as u32,
                x,
                y,
                dotmatrix::RING_DOT_RADIUS,
                dotmatrix::ORANGE,
            );
        }
        // The two chasers, opposite each other, each with two tail dots
        // trailing behind (ring_point wraps the negative offsets).
        for k in 0..2u32 {
            let s = phase * l + l * k as f32 / 2.0;
            for &(behind, alpha) in &dotmatrix::CHASER_TAILS {
                let (x, y) = dotmatrix::ring_point(bounds, s - behind);
                let tail: dotmatrix::Rgba = [
                    dotmatrix::ORANGE[0],
                    dotmatrix::ORANGE[1],
                    dotmatrix::ORANGE[2],
                    alpha,
                ];
                dotmatrix::stamp_dot(
                    buf,
                    w as u32,
                    h as u32,
                    x,
                    y,
                    dotmatrix::RING_DOT_RADIUS,
                    tail,
                );
            }
            let (x, y) = dotmatrix::ring_point(bounds, s);
            dotmatrix::stamp_dot(
                buf,
                w as u32,
                h as u32,
                x,
                y,
                dotmatrix::CHASER_RADIUS,
                dotmatrix::ORANGE,
            );
        }
        self.push(bounds);
    }

    /// Push the DIB to the layered window with per-pixel alpha (ULW_ALPHA).
    fn push(&self, bounds: &Rect) {
        unsafe {
            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };
            let dst = POINT {
                x: bounds.x,
                y: bounds.y,
            };
            let size = SIZE {
                cx: self.w,
                cy: self.h,
            };
            let src = POINT { x: 0, y: 0 };
            let _ = UpdateLayeredWindow(
                self.hwnd,
                None,
                Some(&dst as *const POINT),
                Some(&size as *const SIZE),
                Some(self.dc),
                Some(&src as *const POINT),
                COLORREF(0),
                Some(&blend as *const BLENDFUNCTION),
                ULW_ALPHA,
            );
        }
    }
}

impl Drop for PickerOverlay {
    fn drop(&mut self) {
        self.destroy();
    }
}

/// The overlay's WndProc: nothing to handle — the window never takes focus
/// (WS_POPUP + WS_EX_TOOLWINDOW + SWP_NOACTIVATE) and is painted directly by
/// UpdateLayeredWindow, so no message pump is required.
unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

/// Register the overlay's window class once. RegisterClassW copies the class
/// name; the WndProc is a plain DefWindowProcW pass-through.
fn ensure_class() {
    CLASS_REGISTERED.call_once(|| unsafe {
        let name = wide(CLASS_NAME);
        let instance =
            GetModuleHandleW(None).expect("the module handle is valid in a running process");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wnd_proc),
            hInstance: instance.into(),
            lpszClassName: PCWSTR(name.as_ptr()),
            ..Default::default()
        };
        let _ = RegisterClassW(&wc);
    });
}

/// Create the input-transparent, topmost, borderless overlay window over the
/// monitor's FULL bounds (physical pixels under PMv2).
fn create(bounds: &Rect) -> HWND {
    ensure_class();
    let class_name = wide(CLASS_NAME);
    unsafe {
        let instance =
            GetModuleHandleW(None).expect("the module handle is valid in a running process");
        CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
            PCWSTR(class_name.as_ptr()),
            PCWSTR::null(),
            WS_POPUP,
            bounds.x,
            bounds.y,
            bounds.width,
            bounds.height,
            None,
            None,
            Some(instance.into()),
            None,
        )
        .expect("the picker overlay window is created")
    }
}

/// A null-terminated wide string.
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
