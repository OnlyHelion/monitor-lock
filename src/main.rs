//! Monitor Lock — keeps the cursor confined to the Game monitor while a
//! Game window (the focused window that fills its monitor) has focus.
//!
//! Architecture (one seam): the pure brain (core.rs) sits behind the
//! SystemAccess seam (seam.rs). This file owns the thin layer: the tray icon,
//! the on-demand window, and the ~100 ms poll thread that reads the seam
//! (win32.rs), feeds observations to the brain, and executes the brain's
//! actions. The thin layer has no unit tests; it is verified by the user's
//! in-game hand-check (ticket #5).

#[cfg(test)]
mod fake;
mod core;
mod seam;
#[cfg(windows)]
mod win32;

#[cfg(windows)]
mod app;

#[cfg(windows)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Per-monitor v2 DPI awareness: first statement, before any window or
    // tray creation (spec; user story 21).
    win32::set_per_monitor_dpi_awareness();

    // The tray icon is created on the main thread; eframe's event loop runs
    // on that same thread and pumps its messages. It must stay alive for the
    // whole run — dropping it would destroy the icon.
    let _tray = app::create_tray()?;

    // Poll thread: reads the SystemAccess seam, feeds the brain, publishes
    // the snapshot for the window. ~100 ms (spec); the tighter ~50 ms cadence
    // while the Monitor picker is active comes with ticket #4.
    let shared: std::sync::Arc<std::sync::Mutex<core::LockSnapshot>> =
        std::sync::Arc::new(std::sync::Mutex::new(core::LockSnapshot::default()));
    let poll_shared = shared.clone();
    std::thread::spawn(move || {
        let sys = win32::Win32System;
        let brain = core::LockCore::new();
        loop {
            let snap = brain.observe(&sys);
            *poll_shared.lock().unwrap() = snap;
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    });

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Monitor Lock")
            .with_inner_size([380.0, 240.0])
            .with_visible(false), // on-demand window: starts hidden
        ..Default::default()
    };
    eframe::run_native(
        "Monitor Lock",
        options,
        Box::new(move |_cc| Ok(Box::new(app::MonitorLockApp { shared }) as _)),
    )?;
    Ok(())
}
