# NEXT — Monitor Lock
Updated: 2026-11-20 · implement-spec #8: ALL FOUR tickets merged into `dot-matrix` (bf4a031); full check green; code-review in flight

## Next step
1. Two code-review subagents running: Standards (ses_e09452a00ffe9cVPF71oEweXyo) + Spec (ses_e094529ffffe02bPfJdJZ2UkOs), diff = `git diff master...HEAD`. When BOTH report: aggregate under `## Standards` / `## Spec` (don't rerank across axes), then fix ALL issues in ONE implementer subagent (worktree from `dot-matrix` tip), re-run full `cargo test`, merge the fix branch in.
2. Then report the integration branch to the user for HAND-CHECK (thin layer: window, overlay, icon destinations). Tickets stay open until the user confirms; closing is `gh issue close N --comment`. No PR (repo closes by direct master commits + v* tag).
3. After user confirms: merge `dot-matrix` → master (fast-forward), bump version, tag v* per prior releases. Clean up worktrees `K:\wt\monitor-lock-9|10|11|12`.

## State now
- `dot-matrix` (main worktree) = bf4a031 = master 7c78680 (docs) + merges #9 (fa3fb1e) + #12 (3cb408e) + #10 (7893541) + #11 (bf4a031). Diff vs master: 9 files, +1921/−306 (dotmatrix.rs new; app.rs, overlay.rs, icon.rs, main.rs rewritten parts; res/icon.ico regenerated).
- AFTER the last merge, in the main worktree: full `cargo test` = **95 passed, 0 failed** (incl. real_system_*); `cargo build` clean, zero warnings. Pre-check OK, awaiting user.
- #11's pre-merge conflict (dotmatrix.rs, keep-both) was resolved by its implementer in 793abb1; the final merge into dot-matrix was clean.
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
