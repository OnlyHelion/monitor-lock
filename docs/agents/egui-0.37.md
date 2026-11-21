# egui / eframe 0.37: the API this repo uses

Verified 2026-11-20 against `~/.cargo/registry/src/*/egui-0.37.0` and `epaint-0.37.0`, and against `src/app.rs` / `src/main.rs`. Copy these; grep the registry only for something not listed here, then add it.

The window is one texture (ADR 0001): `src/app.rs` renders the dot panel to RGBA and paints it. Working calls, as they appear there:

| What | Call (0.37) |
|---|---|
| App entry | `impl eframe::App for X { fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) }`. There is no `update(ctx, frame)`. |
| Panel | `egui::CentralPanel::default().show(ui, \|ui\| { ... })`, then `ui.ctx()`, `ui.max_rect()` |
| DPI | `ctx.input(\|i\| i.pixels_per_point())` |
| Texture, every frame | `ctx.load_texture_cached(name: impl Into<String>, id: egui::Id, image: impl Into<ImageData>, options: TextureOptions) -> TextureHandle`. Re-uploads only when the image changes. The `id` must be unique per image. Evicted if not called for a full frame. |
| Pixels | `egui::ColorImage::from_rgba_unmultiplied([w, h], &rgba)` (asserts `w*h*4 == rgba.len()`) |
| Paint | `ui.painter().image(tex.id(), rect, egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), egui::Color32::WHITE)` |
| Pointer | `ctx.input(\|i\| (i.pointer.interact_pos(), i.pointer.primary_clicked()))` |
| Cursor | `ctx.set_cursor_icon(egui::CursorIcon::PointingHand)` / `egui::CursorIcon::Default`. `Arrow` does not exist. |
| Resize | `ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(w, h)))` |
| Close to tray | `ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose)` |
| Window setup (`main.rs`) | `ViewportBuilder::default().with_inner_size([640.0, 400.0]).with_resizable(false).with_decorations(false)` (frameless, #15) |
| Taskbar / title icon | `win32::set_main_window_icon`, not `ViewportCommand::Icon` (that one doesn't reach the taskbar). |
| Drag frameless window | `ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag)` on `i.pointer.primary_pressed()` (unverified on screen) |
| Minimise | `ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true))` |
