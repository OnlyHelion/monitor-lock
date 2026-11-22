# NEXT — Monitor Lock
Updated: 2026-11-22 · v0.3.0 released (details-scroll folded in at 625pt; GUI subsystem in all builds = no black console window)

## Next step
1. Verify the GitHub release: `gh run list --workflow=Release` (tag v0.3.0) — greens in a few minutes (test, release build, sha256, `gh release create`); then `gh release view v0.3.0` should show monitor-lock.exe + .sha256.
2. User eyeball of the new behaviour (pre-check OK: fmt/clippy 0, `cargo test` 119/119): fresh `K:\monitor-lock\target\debug\monitor-lock.exe` (1:47 PM; quit any running instance from the tray first) — double-clicking must NOT open a black console window; click DETAILS → window is exactly 625pt (125 rows), detail rows below the DETAILS label, top bar `_`/`X` pinned, wheel = 3 rows/notch, `↑`/`↓` = 1 row, dot scrollbar col 126 when content overflows.

## State now
- master = `2372f11` "Release v0.3.0: scrollable DETAILS at 625pt, no console window in any build", tag v0.3.0 pushed (docs committed separately). v0.3.0 = `dotmatrix.rs` +`DETAILS_OPEN_ROWS` = 125 / `DETAIL_TOP` = 78 / `details_max_scroll` / `details_visible_map` (window row r shows full row r+scroll; off-dots past the end) / `draw_detail_scrollbar` (col 126) +4 tests (119 total); `app.rs` DETAILS-open = fixed 125 rows (625pt), closed = 80 rows (400pt), `detail_scroll`/`detail_scroll_frac` fields, wheel from raw `i.events` (Line = 3 rows/notch — user-confirmed speed; Point = 5 pt/row; Page = 30), `↑`/`↓` step 1 row; `main.rs` `#![windows_subsystem = "windows"]` in **all builds** (PE subsystem = 2 verified on both exes); `Cargo.toml` 0.3.0.
- Prototype variant set preserved on throwaway branch `prototype/details-heights` (565ef1d: A 98 / B 116 / C 125 / D 134 / E auto + pill); `src/prototype_details.rs` deleted from master.
- Build dirs: default `target\debug` (fresh exe 1:47 PM); `target\wt\main` = worktree target dir, stale (pre-prototype).
- v0.2.0 released (tag v0.2.0). Pin-mode Alt+Tab fine on the user's Win11 build. Hovering the window can disturb the clip: close to tray for in-game use.

## Don't retry
- Image generation: OpenCode credential missing; use docs/design/dot-matrix/tools/render_v2.py. This model also cannot read image FILES (read tool rejects them) — but images pasted in the user's messages ARE visible: read them before diagnosing from code alone.
- ViewportCommand::Icon for the taskbar - use win32::set_main_window_icon.
- `#` in .rc files (RC1021) - use /* */. ICO count is at byte offset 4.
- Rebuilding while the app runs fails (exe locked): quit it from the tray first.
- egui 0.37 wheel (all verified against the registry source): native `line_scroll_speed = 40.0` default (input_state/mod.rs:105-109) — a line-unit notch (winit 0.30.13: WM_MOUSEWHEEL raw/120 = exactly 1.0 line, event_loop.rs:1727-1745; egui-winit passes it through unscaled) reaches `smooth_scroll_delta` as ~40 lines AND is spread over ~60 frames (wheel_state.rs:190-209) — NEVER trust `smooth_scroll_delta` magnitude for discrete stepping; count `Event::MouseWheel` from the public `InputState.events` field instead (its `delta` is unscaled). Negative y = scroll down (verified vs ScrollArea `delta: -scroll_delta`, containers/scroll_area.rs:1355). No `Align2::CENTER` (use `CENTER_CENTER`); `Painter` has no `rounded_rect` (use `rect(..., StrokeKind)`).

## Pointers
- Ledger: `docs/ledgers/monitor-lock.md` (active: 12:15 verdict+bug, 12:2x/12:4x/1:0x fixes, 1:1x speed fix, 13:20 clarification, 13:2x approval, 1:4x PM v0.3.0 fold-in+release; archive: `docs/ledgers/archive/monitor-lock-implement-spec-to-prototype-start-2026-11-22.md`). Design: `docs/design/dot-matrix/README.md`, ADR 0001, GLOSSARY. Agent docs: `docs/agents/`. Remote: github.com/OnlyHelion/monitor-lock. Release: push a v* tag.
