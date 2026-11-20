# NEXT — Monitor Lock
Updated: 2026-11-20 08:xx · Claude · spec #8 done + retro fixes (fmt, CI, clippy, agent docs) on `dot-matrix`, unpushed; AWAITING USER HAND-CHECK

## Next step
1. User hand-checks the `dot-matrix` branch (checklist below). Tickets #8/#9/#10/#11/#12 stay OPEN until the user confirms; closing is `gh issue close N --comment`. If the hand-check finds a defect: fix in a worktree from the dot-matrix tip (own target dir, see docs/agents/parallel-work.md), merge --no-ff back, re-run fmt/clippy/test, re-ask.
2. After user confirms: merge `dot-matrix` → master (fast-forward), bump version (0.1.3 → 0.2.0, design-language release), tag v* per prior releases, push. Watch the first CI run (new ci.yml). Then `git worktree remove` K:\wt\monitor-lock-9|10|11|12 + K:\wt\monitor-lock-review and delete the ticket branches.
3. No PR (repo closes by direct master commits + v* tag).

## Hand-check checklist (pre-checks all OK, awaiting user)
- WINDOW (#11, 10-point list in ledger #11 entry): 640×400 non-resizable, title "Monitor Lock", off-dots across panel; ENGAGED solid orange padlock + "LOCK: ENGAGED"/fg title/"TARGET: <id>" all orange; RELEASED solid white + "LOCK: RELEASED"/"CURSOR FREE" or fg title; SUSPENDED (pinned, released) HOLLOW orange + "LOCK: SUSPENDED"; PICKING solid white + "PICKING MONITOR / CLICK TO PIN / ESC TO CANCEL", PICK dimmed+dead, AUTO/EXIT live; WAVE one-shot ~250 ms from padlock on every state change and NOT on first show (review fix F7); DETAILS click grows with FG/GAME/MONITORS/per-monitor/CLIP REASSERTS rows, click again shrinks; PICK/AUTO/EXIT work + pointing hand over enabled buttons; dots round/crisp at 100/125/150 % DPI; tray menu, icon swap, close-to-tray, clip re-assert unchanged.
- PICKER (#10): dotted #FFB347 ring on hovered monitor (24 px inset/corners), two 7 px chasers opposite sides + 2 fading tail dots each, 6 s orbit (50 ms tick quantization by design), phase resets between monitors. Eyeball (mockup-derived, not ticket-pinned): tail radius 5 px, n = round(L/14) ring dots.
- ICONS (#12): tray + title bar/Alt+Tab + taskbar in BOTH states (orange engaged / white released, live swap on state change); Explorer exe = engaged art only (by design); 16 px reads as a solid padlock blob (inherent at that scale).
- Carried: hovering the Monitor Lock window can disturb the clip - close to tray for in-game checks. UNVERIFIED: pin-mode Alt+Tab release on this Win11 build (if it holds the lock, add the foreground class to is_shell_ui_class in src/win32.rs).
- Review-fix notes for the eyeball: layout values byte-identical (F3), padlock rendering identical (F4), pixels unchanged except F7. Retro commits after that are format/lint/docs only.

## State now
- (verified 08:xx, cargo + git log) Retro commits on top of 2c897a6: b495e66 `cargo fmt` whole tree; 9950689 `.github/workflows/ci.yml` (fmt --check, clippy -D warnings, tests skip real_system, every push) + 8 clippy fixes; 85613a4 `docs/design/dot-matrix/tools/render_v2.py` (re-renders mockups 04/06/07/08 byte-identical) + README; c9d4b0a `docs/agents/parallel-work.md`, `docs/agents/egui-0.37.md`, AGENTS.md pointers, root `/_*` gitignored. Local: fmt 0, clippy 0, `cargo test` 103/103. UNVERIFIED: ci.yml on GitHub (first run on next push).
- `dot-matrix` code = master 7c78680 + merges #9 (fa3fb1e) + #12 (3cb408e) + #10 (7893541) + #11 (bf4a031) + review fixes (7ba61b3) + retro commits above.
- Code review both axes + fixes recorded in ledger "Code-review" lines (F1-F7 all fixed).
- Worktrees: K:\wt\monitor-lock-9|10|11|12 + K:\wt\monitor-lock-review (all merged, remove after user confirms). `_spec_diff.txt` in root is scratch (now gitignored).

## Don't retry
- Image generation: OpenCode's tool calls hosted opencode/grok-imagine; no OpenCode credential configured -> "Invalid credential". Use docs/design/dot-matrix/tools/render_v2.py.
- ViewportCommand::Icon for the taskbar - use win32::set_main_window_icon.
- Hovering the Monitor Lock window changes the clip (suspected winit) - close to tray.
- `#` in .rc files (RC1021) - use /* */. ICO count is at byte offset 4.

## Pointers
- Spec/tickets: `gh issue view 8|9|10|11|12`. ADR 0001 + GLOSSARY + design (`docs/design/dot-matrix/README.md`). Ledger: `docs/ledgers/monitor-lock.md`. Agent docs: `docs/agents/parallel-work.md`, `docs/agents/egui-0.37.md`. Remote: github.com/OnlyHelion/monitor-lock. Release: push a v* tag.
