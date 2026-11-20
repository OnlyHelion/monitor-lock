# NEXT — Monitor Lock
Updated: 2026-11-20 · implement-spec #8: all four tickets done — #11 (window) on `ticket/11-window`, unpushed

## Next step
1. In main worktree `K:\monitor-lock` (on `dot-matrix`): `git merge --no-ff ticket/11-window`. Expected clean: the branch already contains all of `dot-matrix` (7893541), so this only adds the #11 commit.
2. After that merge (the LAST): one full `cargo test` in the main worktree, then the `code-review` skill on `dot-matrix`; fix all issues in ONE implementer subagent (worktree from `dot-matrix` tip).
3. Then report the integration branch to the user for HAND-CHECK (thin layer: window, overlay, icon destinations). Tickets stay open until the user confirms; closing is `gh issue close N --comment`. No PR (repo closes by direct master commits + v* tag).
4. After user confirms: merge `dot-matrix` → master (fast-forward), bump version, tag v* per prior releases. Clean up worktrees `K:\wt\monitor-lock-9|10|11|12`.

## State now
- `ticket/11-window` (wt `K:\wt\monitor-lock-11`) = 793abb1 = merge of `dot-matrix` (7893541) + b7610a1. b7610a1 = the #11 commit: `src/app.rs` (window content = one dot-panel texture per frame: padlock states, 3 status rows, hand-hit-tested PICK/AUTO/EXIT + DETAILS, 250 ms one-shot wave, DPI-snapped pitch, InnerSize grow/shrink, 16/100 ms repaint), `src/main.rs` (640×400 pts non-resizable, 5 new struct fields), `src/dotmatrix.rs` (pure helpers StateKey/state_key, fit_width, cell_at, cell_in_rect, WAVE_MS/wave_progress + 5 tests).
- The 793abb1 merge had ONE conflicted file, `src/dotmatrix.rs`, two regions, both resolved keep-both: `dot_at` (#12) placed before the StateKey block; their chaser/ring tests + the 5 #11 tests kept. Verified AFTER the merge, in private `K:\wt\monitor-lock-11\target-local`: 95/95 green, `cargo build` clean (zero warnings).
- `dot-matrix` (main worktree) still at 7893541 = #9 + #12 + #10 + docs; #11 lands with the step-1 merge.
- Hand-check checklist owed (full detail in ledger "Ticket #9"/"#10"/"#12" sections): window states + wave + DETAILS + DPI; picker ring/chasers (eyeball tail r5, n=round(L/14)); icons in all four destinations × both states.
- UNVERIFIED (carried): pin-mode Alt+Tab release on this Win11 build; if Alt+Tab holds the lock, add the foreground class to `is_shell_ui_class` (src/win32.rs).

## Don't retry
- Shell commands containing an em dash `—` are rejected by the portable shell scanner ("invalid-structure"), even inside here-strings — use ASCII hyphens.
- Image generation (grok-imagine "Invalid credential" ×2) — procedural renderer only.
- ViewportCommand::Icon for the taskbar — use win32::set_main_window_icon.
- Hovering the Monitor Lock window changes the clip (suspected winit) — close to tray.
- `#` in .rc files (RC1021) — use /* */. ICO count is at byte offset 4.

## Pointers
- Spec/tickets: `gh issue view 8|9|10|11|12`. ADR 0001 + GLOSSARY + mockups in `docs/` (committed). Ledger: `docs/ledgers/monitor-lock.md`. Remote: github.com/OnlyHelion/monitor-lock. Release: push a v* tag.
- Build speed: worktrees share `$env:CARGO_TARGET_DIR="K:\monitor-lock\target"` (deps cached; cargo file-locks serialize concurrent builds — normal, don't kill). The #11 wt's `target-local/` is a private throwaway dir (untracked).
