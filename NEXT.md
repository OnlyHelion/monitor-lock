# NEXT.md — Monitor Lock

## State now
- Project: K:\monitor-lock (git repo; first commit = interview docs).
- Flow: main flow. Grilling complete — frontier empty. Design settled in conversation; awaiting user's final confirmation.
- GLOSSARY.md: 7 terms (Lock, Game monitor, Game window, Auto-follow focus, Escape hatch, Monitor override, Monitor picker).

## Design (settled, rounds 1–4)
- Standalone Windows utility, Rust, single exe `monitor-lock.exe`. UI = tray icon + on-demand window. Default icon for now.
- Game window = focused window that fills its monitor (fills-monitor only — no any-window fallback, no game list).
- Auto-follow focus: engage while the game window is focused; ANY focus loss (Alt+Tab) releases immediately and unconditionally; re-engage on re-focus. No temp-release hotkey.
- Lock target: auto = Game monitor; override = pin via the Monitor picker (explicit trigger from window/tray; thin yellow border on the hovered monitor; click pins; Esc cancels; "back to auto" available).
- Exclusive fullscreen: no-op (game owns the mouse). No auto-start at login; no settings persistence.

## Build details (my calls, flagged to user)
- Cursor clip (ClipCursor) is global Windows state and survives process death → on startup clear any stale clip; on exit release it.
- If the game regains focus while the cursor sits on another monitor, move the cursor to the game monitor's center on engage.
- Per-monitor DPI awareness; ~95% coverage threshold for "fills its monitor"; foreground watched via poll (~100 ms) or WinEvent hook — pick whichever proves robust.

## Next step
- User confirms shared understanding → /implement (single-session build, driving /tdd), close with /code-review.
- TDD slice order: monitor geometry → fills-monitor detection → lock engage/release → picker (yellow border + pin) → tray + on-demand window.
- Real pass = user's in-game hand-check with their own borderless game.

## Open assumptions (UNVERIFIED)
- "borderless/windowed" = windowed-fullscreen (fills the monitor, no border).
