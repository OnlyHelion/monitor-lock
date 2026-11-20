# NEXT — Monitor Lock
Updated: 2026-11-19 · OpenCode (Qwen3.8 27B UD) · implement-spec #8: #9 MERGED into `dot-matrix`; #10/#11/#12 in parallel now

## Next step
1. When each implementer reports done (sessions below): in main worktree `K:\monitor-lock` (on `dot-matrix`): `git merge --no-ff ticket/N-...`. On conflict → dispatch a merger subagent. Merge order = completion order.
2. After the LAST merge: one full `cargo test` in the main worktree, then the `code-review` skill on `dot-matrix`; fix all issues in ONE implementer subagent (worktree from `dot-matrix` tip).
3. Then report the integration branch to the user for HAND-CHECK (thin layer: window, overlay, icon destinations). Tickets stay open until the user confirms; closing is `gh issue close N --comment`. No PR (repo closes by direct master commits + v* tag).
4. After user confirms: merge `dot-matrix` → master (or fast-forward master onto it), bump version, tag v* per prior releases. Clean up worktrees `K:\wt\monitor-lock-9|10|11|12`.

## State now
- `dot-matrix` (main worktree) = fa3fb1e = master 7c78680 (docs: ADR 0001, GLOSSARY, mockups) + merge of ticket #9. #9 = pure `src/dotmatrix.rs` (967 lines, 16 tests) + `mod dotmatrix;` in main.rs; 85/85 in its worktree (pre-check OK, awaiting user). Deviations flagged for hand-check (ledger "Ticket #9"): detail rows share a grid row at the 6-row step; status row 1 = padlock colour; DETAILS rect = text extent (no frame); `stamp_dot` added for #10/#11; unknown chars → blank.
- Running subagents: #10 picker ring (ses_e09f98eccffeqaDLUl3ssAc0nG, wt `K:\wt\monitor-lock-10`, br `ticket/10-picker-ring`); #11 window (ses_e09f98eccffdjGfw9oBRoZjvoq, wt `K:\wt\monitor-lock-11`, br `ticket/11-window`); #12 icons (ses_e09f98ec9ffesCjuImyYohm9Zb, wt `K:\wt\monitor-lock-12`, br `ticket/12-icons`). All based on fa3fb1e.
- Explorer notes (done): `%TEMP%\opencode\ml-exploration\NOTES.md` (202 lines, 7 sections) — handed to all three.
- Build speed: worktrees share `$env:CARGO_TARGET_DIR="K:\monitor-lock\target"` (deps cached; cargo file-locks serialize concurrent builds — normal, don't kill).
- UNVERIFIED (carried): pin-mode Alt+Tab release on this Win11 build; if Alt+Tab holds the lock, add the foreground class to `is_shell_ui_class` (src/win32.rs).

## Don't retry
- Shell commands containing an em dash `—` are rejected by the portable shell scanner ("invalid-structure"), even inside here-strings — use ASCII hyphens.
- Image generation (grok-imagine "Invalid credential" ×2) — procedural renderer only.
- ViewportCommand::Icon for the taskbar — use win32::set_main_window_icon.
- Hovering the Monitor Lock window changes the clip (suspected winit) — close to tray.
- `#` in .rc files (RC1021) — use /* */. ICO count is at byte offset 4.

## Pointers
- Spec/tickets: `gh issue view 8|9|10|11|12`. ADR 0001 + GLOSSARY + mockups in `docs/` (committed). Ledger: `docs/ledgers/monitor-lock.md` (top section = ticket #9 facts). Remote: github.com/OnlyHelion/monitor-lock. Release: push a v* tag.
