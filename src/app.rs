//! Thin-layer UI: the tray icon and the on-demand egui window. No unit tests
//! (spec: the thin layer is verified by the user's in-game hand-check).

use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use eframe::egui;
use tray_icon::menu::{Menu, MenuEvent, MenuItem};
use tray_icon::{Icon, MouseButton, TrayIcon, TrayIconBuilder, TrayIconEvent};

use crate::core;
use crate::icon;
use crate::seam::Rect;
use crate::win32;

/// The two pre-built tray icons: one per Lock state, swapped on state change
/// so the state is visible at a glance (user story 14). The window icon
/// swaps along with them.
pub struct TrayIcons {
    /// Engaged: amber screen with a padlock — the cursor is confined to the
    /// Game monitor.
    pub engaged: Icon,
    /// Released: dark gray screen — the cursor is free on all monitors.
    pub released: Icon,
    pub window_engaged: Arc<egui::IconData>,
    pub window_released: Arc<egui::IconData>,
}

impl TrayIcons {
    pub fn new() -> Self {
        Self {
            engaged: tray_icon(true),
            released: tray_icon(false),
            window_engaged: Arc::new(window_icon(true)),
            window_released: Arc::new(window_icon(false)),
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
    menu.append_items(&[&open, &pick, &auto, &exit])
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
                // Close hides the window; the app lives in the tray. Without
                // CancelClose the window closes for real and the app exits.
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                win32::hide_main_window();
            }
        }
        // Lock-state changes reported by the poll thread: swap the tray and
        // window icons.
        while let Ok(engaged) = self.state_rx.try_recv() {
            let (icon, window) = if engaged {
                (self.icons.engaged.clone(), self.icons.window_engaged.clone())
            } else {
                (self.icons.released.clone(), self.icons.window_released.clone())
            };
            let _ = self.tray.set_icon(Some(icon));
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Icon(Some(window)));
        }

        let snap = self.shared.lock().unwrap().clone();
        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("Monitor Lock");
            ui.separator();

            // The essentials: the Lock state, its target, and the pin.
            match &snap.lock {
                core::LockState::Released => {
                    ui.label(
                        egui::RichText::new("Lock: released")
                            .color(egui::Color32::from_rgb(136, 136, 136)),
                    );
                }
                core::LockState::Engaged { window_title, .. } => {
                    ui.label(
                        egui::RichText::new(format!(
                            "Lock: engaged — {}",
                            if window_title.is_empty() {
                                "window with no title"
                            } else {
                                window_title
                            }
                        ))
                        .color(egui::Color32::from_rgb(80, 200, 120)),
                    );
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
                // Pin mode is click-driven: any focus change suspends the
                // lock; a click inside the pinned monitor re-locks, a click
                // outside it drops the pin.
                if matches!(snap.lock, core::LockState::Released) && !snap.picking {
                    ui.label(
                        egui::RichText::new(
                            "Pinned (suspended): click inside this monitor to re-lock, click elsewhere to drop the pin",
                        )
                        .color(egui::Color32::from_rgb(255, 191, 64)),
                    );
                } else {
                    ui.label("Pinned: Alt+Tab or any focus change suspends the lock");
                }
            } else if let Some(game) = &snap.game_window {
                ui.label(format!(
                    "Target monitor (auto): {}",
                    format_monitor(&game.game_monitor_id, &game.game_monitor_bounds)
                ));
            } else {
                ui.label("Target monitor: — (no game window)");
            }

            if snap.picking {
                ui.label(
                    egui::RichText::new(
                        "Monitor picker: move to a monitor — click to pin, Esc to cancel",
                    )
                    .color(egui::Color32::from_rgb(255, 255, 0)),
                );
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

            // Everything else is diagnostics, one click away.
            ui.separator();
            egui::CollapsingHeader::new("Details")
                .default_open(false)
                .show(ui, |ui| {
                    ui.label(format!("Foreground window: {}", foreground_label(&snap)));
                    ui.label(format!("Game window: {}", game_window_label(&snap)));
                    ui.label(format!("Monitors: {}", snap.monitors.len()));
                    for m in &snap.monitors {
                        ui.label(format!("  {}", format_monitor(&m.id, &m.bounds)));
                    }
                    if snap.clip_reasserts > 0 {
                        ui.label(format!(
                            "Another program reset the cursor clip {} time(s); the lock put it back and the mouse guard kept the cursor in{}",
                            snap.clip_reasserts,
                            match snap.foreign_clip {
                                Some(r) => {
                                    format!(" (last: {},{} {}x{})", r.x, r.y, r.width, r.height)
                                }
                                None => String::new(),
                            }
                        ));
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
    format!(
        "{}  ({}, {}) {}x{}",
        id, bounds.x, bounds.y, bounds.width, bounds.height
    )
}

/// Install the tray and menu event handlers. They run as soon as an event
/// arrives, independent of whether the (possibly hidden) window is being
/// repainted: left-click / Open shows the on-demand window; `pick` / `auto`
/// are forwarded to the poll thread, which applies them to the brain; Exit
/// clears the clip and quits.
pub fn install_tray_handlers(ctx: egui::Context, cmd_tx: mpsc::Sender<UiCommand>) {
    let show = {
        let ctx = ctx.clone();
        move || {
            win32::show_main_window();
            ctx.request_repaint();
        }
    };
    {
        let show = show.clone();
        TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                ..
            } = event
            {
                show();
            }
        }));
    }
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| match event.id.0.as_str() {
        "open" => show(),
        "pick" => {
            let _ = cmd_tx.send(UiCommand::StartPicker);
        }
        "auto" => {
            let _ = cmd_tx.send(UiCommand::BackToAuto);
        }
        "exit" => exit_app(),
        _ => {}
    }));
}

/// A tray icon for one Lock state, drawn by `icon::rgba` at 32x32 (Windows
/// scales it to the tray's size).
fn tray_icon(engaged: bool) -> Icon {
    Icon::from_rgba(icon::rgba(32, engaged), 32, 32).expect("a 32x32 RGBA buffer is a valid icon")
}

/// The window's icon (title bar, taskbar, Alt+Tab) for one Lock state. Without
/// one, eframe shows its own default (the egui logo).
pub fn window_icon(engaged: bool) -> egui::IconData {
    egui::IconData {
        rgba: icon::rgba(64, engaged),
        width: 64,
        height: 64,
    }
}
