//! The Monitor picker's overlay: a hand-rolled Win32 layered window that
//! draws a thin yellow border around the FULL bounds (rcMonitor) of the
//! monitor under the cursor.
//!
//! Transparent, topmost, and input-transparent (WS_EX_LAYERED +
//! WS_EX_TRANSPARENT + WS_EX_TOPMOST + WS_EX_TOOLWINDOW, WS_POPUP,
//! SWP_NOACTIVATE): it never steals a click or focus — the picker's click is
//! resolved from the cursor position, not through the overlay (spec: Monitor
//! picker). It is driven from the poll thread, not from egui.
//!
//! Thin layer — no unit tests; verified by the user's in-game hand-check
//! (ticket #5).

use windows::core::PCWSTR;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, SelectObject, BLENDFUNCTION,
    BITMAPINFO, BITMAPINFOHEADER, AC_SRC_ALPHA, AC_SRC_OVER, DIB_RGB_COLORS, HGDIOBJ,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, RegisterClassW, SetWindowPos,
    UpdateLayeredWindow, HWND_TOPMOST, SWP_NOACTIVATE, SWP_SHOWWINDOW, ULW_ALPHA, WS_EX_LAYERED,
    WS_EX_TOPMOST, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT, WS_POPUP, WNDCLASSW,
};

use crate::seam::Rect;

/// Border thickness in physical pixels — the "thin yellow border" (spec).
const BORDER_PX: i32 = 5;
/// Opaque yellow (255, 255, 0, 255).
const YELLOW: (u8, u8, u8) = (255, 255, 0);

const CLASS_NAME: &str = "MonitorLockPickerOverlay";

static CLASS_REGISTERED: std::sync::Once = std::sync::Once::new();

/// The overlay window handle and the bounds the border is currently drawn
/// around. Created on the first `outline`, destroyed on picker end or exit.
pub struct PickerOverlay {
    hwnd: HWND,
    /// None = idle (no window).
    current: Option<Rect>,
}

impl PickerOverlay {
    pub fn new() -> Self {
        Self {
            hwnd: HWND::default(),
            current: None,
        }
    }

    /// Draw a thin yellow border around the monitor's FULL bounds. Creates
    /// the window on the first call; move + repaint happen only when the
    /// bounds change (repaint only when the hovered monitor changes).
    pub fn outline(&mut self, bounds: &Rect) {
        if self.current == Some(*bounds) && !self.hwnd.is_invalid() {
            return;
        }
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
        paint(self.hwnd, bounds);
        self.current = Some(*bounds);
    }

    /// Destroy the overlay window (picker end or exit). Idempotent and cheap
    /// when idle — the poll loop calls it every tick while the picker is not
    /// active. (No UnregisterClassW: at exit the process dies, and the class
    /// table goes with it.)
    pub fn destroy(&mut self) {
        if !self.hwnd.is_invalid() {
            unsafe {
                let _ = DestroyWindow(self.hwnd);
            }
            self.hwnd = HWND::default();
        }
        self.current = None;
    }
}

/// The overlay's WndProc: nothing to handle — the window never takes focus
/// (WS_POPUP + WS_EX_TOOLWINDOW + SWP_NOACTIVATE) and is painted directly by
/// UpdateLayeredWindow, so no message pump is required.
unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

/// Register the overlay's window class once. RegisterClassW copies the class
/// name; the WndProc is a plain DefWindowProcW pass-through.
fn ensure_class() {
    CLASS_REGISTERED.call_once(|| unsafe {
        let name = wide(CLASS_NAME);
        let instance = GetModuleHandleW(None).expect("the module handle is valid in a running process");
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
        let instance = GetModuleHandleW(None)
            .expect("the module handle is valid in a running process");
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

/// Paint the border: a top-down 32-bit ARGB buffer, all transparent, with a
/// BORDER_PX ring of opaque yellow around the edge, pushed to the layered
/// window with per-pixel alpha (ULW_ALPHA).
fn paint(hwnd: HWND, bounds: &Rect) {
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
        // Fill: everything transparent, the BORDER_PX ring opaque yellow.
        let buf =
            core::slice::from_raw_parts_mut(pixels as *mut u8, (w as usize) * (h as usize) * 4);
        for row in 0..h {
            for col in 0..w {
                if col < BORDER_PX
                    || col >= w - BORDER_PX
                    || row < BORDER_PX
                    || row >= h - BORDER_PX
                {
                    let i = ((row as usize) * (w as usize) + (col as usize)) * 4;
                    buf[i] = YELLOW.0;
                    buf[i + 1] = YELLOW.1;
                    buf[i + 2] = YELLOW.2;
                    buf[i + 3] = 255;
                }
            }
        }
        let old = SelectObject(dc, HGDIOBJ(bitmap.0));
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
        let size = SIZE { cx: w, cy: h };
        let src = POINT { x: 0, y: 0 };
        let _ = UpdateLayeredWindow(
            hwnd,
            None,
            Some(&dst as *const POINT),
            Some(&size as *const SIZE),
            Some(dc),
            Some(&src as *const POINT),
            COLORREF(0),
            Some(&blend as *const BLENDFUNCTION),
            ULW_ALPHA,
        );
        let _ = SelectObject(dc, old);
        let _ = DeleteObject(HGDIOBJ(bitmap.0));
        let _ = DeleteDC(dc);
    }
}

/// A null-terminated wide string.
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
