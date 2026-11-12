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
mod overlay;

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
    // the snapshot for the window, reports Lock-state changes to the UI
    // thread, and drives the Monitor picker's overlay. ~100 ms (spec); ~50
    // ms while the picker is active.
    let (state_tx, state_rx) = std::sync::mpsc::channel::<bool>();
    let (cmd_tx, cmd_rx) = std::sync::mpsc::channel::<app::UiCommand>();
    let shared: std::sync::Arc<std::sync::Mutex<core::LockSnapshot>> =
        std::sync::Arc::new(std::sync::Mutex::new(core::LockSnapshot::default()));
    let poll_shared = shared.clone();
    std::thread::spawn(move || {
        let mut sys = win32::Win32System;
        let mut brain = core::LockCore::new();
        let mut overlay = overlay::PickerOverlay::new();
        // Startup: clear any stale clip left by a previous run (user story
        // 20 — the clip is global Windows state).
        brain.startup(&mut sys);
        // The tray starts with the released icon, so only report when the
        // engaged state actually changes.
        let mut last_engaged: Option<bool> = Some(false);
        // Rising-edge debounce for the picker's Esc / click (a held key or
        // button must not fire again after the picker restarts).
        let mut esc_down = false;
        let mut click_down = false;
        loop {
            // User commands from the tray menu / window buttons beat the
            // tick.
            let mut command = false;
            while let Ok(cmd) = cmd_rx.try_recv() {
                command = true;
                match cmd {
                    app::UiCommand::StartPicker => brain.start_picker(&mut sys),
                    app::UiCommand::BackToAuto => brain.back_to_auto(),
                }
            }
            if command {
                // A command may have ended the picker: drop the debounce
                // state so a still-held key or button can't fire again.
                esc_down = false;
                click_down = false;
            }

            let snap = brain.update(&mut sys);
            let engaged = matches!(snap.lock, core::LockState::Engaged { .. });
            if last_engaged != Some(engaged) {
                let _ = state_tx.send(engaged);
                last_engaged = Some(engaged);
            }
            *poll_shared.lock().unwrap() = snap.clone();

            if snap.picking {
                // Outline the hovered monitor; hide the overlay when the
                // cursor is over no monitor.
                match &snap.picker_hover {
                    Some(hover) => overlay.outline(&hover.bounds),
                    None => overlay.destroy(),
                }
                // Rising edge of Esc or the left mouse button.
                let esc = win32::esc_down();
                let click = win32::left_click_down();
                let esc_edge = esc && !esc_down;
                let click_edge = click && !click_down;
                if click_edge || esc_edge {
                    if click_edge {
                        // A click pins the monitor under the cursor.
                        brain.confirm_picker(&mut sys);
                    } else {
                        // Esc cancels without pinning.
                        brain.cancel_picker();
                    }
                    overlay.destroy();
                }
                esc_down = esc;
                click_down = click;
            } else {
                // No picker: the overlay is down (idempotent, cheap when
                // idle) and the debounce state is reset.
                overlay.destroy();
                esc_down = false;
                click_down = false;
            }

            std::thread::sleep(std::time::Duration::from_millis(if snap.picking {
                50
            } else {
                100
            }));
        }
    });

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Monitor Lock")
            .with_inner_size([380.0, 340.0])
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
            cmd_tx,
        }) as _)),
    )?;
    Ok(())
}
