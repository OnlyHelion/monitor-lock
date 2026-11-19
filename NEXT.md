# NEXT — Monitor Lock
Updated: 2026-11-19 · OpenCode (Qwen3.8 27B UD) · implement-spec #8: ticket #9 DONE (pre-check), #10/#11/#12 next

## Next step
1. In main worktree `K:\monitor-lock` (on `dot-matrix`, HEAD = this NEXT.md commit): `git merge --no-ff ticket/9-dotmatrix-module` — should auto-resolve (ticket branch already contains dot-matrix tip 7f799ee; sides touch disjoint files). If a conflict appears anyway, dispatch a merger subagent.
2. After merge: run full `cargo test` once in the main worktree (expect 85 passed), then dispatch 3 parallel implementer subagents: worktrees `K:\wt\monitor-lock-10|11|12`, branches `ticket/N-...` from the dot-matrix tip, for #10 (picker ring + 2 chasers), #11 (window), #12 (icons). First check explorer notes `%TEMP%\opencode\ml-exploration\NOTES.md` (absent as of 23:29 — if still missing, dispatch without them; ticket bodies carry the acceptance criteria + constants).
3. After all merges: full `cargo test` once + `code-review` skill on `dot-matrix`; fix issues in one implementer subagent. Then report the integration branch to the user for HAND-CHECK (thin layer). Tickets stay open until the user confirms; closing is `gh issue close N --comment`. No PR (repo closes by direct master commits + v* tag, per prior releases).

## State now
- Ticket #9 (worktree `K:\wt\monitor-lock-9`, branch `ticket/9-dotmatrix-module`): `0638f7c` = pure `src/dotmatrix.rs` (967 lines: palette, 5×7 font, padlocks solid/hollow, CellMap, settled layout, AA real-dot renderer, wave, ring, stamp_dot; 16 new tests) + `mod dotmatrix;`; ledger entry committed on the branch; `dot-matrix` merged in as `26d68d7` (docs only, no conflicts). Worktree `cargo test`: 85 passed, 0 failed, 0 warnings = pre-check OK, awaiting user.
- `dot-matrix` (main worktree): 7f799ee + this NEXT.md commit. Remote: github.com/OnlyHelion/monitor-lock.
- Deviations flagged for hand-check (detail in ledger, top section "Ticket #9"): detail rows 6-row step shares a grid row with the 7-tall font; status row 1 = padlock colour; DETAILS rect = text extent 41×7 (no frame); stamp_dot added for #10/#11 chasers; unknown chars → blank.
- UNVERIFIED (carried): pin-mode Alt+Tab release on this Win11 build; if Alt+Tab holds the lock, add the foreground class to `is_shell_ui_class` (src/win32.rs).

## Don't retry
- Image generation (grok-imagine "Invalid credential" ×2) — procedural renderer only.
- ViewportCommand::Icon for the taskbar — use win32::set_main_window_icon.
- Hovering the Monitor Lock window changes the clip (suspected winit) — close to tray.
- `#` in .rc files (RC1021) — use /* */. ICO count is at byte offset 4.

## Pointers
- Spec/tickets: `gh issue view 8|9|10|11|12`. ADR 0001 + GLOSSARY + mockups in `docs/` (committed). Ledger: `docs/ledgers/monitor-lock.md` (top section = ticket #9 facts; flows in with the merge). Explorer notes (when ready): `%TEMP%\opencode\ml-exploration\NOTES.md`.
- Build speed: worktrees share `$env:CARGO_TARGET_DIR="K:\monitor-lock\target"` (deps cached; cargo file-locks serialize concurrent builds).
- Release: push a v* tag.
