# NEXT — Monitor Lock
Updated: 2026-11-20 · implement-spec #8: all four tickets merged (code tip bf4a031), 95/95 green; code review done; fixer in flight

## Next step
1. Fixer subagent running (ses_e0916b301ffet3MO1KP1vVD9kk): worktree `K:\wt\monitor-lock-review`, br `ticket/dot-matrix-review-fixes`, fixes F1-F7 (F1 move pure helpers to dotmatrix + tests + panel_content returns Panel; F2 glossary avoid-term in StateKey docs; F3 single-source layout consts; F4 unify (col,row) vs (row,col) convention; F5 overlay chaser helper; F6 hoist icon.rs consts; F7 no wave on first paint). When it reports: in `K:\monitor-lock` (on `dot-matrix`): `git merge --no-ff ticket/dot-matrix-review-fixes`, then full `cargo test` + `cargo build` (expect >=95 passed, 0 failed, zero warnings).
2. Then report the integration branch to the user for HAND-CHECK (thin layer: window, overlay, icon destinations). Tickets stay open until the user confirms; closing is `gh issue close N --comment`. No PR (repo closes by direct master commits + v* tag).
3. After user confirms: merge `dot-matrix` → master (fast-forward), bump version, tag v* per prior releases. Clean up worktrees `K:\wt\monitor-lock-9|10|11|12` + `K:\wt\monitor-lock-review`.

## State now
- `dot-matrix` (main worktree) = 5f34e47 (docs) / code tip bf4a031 = master 7c78680 + merges #9 (fa3fb1e) + #12 (3cb408e) + #10 (7893541) + #11 (bf4a031). Full `cargo test` = 95/95 (incl. real_system_*); `cargo build` zero warnings. Pre-check OK, awaiting user.
- Code review BOTH axes in (ledger "Code-review" lines): Standards - no hard violations, judgement calls J1/J2 + smells S1-S5; Spec - no missing requirements, no scope creep, one minor (wave on first paint = F7).
- Hand-check checklist owed (detail in ledger: #10/#12 entries + #11 10-point list; #9 section in `docs/ledgers/archive/monitor-lock-ticket-9-dotmatrix-2026-11-19.md`): window states + wave + DETAILS + DPI; picker ring/chasers (eyeball tail r5, n=round(L/14)); icons in all four destinations x both states.
- UNVERIFIED (carried): pin-mode Alt+Tab release on this Win11 build; if Alt+Tab holds the lock, add the foreground class to `is_shell_ui_class` (src/win32.rs).

## Don't retry
- Shell commands containing an em dash are rejected by the portable shell scanner ("invalid-structure"), even inside here-strings - use ASCII hyphens.
- Image generation (grok-imagine "Invalid credential" x2) - procedural renderer only.
- ViewportCommand::Icon for the taskbar - use win32::set_main_window_icon.
- Hovering the Monitor Lock window changes the clip (suspected winit) - close to tray.
- `#` in .rc files (RC1021) - use /* */. ICO count is at byte offset 4.

## Pointers
- Spec/tickets: `gh issue view 8|9|10|11|12`. ADR 0001 + GLOSSARY + mockups in `docs/` (committed). Ledger: `docs/ledgers/monitor-lock.md` (code-review lines near top; grill-with-docs + ticket #9 sections in `docs/ledgers/archive/`). Remote: github.com/OnlyHelion/monitor-lock. Release: push a v* tag.
- Build speed: worktrees share `$env:CARGO_TARGET_DIR="K:\monitor-lock\target"` (deps cached; cargo file-locks serialize concurrent builds - normal, don't kill). The #11 wt's `target-local/` is a private throwaway dir (untracked).
