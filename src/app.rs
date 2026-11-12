//! Thin-layer UI: the tray icon and the on-demand egui window. No unit tests
//! (spec: the thin layer is verified by the user's in-game hand-check).

use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use eframe::egui;
use tray_icon::menu::{Menu, MenuEvent, MenuItem};
use tray_icon::{Icon, MouseButton, TrayIcon, TrayIconBuilder, TrayIconEvent};

use crate::core;
use crate::seam::Rect;
use crate::win32;

/// The two pre-built tray icons: one per Lock state, swapped on state change
/// so the state is visible at a glance (user story 14).
pub struct TrayIcons {
    /// Engaged: amber — the cursor is confined to the Game monitor.
    pub engaged: Icon,
    /// Released: gray — the cursor is free on all monitors.
    pub released: Icon,
}

impl TrayIcons {
    pub fn new() -> Self {
        Self {
            engaged: padlock_icon((255, 191, 64)),
            released: padlock_icon((136, 136, 136)),
        }
    }
}

/// A user command from the UI (tray menu or window button) to the poll
/// thread, which applies it to the brain. Small and `Copy`: one mpsc sender
/// is shared by both.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiCommand {
    /// Start the Monitor picker (explicit user command only — the picker
    /// never starts on its own; idempotent while already picking).
    StartPicker,
    /// Back to auto: clear the Monitor override, cancel an active picker,
    /// and resume following the Game monitor.
    BackToAuto,
}

/// Create the tray icon and its menu. Menu ids are the command vocabulary
/// drained by `drain_tray_events`: Open, Pick monitor, Back to auto, Exit.
pub fn create_tray(initial_icon: &Icon) -> Result<TrayIcon, tray_icon::Error> {
    let open = MenuItem::with_id("open", "Open", true, None);
    let pick = MenuItem::with_id("pick", "Pick monitor", true, None);
    let auto = MenuItem::with_id("auto", "Back to auto", true, None);
    let exit = MenuItem::with_id("exit", "Exit", true, None);
    let menu = Menu::new();
    menu
        .append_items(&[&open, &pick, &auto, &exit])
        .expect("static menu items always append");
    TrayIconBuilder::new()
        .with_icon(initial_icon.clone())
        .with_tooltip("Monitor Lock")
        .with_menu(Box::new(menu))
        .with_menu_on_left_click(false) // left-click opens the window, not the menu
        .build()
}

