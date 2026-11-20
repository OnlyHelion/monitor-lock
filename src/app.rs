//! Thin-layer UI: the tray icon and the on-demand egui window. No unit tests
//! (spec: the thin layer is verified by the user's in-game hand-check). The
//! window's content is one dot-matrix panel texture (the `dotmatrix` module):
//! no native widgets are drawn for it; the buttons are hit-tested by hand.

use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use eframe::egui;
use tray_icon::menu::{Menu, MenuEvent, MenuItem};
use tray_icon::{Icon, MouseButton, TrayIcon, TrayIconBuilder, TrayIconEvent};

use crate::core;
use crate::dotmatrix;
use crate::icon;
use crate::win32;

/// The two pre-built tray icons: one per Lock state, swapped on state change
/// so the state is visible at a glance (user story 14). The window's icons
/// swap along with them (`win32::set_main_window_icon`).
pub struct TrayIcons {
    /// Engaged: amber screen with a padlock — the cursor is confined to the
    /// Game monitor.
    pub engaged: Icon,
    /// Released: dark gray screen — the cursor is free on all monitors.
    pub released: Icon,
}

impl TrayIcons {
    pub fn new() -> Self {
        Self {
            engaged: tray_icon(true),
            released: tray_icon(false),
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
    /// When the one-shot state-change wave started (None: not playing).
    pub(crate) wave_start: Option<Instant>,
    /// The last rendered state key (a change starts a fresh wave).
    pub(crate) prev_key: Option<dotmatrix::StateKey>,
    /// The DETAILS diagnostic rows are shown (the window grows with them).
    pub(crate) details_open: bool,
    /// The last requested inner height (points).
    pub(crate) inner_h: f32,
    /// The cursor shows a pointing hand (hovering an enabled button).
    pub(crate) cursor_hand: bool,
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
            let icon = if engaged {
                self.icons.engaged.clone()
            } else {
                self.icons.released.clone()
            };
            let _ = self.tray.set_icon(Some(icon));
            win32::set_main_window_icon(engaged);
        }

        let snap = self.shared.lock().unwrap().clone();
        let key = dotmatrix::state_key(&snap);

        // The one-shot state-change wave: fresh on every state-key change
        // (250 ms, from the padlock centre, dots blending toward ORANGE),
        // then the panel is static again. The first paint has no previous
        // state — the panel appears without waving.
        let now = Instant::now();
        if self.prev_key != Some(key) {
            if self.prev_key.is_some() {
                self.wave_start = Some(now);
            }
            self.prev_key = Some(key);
        }
        let wave_t = self.wave_start.and_then(|start| {
            dotmatrix::wave_progress(now.duration_since(start).as_millis(), dotmatrix::WAVE_MS)
        });

        // The panel content (padlock, the three status rows, the detail
        // rows) and the settled layout, in cells.
        let (mut map, placed) =
            dotmatrix::layout(&dotmatrix::panel_content(&snap, key, self.details_open));
        let cols = map.cols();
        let rows = map.rows();

        // PICK is dim and dead while the picker is active (the Lock is
        // released while picking; a repeated start would be a no-op anyway).
        if snap.picking {
            let b = placed.buttons[0];
            map.frame(b.col, b.row, b.w, b.h, dotmatrix::dim(dotmatrix::GREY));
            map.stamp_text(
                "PICK",
                b.col + 2,
                b.row + 2,
                dotmatrix::dim(dotmatrix::WHITE),
            );
        }

        // The wave: blend dots toward ORANGE by wave_intensity(d, t). The
        // formula's distances are 1x-pitch px, so 5 px per cell; the padlock
        // centre is (8+9, 21+9) in cell units.
        if let Some(t) = wave_t {
            let (pc, pr) = placed.padlock;
            let (pc, pr) = (pc as f32 + 9.0, pr as f32 + 9.0);
            for r in 0..rows {
                for c in 0..cols {
                    let dx = (c as f32 + 0.5) - pc;
                    let dy = (r as f32 + 0.5) - pr;
                    let d = (dx * dx + dy * dy).sqrt() * 5.0;
                    let i = dotmatrix::wave_intensity(d, t);
                    if i > 0.0 {
                        map.set(
                            c,
                            r,
                            dotmatrix::blend_toward(map.get(c, r), dotmatrix::ORANGE, i),
                        );
                    }
                }
            }
        }

        egui::CentralPanel::default().show(ui, |ui| {
            let ctx = ui.ctx();
            let rect = ui.max_rect();
            let ppp = ctx.input(|i| i.pixels_per_point());

            // The DPI-snapped pitch: 5 px per cell at 1x, scaled with the
            // display, clamped so the whole grid still fits the client area.
            // The texture is then stretched back to the panel: a uniform
            // scale on both axes (the window's aspect matches the grid's),
            // so the dots stay round.
            let client_w_px = (rect.width() * ppp) as u32;
            let client_h_px = (rect.height() * ppp) as u32;
            let mut pitch = (5.0 * ppp).round() as u32;
            pitch = pitch.min(client_w_px / cols).min(client_h_px / rows);
            let pitch = pitch.max(1);
            let tex_w = cols * pitch;
            let tex_h = rows * pitch;

            let rgba = dotmatrix::render(&map, pitch, tex_w, tex_h);
            let tex = ctx.load_texture_cached(
                "dot-panel",
                egui::Id::unique("dot-panel"),
                egui::ColorImage::from_rgba_unmultiplied([tex_w as usize, tex_h as usize], &rgba),
                Default::default(),
            );
            ui.painter().image(
                tex.id(),
                rect,
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );

            // Hit-testing: the texture is stretched to exactly the panel, so
            // the pointer's cell is its panel point divided by the cell
            // pitch in points.
            let (hit, clicked) = ctx.input(|i| {
                (
                    i.pointer.interact_pos().and_then(|p| {
                        let local = p - rect.min;
                        dotmatrix::cell_at(
                            local.x,
                            local.y,
                            rect.width() / cols as f32,
                            rect.height() / rows as f32,
                            cols,
                            rows,
                        )
                    }),
                    i.pointer.primary_clicked(),
                )
            });
            let in_ = |b: &dotmatrix::ButtonRect| {
                hit.map_or(false, |cell| dotmatrix::cell_in_rect(cell, b))
            };
            let over_pick = !snap.picking && in_(&placed.buttons[0]);
            let over_auto = in_(&placed.buttons[1]);
            let over_exit = in_(&placed.buttons[2]);
            let over_details = in_(&placed.details);

            if clicked {
                if over_pick {
                    let _ = self.cmd_tx.send(UiCommand::StartPicker);
                } else if over_auto {
                    let _ = self.cmd_tx.send(UiCommand::BackToAuto);
                } else if over_exit {
                    exit_app();
                } else if over_details {
                    self.details_open = !self.details_open;
                }
            }

            let hand = over_pick || over_auto || over_exit || over_details;
            if hand != self.cursor_hand {
                self.cursor_hand = hand;
                ctx.set_cursor_icon(if hand {
                    egui::CursorIcon::PointingHand
                } else {
                    egui::CursorIcon::Default
                });
            }
        });

        // The detail rows grow the grid; the window follows (and shrinks
        // back): 5 points per cell row, 640 points wide.
        let want_h = rows as f32 * 5.0;
        if (want_h - self.inner_h).abs() > f32::EPSILON {
            self.inner_h = want_h;
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(640.0, want_h)));
        }

        // Live-updates: the wave animates at ~16 ms; otherwise the poll
        // cadence.
        ui.ctx()
            .request_repaint_after(Duration::from_millis(if wave_t.is_some() {
                16
            } else {
                100
            }));
    }
}

/// Exit: the cursor clip is global Windows state that survives process
/// death (user story 20), so clear it before exiting.
fn exit_app() -> ! {
    win32::clear_clip();
    std::process::exit(0);
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

/// The window's startup icon (released art). Without one, eframe shows its
/// own default (the egui logo). Later swaps go through
/// `win32::set_main_window_icon`, which also sets the taskbar's big icon.
pub fn window_icon(engaged: bool) -> egui::IconData {
    egui::IconData {
        rgba: icon::rgba(64, engaged),
        width: 64,
        height: 64,
    }
}
