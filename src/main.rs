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

    // The tray icon is created on the main thread and owned by the eframe
    // app: the tray's message window is pumped by the UI thread, so the
    // TrayIcon handle must live there (it is not Send). It starts in the
    // released state; the poll thread reports state changes and the UI
    // thread swaps the icon (user story 14).
    let icons = app::TrayIcons::new();
    let tray = app::create_tray(&icons.released)?;

    // Poll thread: reads the SystemAccess seam, feeds the brain, publishes
    // the snapshot for the window, and reports Lock-state changes to the UI
    // thread. ~100 ms (spec); the tighter ~50 ms cadence while the Monitor
    // picker is active comes with ticket #4.
    let (state_tx, state_rx) = std::sync::mpsc::channel::<bool>();
    let shared: std::sync::Arc<std::sync::Mutex<core::LockSnapshot>> =
        std::sync::Arc::new(std::sync::Mutex::new(core::LockSnapshot::default()));
    let poll_shared = shared.clone();
    std::thread::spawn(move || {
        let mut sys = win32::Win32System;
        let mut brain = core::LockCore::new();
        // Startup: clear any stale clip left by a previous run (user story
        // 20 — the clip is global Windows state).
        brain.startup(&mut sys);
        // The tray starts with the released icon, so only report when the
        // engaged state actually changes.
        let mut last_engaged: Option<bool> = Some(false);
        loop {
            let snap = brain.update(&mut sys);
            let engaged = matches!(snap.lock, core::LockState::Engaged { .. });
            if last_engaged != Some(engaged) {
                let _ = state_tx.send(engaged);
                last_engaged = Some(engaged);
            }
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
        Box::new(move |_cc| Ok(Box::new(app::MonitorLockApp {
            shared,
            tray,
            icons,
            state_rx,
        }) as _)),
    )?;
    Ok(())
}
