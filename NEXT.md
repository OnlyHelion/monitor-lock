# NEXT — Monitor Lock
Updated: 2026-11-22 (tickets #13-#15 created from hand-check; ledger has the quote) - was 2026-11-20 10:4x · User hand-check 10:44: "looks good but there are some issues" - user will open a NEW session for the issues; tickets stay OPEN

## Next step
1. Hand-check issues are now TICKETS (children of #8, label ready-for-agent): #13 DETAILS spacing (lines 6 rows apart vs 7-row glyphs -> overlap; also clipping), #14 wave as discrete dots on the grid (not gradient), #15 frameless whole-window dot matrix with dot `_` and `X` (no maximise; blocked by #13). #13 and #14 independent. Implement each in a worktree from the dot-matrix tip (own target dir, docs/agents/parallel-work.md), merge --no-ff, fmt/clippy/test, re-ask user to hand-check.
2. After user's FULL confirmation: merge `dot-matrix` -> master (ff), bump 0.1.3 -> 0.2.0, tag v*, push, watch first CI run (ci.yml UNVERIFIED on GitHub). Then remove worktrees K:\wt\monitor-lock-9|10|11|12|review + ticket branches.
3. No PR (direct master commits + v* tag).

## Hand-check checklist (pre-checks OK; user impression "looks good" but some issues, not yet enumerated)
- WINDOW (#11, 10-point list in ledger #11 entry): 640×400 non-resizable, title "Monitor Lock", off-dots across panel; ENGAGED solid orange padlock + "LOCK: ENGAGED"/fg title/"TARGET: <id>" all orange; RELEASED solid white + "LOCK: RELEASED"/"CURSOR FREE" or fg title; SUSPENDED (pinned, released) HOLLOW orange + "LOCK: SUSPENDED"; PICKING solid white + "PICKING MONITOR / CLICK TO PIN / ESC TO CANCEL", PICK dimmed+dead, AUTO/EXIT live; WAVE one-shot ~250 ms from padlock on every state change and NOT on first show (review fix F7); DETAILS click grows with FG/GAME/MONITORS/per-monitor/CLIP REASSERTS rows, click again shrinks; PICK/AUTO/EXIT work + pointing hand over enabled buttons; dots round/crisp at 100/125/150 % DPI; tray menu, icon swap, close-to-tray, clip re-assert unchanged.
- PICKER (#10): dotted #FFB347 ring on hovered monitor (24 px inset/corners), two 7 px chasers opposite sides + 2 fading tail dots each, 6 s orbit (50 ms tick quantization by design), phase resets between monitors. Eyeball (mockup-derived, not ticket-pinned): tail radius 5 px, n = round(L/14) ring dots.
- ICONS (#12): tray + title bar/Alt+Tab + taskbar in BOTH states (orange engaged / white released, live swap on state change); Explorer exe = engaged art only (by design); 16 px reads as a solid padlock blob (inherent at that scale).
- Carried: hovering the Monitor Lock window can disturb the clip - close to tray for in-game checks. UNVERIFIED: pin-mode Alt+Tab release on this Win11 build (if it holds the lock, add the foreground class to is_shell_ui_class in src/win32.rs).
- Review-fix notes for the eyeball: layout values byte-identical (F3), padlock rendering identical (F4), pixels unchanged except F7. Retro commits after that are format/lint/docs/CI only.

## State now
- (verified 10:4x this session: fmt/clippy/test + git log) `dot-matrix` = 14d39d8 = 2c897a6 + retro b495e66 (fmt whole tree) + 9950689 (ci.yml + 8 clippy fixes) + 85613a4 (render_v2.py committed + README) + c9d4b0a (agent docs + AGENTS pointers). Behaviour = 101171b (review fixes F1-F7); retro is format/lint/docs/CI only. Local: `cargo fmt --check` 0, `cargo clippy --all-targets -- -D warnings` 0, `cargo test` 103/103.
- 2026-11-20 10:44 (user, quoted in ledger): "looks good but there are some issues, will make a new session for that." Issue list NOT in any file yet.
- Worktrees: K:\wt\monitor-lock-9|10|11|12 + K:\wt\monitor-lock-review (all merged, remove after user confirms). `_spec_diff.txt` in root is scratch (now gitignored).

## Don't retry
- Image generation: OpenCode's tool calls hosted opencode/grok-imagine; no OpenCode credential configured -> "Invalid credential". Use docs/design/dot-matrix/tools/render_v2.py.
- ViewportCommand::Icon for the taskbar - use win32::set_main_window_icon.
- Hovering the Monitor Lock window changes the clip (suspected winit) - close to tray.
- `#` in .rc files (RC1021) - use /* */. ICO count is at byte offset 4.

## Pointers
- Spec/tickets: `gh issue view 8|9|10|11|12`. ADR 0001 + GLOSSARY + design (`docs/design/dot-matrix/README.md`). Ledger: `docs/ledgers/monitor-lock.md`. Agent docs: `docs/agents/parallel-work.md`, `docs/agents/egui-0.37.md`. Remote: github.com/OnlyHelion/monitor-lock. Release: push a v* tag.
