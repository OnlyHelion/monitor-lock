# NEXT — Monitor Lock
Updated: 2026-11-20 · OpenCode (Qwen3.8 27B UD) · implement-spec #8: #9 + #10 + #12 MERGED into `dot-matrix`; only #11 (window) in flight

## Next step
1. When #11 (window, ses_e09f98eccffdjGfw9oBRoZjvoq, wt `K:\wt\monitor-lock-11`, br `ticket/11-window`) reports done: in main worktree `K:\monitor-lock` (on `dot-matrix`): `git merge --no-ff ticket/11-window`. On conflict → dispatch a merger subagent (it may touch dotmatrix.rs where #10/#12 added helpers).
2. After that merge (the LAST): one full `cargo test` in the main worktree, then the `code-review` skill on `dot-matrix`; fix all issues in ONE implementer subagent (worktree from `dot-matrix` tip).
3. Then report the integration branch to the user for HAND-CHECK (thin layer: window, overlay, icon destinations). Tickets stay open until the user confirms; closing is `gh issue close N --comment`. No PR (repo closes by direct master commits + v* tag).
4. After user confirms: merge `dot-matrix` → master (or fast-forward master onto it), bump version, tag v* per prior releases. Clean up worktrees `K:\wt\monitor-lock-9|10|11|12`.

## State now
- `dot-matrix` (main worktree) = 7893541 = master 7c78680 (docs) + merge #9 (fa3fb1e) + merge #12 (3cb408e) + merge #10 (7893541). #9 = pure `src/dotmatrix.rs`; #10 = `src/overlay.rs` ring rewrite + chaser helpers (90/90 at its tip); #12 = `src/icon.rs` dot-panel screen + `dotmatrix::dot_at` + regenerated `res/icon.ico`. All pre-checks OK, awaiting user.
- Running subagents: #11 window (ses_e09f98eccffdjGfw9oBRoZjvoq, wt `K:\wt\monitor-lock-11`, br `ticket/11-window`). Based on fa3fb1e (pre-#10/#12) — it merges `dot-matrix` into its branch before reporting, so expect it to pick up the overlay/icon changes.
- Hand-check checklist owed (full detail in ledger "Ticket #9"/"#10"/"#12" sections): window states + wave + DETAILS + DPI; picker ring/chasers (eyeball tail r5, n=round(L/14)); icons in all four destinations × both states.
- Explorer notes (done): `%TEMP%\opencode\ml-exploration\NOTES.md` (222 lines, 7 sections) — handed to all three.
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
