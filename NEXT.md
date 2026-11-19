# NEXT — Monitor Lock
Updated: 2026-11-19 · OpenCode (Qwen3.8 27B UD) · dot-matrix redesign: spec #8 + 4 tickets published (ready-for-agent) — implementation is next, start #9

## Next step
1. Implement ticket #9 (pure dot-matrix module, no blockers — the frontier). Its body carries the approved prototype constants (palette, layout cells, wave formula, ring params):
   - NEW `src/dotmatrix.rs` (pure, unit-tested): palette BG #0E0F13 / OFF #26282E / ORANGE #FFB347 / WHITE #E8E8EC / GREY #888888 / DIML #3A3D45 / SOFT #60636C; 5×7 font + padlock 18×18 solid/hollow (copy from `%TEMP%\opencode\render_v2.py` — the approved design as code); CellMap; squircle LUT renderer (ratio 0.78, n=4, ss=16); `wave_intensity` (250 ms, Rmax 140 px, σ 13, I = exp(-((d-Rt)²/2σ²))·(1-t)·(1-d/Rmax), t=progress); `ring_point`/`ring_length` (port of mockup `ring_at`).
2. After #9 the frontier is #11 (window), #10 (picker ring), #12 (icons) — independent, parallelizable; each body has the acceptance criteria; thin layer is hand-checked by the user.
3. Window notes (#11): egui 0.37 texture API verified (`ctx.load_texture(impl Into<String>, impl Into<ImageData>, TextureOptions) -> TextureHandle`); panel = `ui.max_rect()` × `pixels_per_point()`; pitch = round(5·ppp) clamped so 128×80 cells fit; hit-test buttons + DETAILS in cell coords; PICK disabled while picking; DETAILS toggles window 640×400 ↔ 640×640 (`ViewportCommand::Resize` in points); wave on any state-key change, 16 ms repaints during the 250 ms wave else 100 ms.
4. Icons (#12): screen rect → 18×18 SQUARE dot panel (u 0.26–0.74, v 0.15–0.63, pitch 0.48/18, squircle ratio 0.78); engaged = solid padlock ORANGE, released = solid padlock WHITE, off-dots OFF on BG; bezel/neck/base UNCHANGED (test keeps alpha identical). Then `MONITOR_LOCK_WRITE_ICO=1 cargo test ico_file`, full `cargo test`, hand-check.

## State now
- SETTLED (user, quoted in ledger): scope = everything (window, tray, taskbar, exe); one shared dot grid, 5 px pitch @1x, 640×400; "real dots" = squircles (superellipse n=4, ratio 0.78); glyph = padlock 18×18; locked #FFB347, released #E8E8EC, suspended = hollow same orange; off-dots visible panel-wide; static UI + one-shot ≤300 ms = small localized orange wave from the padlock; picker ring = dotted orange + 2 chasers opposite sides (only continuous animation); short labels; status rows 2-row gap; buttons left edge = col 8 (same as heading/DETAILS).
- Published: spec #8 (ready-for-agent); tickets #9 (module, no blockers), #11 (window), #10 (picker ring), #12 (icons) — all sub-issues of #8, ready-for-agent, blocked-by edges in body. ADR 0001 + GLOSSARY (incl. Wave) + final mockups (04/06/07/08) in docs. Code NOT yet changed (v0.1.3, f58312b).
- Type defs: `LockState`/`LockSnapshot` core.rs:459-528; window size main.rs:167; overlay paint overlay.rs:153-228; icon geometry icon.rs:63-109; `win32::set_main_window_icon` :378.
- UNVERIFIED (carried): pin-mode Alt+Tab release on this Win11 build; if Alt+Tab holds the lock, add the foreground class to `is_shell_ui_class` (src/win32.rs).

## Don't retry
- Image generation (grok-imagine "Invalid credential" ×2) — procedural renderer only.
- ViewportCommand::Icon for the taskbar — use win32::set_main_window_icon.
- Hovering the Monitor Lock window changes the clip (suspected winit) — close to tray.
- `#` in .rc files (RC1021) — use /* */. ICO count is at byte offset 4.

## Pointers
- `docs/design/dot-matrix/` (refs, 04-real-dot-shape.png, README, mockups); `docs/ledgers/monitor-lock.md` (top section: Dot-matrix redesign, all rounds quoted); renderer `%TEMP%\opencode\render_v2.py` (re-runnable, carries every layout constant); release: push a v* tag.
