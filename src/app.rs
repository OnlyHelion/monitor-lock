//! Thin-layer UI: the tray icon and the on-demand egui window. No unit tests
//! (spec: the thin layer is verified by the user's in-game hand-check).

use std::sync::{Arc, Mutex};
use std::time::Duration;

use eframe::egui;
use tray_icon::menu::{Menu, MenuEvent, MenuItem};
use tray_icon::{Icon, MouseButton, TrayIcon, TrayIconBuilder, TrayIconEvent};

use crate::core;

/// Create the tray icon and its menu. Menu ids are the command vocabulary
/// drained by `drain_tray_events`; ticket #4 adds the picker commands.
pub fn create_tray() -> Result<TrayIcon, tray_icon::Error> {
    let open = MenuItem::with_id("open", "Open", true, None);
    let exit = MenuItem::with_id("exit", "Exit", true, None);
    let menu = Menu::new();
    menu.append_items(&[&open, &exit]).expect("static menu items always append");
    TrayIconBuilder::new()
        .with_icon(lock_icon())
        .with_tooltip("Monitor Lock")
        .with_menu(Box::new(menu))
        .with_menu_on_left_click(false) // left-click opens the window, not the menu
        .build()
}

/// The on-demand window. Reads the shared snapshot (written by the poll
/// thread) and drains tray events; user commands become tray events too.
pub struct MonitorLockApp {
    pub(crate) shared: Arc<Mutex<core::LockSnapshot>>,
}

impl eframe::App for MonitorLockApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // The window is on-demand: a close request hides it; the app lives in
        // the tray. Scope the ctx uses before the panel takes `ui`.
        {
            let ctx = ui.ctx();
            if ctx.input(|i| i.viewport().close_requested()) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            }
            drain_tray_events(&ctx);
        }

        let snap = self.shared.lock().unwrap().clone();
        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("Monitor Lock");
            ui.separator();

            ui.label(format!("Foreground window: {}", foreground_label(&snap)));

            ui.label(format!("Monitors: {}", snap.monitors.len()));
            for m in &snap.monitors {
                ui.label(format!(
                    "  {}  ({}, {}) {}x{}",
                    m.id, m.bounds.x, m.bounds.y, m.bounds.width, m.bounds.height
                ));
            }

            ui.separator();
            match &snap.game_window {
                Some(g) => {
                    ui.label(egui::RichText::new(format!(
                        "Fills monitor (Game window): yes — {}",
                        g.game_monitor_id
                    ))
                    .color(egui::Color32::from_rgb(80, 200, 120)))
                }
                None => {
                    ui.label(egui::RichText::new("Fills monitor (Game window): no")
                        .color(egui::Color32::GRAY))
                }
            }
        });

        // Live-updates: repaint on the poll cadence even without input.
        ui.ctx().request_repaint_after(Duration::from_millis(100));
    }
}

/// The foreground window line: title, or a fallback for the desktop and for
/// windows that report no title (some games).
fn foreground_label(snap: &core::LockSnapshot) -> String {
    match &snap.foreground {
        None => "(desktop — no foreground window)".to_string(),
        Some(w) if w.title.is_empty() => "(window with no title)".to_string(),
        Some(w) => w.title.clone(),
    }
}

/// Drain tray events on the UI thread: left-click opens the on-demand window;
/// the menu items are commands.
fn drain_tray_events(ctx: &egui::Context) {
    while let Ok(event) = TrayIconEvent::receiver().try_recv() {
        if let TrayIconEvent::Click {
            button: MouseButton::Left,
            ..
        } = event
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        }
    }
    while let Ok(event) = MenuEvent::receiver().try_recv() {
        match event.id.0.as_str() {
            "open" => ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true)),
            "exit" => {
                // No clip to clear yet — the cursor-locking actions are inert
                // stubs in this ticket (ticket #3).
                std::process::exit(0);
            }
            _ => {}
        }
    }
}

/// A simple generated padlock icon (32x32 RGBA, transparent background) —
/// "default icon for now" (spec): custom icon art is out of scope.
fn lock_icon() -> Icon {
    const S: u32 = 32;
    let mut rgba = vec![0u8; (S * S * 4) as usize];
    for y in 0..S {
        for x in 0..S {
            // Pixel-center coordinates with the grid origin (16, 16).
            let px = (x as f32 + 0.5) - 16.0;
            let py = (y as f32 + 0.5) - 16.0;
            // Body: the lower block of the padlock.
            let body = (8..=23).contains(&x) && (16..=28).contains(&y);
            // Shackle: the upper ring (outer r 9, inner r 5.5), centered at
            // (0, -4), top half only.
            let d = (px * px + (py + 4.0) * (py + 4.0)).sqrt();
            let shackle = (5.5..=9.0).contains(&d) && y < 16;
            // Keyhole: a small dot in the body.
            let keyhole = (px * px + (py - 4.0) * (py - 4.0)).sqrt() < 2.5;
            if (body || shackle) && !keyhole {
                let i = ((y * S + x) as usize) * 4;
                rgba[i] = 255; // amber
                rgba[i + 1] = 191;
                rgba[i + 2] = 64;
                rgba[i + 3] = 255;
            }
        }
    }
    Icon::from_rgba(rgba, S, S).expect("a 32x32 RGBA buffer is a valid icon")
}
