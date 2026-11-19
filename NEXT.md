# NEXT — Monitor Lock
Updated: 2026-11-19 · OpenCode (Qwen3.8 27B UD) · implement-spec #8 in progress: integration branch `dot-matrix`, ticket #9 being implemented

## Next step
1. Implementer subagent (session ses_e0a5474c5ffdmRADEe5Z6Uvch3) is building ticket #9 in worktree `K:\wt\monitor-lock-9` (branch `ticket/9-dotmatrix-module`, based on `dot-matrix` tip 7c78680). When it reports done:
   - In main worktree `K:\monitor-lock` (on `dot-matrix`): `git merge --no-ff ticket/9-dotmatrix-module`; on conflict dispatch a merger subagent.
   - Then the frontier opens: create worktrees `K:\wt\monitor-lock-10|11|12` on branches `ticket/N-...` from the `dot-matrix` tip and dispatch 3 parallel implementer subagents for #10 (picker ring), #11 (window), #12 (icons). Each ticket body has acceptance criteria + constants.
   - After all merges: run full `cargo test` once, then the `code-review` skill on `dot-matrix`; fix issues in one implementer subagent.
   - Report the integration branch to the user for HAND-CHECK (thin layer). Tickets stay open until the user confirms; closing is `gh issue close N --comment`. No PR (repo closes by direct master commits + v* tag, per prior releases).
2. Merge order after #9: any order; #11 is the largest. #12 also needs `MONITOR_LOCK_WRITE_ICO=1 cargo test ico_file` then full `cargo test`.

## State now
- Committed to master: `7c78680` docs (ADR 0001, GLOSSARY update, mockups, ledger). `dot-matrix` branch = master + (pending merges). Remote: github.com/OnlyHelion/monitor-lock.
- Subagents launched: explorer (ses_e0a5474c5ffeTX3PJYoYMEt7wh) → notes to `%TEMP%\opencode\ml-exploration\NOTES.md` (briefs for #10/#11/#12); implementer #9 (ses_e0a5474c5ffdmRADEe5Z6Uvch3).
- SETTLED (user, quoted in ledger): scope = everything; one shared dot grid, 5 px pitch @1x, 640×400; squircles (n=4, ratio 0.78); padlock 18×18; locked #FFB347 / released #E8E8EC / suspended = hollow orange; off-dots visible panel-wide; static UI + one-shot ≤300 ms orange wave from padlock on state-key change; picker ring = dotted orange + 2 chasers opposite sides (only continuous animation); short labels; status rows 2-row gap; buttons left edge = col 8.
- Tickets: #9 no blockers (frontier, in progress); #10/#11/#12 blocked by #9. All sub-issues of spec #8, ready-for-agent.
- Type defs: `LockState`/`LockSnapshot` core.rs:459-528; window size main.rs:167; overlay paint overlay.rs:153-228; icon geometry icon.rs:63-109; `win32::set_main_window_icon` :378. Renderer source of constants: `%TEMP%\opencode\render_v2.py`.
- UNVERIFIED (carried): pin-mode Alt+Tab release on this Win11 build; if Alt+Tab holds the lock, add the foreground class to `is_shell_ui_class` (src/win32.rs).

## Don't retry
- Image generation (grok-imagine "Invalid credential" ×2) — procedural renderer only.
- ViewportCommand::Icon for the taskbar — use win32::set_main_window_icon.
- Hovering the Monitor Lock window changes the clip (suspected winit) — close to tray.
- `#` in .rc files (RC1021) — use /* */. ICO count is at byte offset 4.

## Pointers
- Spec/tickets: `gh issue view 8|9|10|11|12`. ADR 0001 + GLOSSARY + mockups in `docs/` (committed). Explorer notes (when ready): `%TEMP%\opencode\ml-exploration\NOTES.md`. Ledger: `docs/ledgers/monitor-lock.md` (Dot-matrix redesign section). Release: push a v* tag.
