# NEXT — Monitor Lock
Updated: 2026-11-22 · Tickets #13-#15 implemented + code-reviewed + merged into `dot-matrix`; AWAITING USER HAND-CHECK (never run on screen by an agent)

## Next step
1. User hand-checks the new build (`cargo run` from K:\monitor-lock, branch dot-matrix = e4a92cd):
   - #13: DETAILS open with 1 and 2+ monitors: no overlap, no cut-off at right/bottom; click again shrinks. 7 lines = 715 pt tall: does it fit the screen at 125/150 %? (cap NOT implemented.)
   - #14: wave = discrete dots (3 levels, WAVE_LEVELS) on ENGAGED/RELEASED/SUSPENDED/PICKING; once per change, not on first show.
   - #15: no native frame; dot title strip + `_` + `X`; drag the strip moves the window (also when DETAILS grown); `_` minimises, taskbar restores; `X` hides to tray; hand cursor over both; taskbar entry/Alt+Tab/icon swap OK; crisp at 100/125/150 % DPI. UNVERIFIED on screen: StartDrag/Minimized on undecorated eframe 0.37.
   - #16 (user: icon was a monitor+dot-panel mix, wanted ONLY the dot padlock): icon = dark rounded tile with 18x18 dots + padlock, no monitor. Check tray, Alt+Tab, taskbar, Explorer (icon cache may need refresh), both states, live swap. Previews in _notes/icon-*.png (gitignored).
   - #17 (user: "one more state for when its disengaged" = SUSPENDED): third icon art, hollow orange padlock, in tray/title bar/taskbar. Pin a monitor, Alt+Tab away -> hollow; click back -> solid orange; picker/auto -> solid white; no stale icon. Previews _notes/icon17-*.png.
   - Review fix: monitor ids now shown without `\\.\` prefix (DISPLAY1) in the TARGET status row and DETAILS rows.
2. Then (only after user's FULL confirmation): merge `dot-matrix` -> master (ff), bump 0.1.3 -> 0.2.0, tag v*, push, watch first CI run (ci.yml UNVERIFIED on GitHub). Close #8-#17. Remove worktrees K:\wt\monitor-lock-9|10|11|12|review + their ticket branches.
3. No PR (direct master commits + v* tag).

## State now
- (verified) `dot-matrix` = see git log (#16 icon merged). fmt/clippy clean, `cargo test` 114/114. Tickets #13-#17 + review-fix branches merged --no-ff and deleted; their worktrees removed. Issues #8-#17 still OPEN on purpose (close on release).
- Old worktrees still present: K:\wt\monitor-lock-9|10|11|12|review (all merged).
- ADR 0001 amended (discrete wave, frameless window); GLOSSARY has Title strip/Minimise/Close.
- Carried, UNVERIFIED: pin-mode Alt+Tab release on this Win11 build (if it holds the lock, add the foreground class to is_shell_ui_class in src/win32.rs). Hovering the Monitor Lock window can disturb the clip: close to tray for in-game checks.

## Don't retry
- Image generation: OpenCode credential missing; use docs/design/dot-matrix/tools/render_v2.py.
- ViewportCommand::Icon for the taskbar - use win32::set_main_window_icon.
- `#` in .rc files (RC1021) - use /* */. ICO count is at byte offset 4.

## Pointers
- Spec/tickets: `gh issue view 8..15`. ADR 0001 + GLOSSARY + `docs/design/dot-matrix/README.md`. Ledger: `docs/ledgers/monitor-lock.md`. Agent docs: `docs/agents/parallel-work.md`, `docs/agents/egui-0.37.md`. Remote: github.com/OnlyHelion/monitor-lock. Release: push a v* tag.
