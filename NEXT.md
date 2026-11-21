# NEXT — Monitor Lock
Updated: 2026-11-22 · v0.2.0 (dot-matrix redesign, spec #8, tickets #9-#18) hand-checked OK by the user, merged to master, tagged, pushed

## Next step
1. Watch the first GitHub CI run + the release workflow for tag v0.2.0 (`gh run list`). UNVERIFIED: ci.yml and release.yml have not run on GitHub for this version. If the release asset is missing/broken, fix on master and retag (or v0.2.1).
2. After CI is green: close issues #8-#18 (`gh issue close`), remove old worktrees K:\wt\monitor-lock-9|10|11|12|review and their ticket branches + `dot-matrix` branch.
3. No PR (direct master commits + v* tag).

## State now
- master = dot-matrix tip + version bump 0.1.3 -> 0.2.0 (Cargo.toml/lock), tag v0.2.0. fmt/clippy clean, `cargo test` 115/115 before the bump.
- User hand-check: "it was good!" (wave rules, 3 icon states, frameless window, DETAILS spacing).
- Carried, UNVERIFIED: pin-mode Alt+Tab release on this Win11 build (if it holds the lock, add the foreground class to is_shell_ui_class in src/win32.rs). Not implemented: window height cap for 7 DETAILS lines (715 pt). Hovering the window can disturb the clip: close to tray for in-game use.

## Don't retry
- Image generation: OpenCode credential missing; use docs/design/dot-matrix/tools/render_v2.py.
- ViewportCommand::Icon for the taskbar - use win32::set_main_window_icon.
- `#` in .rc files (RC1021) - use /* */. ICO count is at byte offset 4.
- Rebuilding while the app runs fails (exe locked): quit it from the tray first.

## Pointers
- Ledger: `docs/ledgers/monitor-lock.md`. Design: `docs/design/dot-matrix/README.md`, ADR 0001, GLOSSARY. Agent docs: `docs/agents/`. Remote: github.com/OnlyHelion/monitor-lock. Release: push a v* tag.
