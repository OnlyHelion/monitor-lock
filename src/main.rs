//! Monitor Lock — keeps the cursor confined to the Game monitor while a
//! Game window (the focused window that fills its monitor) has focus.
//!
//! Structure (one seam): the pure brain (core.rs) sits behind the SystemAccess
//! seam (seam.rs). The thin layer (win32.rs, tray, window) implements the seam
//! with real Win32 calls and is verified by the user's in-game hand-check, not
//! by unit tests.

#[cfg(test)]
mod fake;
mod core;
mod seam;

#[cfg(windows)]
mod win32;

fn main() {
    todo!("tray, on-demand window, and poll thread")
}