/// The on-demand window. Reads the shared snapshot (written by the poll
/// thread) and drains tray events; user commands become tray events too.
/// Owns the tray icon: the tray's message window is pumped by this thread,
/// so the icon swap (user story 14) happens here, fed by state changes the
/// poll thread reports over `state_rx`.
pub struct MonitorLockApp {
    pub(crate) shared: Arc<Mutex<core::LockSnapshot>>,
    pub(crate) tray: TrayIcon,
    pub(crate) icons: TrayIcons,
    pub(crate) state_rx: mpsc::Receiver<bool>,
    /// Sends user commands (tray menu / window buttons) to the poll thread.
    pub(crate) cmd_tx: mpsc::Sender<UiCommand>,
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
            drain_tray_events(&ctx, &self.cmd_tx);
        }
        // Lock-state changes reported by the poll thread: swap the tray icon.
        while let Ok(engaged) = self.state_rx.try_recv() {
            let icon = if engaged {
                self.icons.engaged.clone()
            } else {
                self.icons.released.clone()
            };
            let _ = self.tray.set_icon(Some(icon));
        }

        let snap = self.shared.lock().unwrap().clone();
        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("Monitor Lock");
            ui.separator();

            ui.label(format!("Foreground window: {}", foreground_label(&snap)));
            ui.label(format!("Game window: {}", game_window_label(&snap)));

            ui.label(format!("Monitors: {}", snap.monitors.len()));
            for m in &snap.monitors {
                ui.label(format!("  {}", format_monitor(&m.id, &m.bounds)));
            }

            if snap.picking {
                ui.label(egui::RichText::new(
                    "Monitor picker: move to a monitor — click to pin, Esc to cancel",
                )
                .color(egui::Color32::from_rgb(255, 255, 0)));
            }

            ui.separator();
            match &snap.lock {
                core::LockState::Released => {
                    ui.label(egui::RichText::new("Lock: released")
                        .color(egui::Color32::from_rgb(136, 136, 136)));
                }
                core::LockState::Engaged { window_title, .. } => {
                    ui.label(egui::RichText::new(format!(
                        "Lock: engaged — {}",
                        if window_title.is_empty() {
                            "window with no title"
                        } else {
                            window_title
                        }
                    ))
                    .color(egui::Color32::from_rgb(80, 200, 120)));
                }
            }

            // The current target: the pinned monitor (Monitor override) if
            // any, else the Game monitor. Shown while the Lock is released
            // too — a pin outlives a focus loss.
            if let Some(pinned) = &snap.pinned {
                ui.label(format!(
                    "Target monitor (pinned): {}",
                    format_monitor(&pinned.monitor_id, &pinned.monitor_bounds)
                ));
                // Pin mode engages on ANY focused window (no fills-monitor
                // requirement); the desktop is the release.
                ui.label("Pinned: engages while any window has focus — click the desktop or Back to auto to release");
            } else if let Some(game) = &snap.game_window {
                ui.label(format!(
                    "Target monitor (auto): {}",
                    format_monitor(&game.game_monitor_id, &game.game_monitor_bounds)
                ));
            } else {
                ui.label("Target monitor: — (no game window)");
            }

            ui.separator();
            ui.horizontal(|ui| {
                let pick = ui.add_enabled(!snap.picking, egui::Button::new("Pick monitor"));
                let auto = ui.button("Back to auto");
                let exit = ui.button("Exit");
                if pick.clicked() {
                    let _ = self.cmd_tx.send(UiCommand::StartPicker);
                }
                if auto.clicked() {
                    let _ = self.cmd_tx.send(UiCommand::BackToAuto);
                }
                if exit.clicked() {
                    exit_app();
                }
            });
        });

        // Live-updates: repaint on the poll cadence even without input.
        ui.ctx().request_repaint_after(Duration::from_millis(100));
    }
}

/// Exit: the cursor clip is global Windows state that survives process
/// death (user story 20), so clear it before exiting.
fn exit_app() -> ! {
    win32::clear_clip();
    std::process::exit(0);
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

/// The detected Game window line (user story 15): the Game window's title,
/// or "none" when the foreground window is not a Game window.
fn game_window_label(snap: &core::LockSnapshot) -> String {
    match &snap.game_window {
        None => "none".to_string(),
        Some(g) if g.title.is_empty() => "(window with no title)".to_string(),
        Some(g) => g.title.clone(),
    }
}

/// One monitor's id and FULL bounds (x, y, w, h) for the on-demand window.
fn format_monitor(id: &str, bounds: &Rect) -> String {
    format!("{}  ({}, {}) {}x{}", id, bounds.x, bounds.y, bounds.width, bounds.height)
}

/// Drain tray events on the UI thread: left-click opens the on-demand window;
/// the menu items are commands (`pick` / `auto` are forwarded to the poll
/// thread, which applies them to the brain).
fn drain_tray_events(ctx: &egui::Context, cmd_tx: &mpsc::Sender<UiCommand>) {
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
            "pick" => {
                let _ = cmd_tx.send(UiCommand::StartPicker);
            }
            "auto" => {
                let _ = cmd_tx.send(UiCommand::BackToAuto);
            }
            "exit" => {
                exit_app();
            }
            _ => {}
        }
    }
}

/// A simple generated padlock icon (32x32 RGBA, transparent background) —
/// "default icon for now" (spec): custom icon art is out of scope. One per
/// Lock state (amber = engaged, gray = released).
fn padlock_icon(rgb: (u8, u8, u8)) -> Icon {
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
                rgba[i] = rgb.0;
                rgba[i + 1] = rgb.1;
                rgba[i + 2] = rgb.2;
                rgba[i + 3] = 255;
            }
        }
    }
    Icon::from_rgba(rgba, S, S).expect("a 32x32 RGBA buffer is a valid icon")
}
