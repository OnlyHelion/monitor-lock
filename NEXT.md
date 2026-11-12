# NEXT.md — Monitor Lock

## State now
- Project: K:\monitor-lock (git; master = 2 docs commits, pushed: github.com/OnlyHelion/monitor-lock, public).
- /implement-spec for spec #1: ALL agent-doable work DONE (2026-11-12). Integration branch `implement/spec-1-monitor-lock` @ 22d7c41 pushed to origin; `cargo test` 43/43; `cargo build --release` OK.
- Release exe: `K:\monitor-lock\target\release\monitor-lock.exe` (7.9 MB, 2026-11-12).
- Tracker: #2–#4 CLOSED with comments; #1 open (spec); #5 OPEN for the USER — in-game hand-check, steps in its #5 comment (issue comment 6812768234). Worktrees removed; PC shut down (user permission 2026-11-12 01:22).
- Only remaining work is #5: the user plays a real borderless game and confirms. Nothing in #5 is done until the user confirms in-game.

## Next step (for the user / next session)
1. Run the hand-check per the #5 comment: run the exe; start a borderless game filling a monitor → cursor confined to that monitor's FULL bounds (incl. under the taskbar); alt-tab away → release; a maximized browser must NOT engage; picker (tray → Pick monitor; click = pin, Esc = cancel; Back to auto); exit clears the clip.
2. If a borderless game covering only the work area does NOT engage: lower `FILL_THRESHOLD` in `src/core.rs` (0.97; a maximized browser measures 95.6–96.7% of full bounds), `cargo build --release`, retest; record the final value in a comment on #5.
3. Watch items: a click on the on-demand window during the picker also counts as the global pin click; border fixed at 5 physical px (no DPI scaling); pinning does NOT make windowed games engage (open spec item).
4. On user confirmation: `gh issue close 5 --repo OnlyHelion/monitor-lock --comment '<verdict>'` (and #1 if the spec is fully accepted).

## Detection edge (baked into #2's rule, arbitrated in #5)
- "fills" = FULL monitor bounds (incl. under the taskbar), single tunable threshold FILL_THRESHOLD=0.97. A maximized browser (work area ≈ 95.6–96.7% of full bounds) must NOT engage; a borderless-maximized game covering only the work area might then miss — adjust in #5.

## Open item (from spec)
- Pinning does NOT make windowed games trigger the Lock (fills-monitor activation in both modes). Flagged to user; no response yet.

## Open assumptions (UNVERIFIED)
- "borderless/windowed" = windowed-fullscreen (fills the monitor, no border).
