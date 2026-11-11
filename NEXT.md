# NEXT.md — Monitor Lock

## State now
- Project: K:\monitor-lock (git repo). Setup complete + committed: AGENTS.md (## Agent skills block) + docs/agents/{issue-tracker,triage-labels,domain}.md.
- Tracker: **GitHub** (gh CLI). Repo NOT pushed yet (no remote) → after first push: `gh label create` the five labels (noted in issue-tracker.md).
- Flow: main flow. Grilling done, /setup-matt-pocock-skills done. Back in /to-spec — seam confirmation is the only remaining blocker.
- GLOSSARY.md: 7 terms (Lock, Game monitor, Game window, Auto-follow focus, Escape hatch, Monitor override, Monitor picker). No ADRs.

## In flight
- /to-spec step 2 seam question awaiting user yes/no: pure LockCore (detection + lock/picker state machine + geometry) behind ONE SystemAccess seam (in: monitors/foreground/cursor; out: set_clip/move_cursor); unit-tested with a fake system; thin Win32/UI layer verified by in-game hand-check.

## Next step
- User confirms seam → write the spec (to-spec template) → publish via `gh issue create` with `ready-for-agent` (needs the repo on GitHub first; if not pushed yet, ask user to push or hold the spec in a file) → /implement (single session, /tdd slices: geometry → detection → lock state machine → picker → tray/window) → /code-review.

## Open assumptions (UNVERIFIED)
- "borderless/windowed" = windowed-fullscreen (fills the monitor, no border).
- GitHub repo owner/name unknown (not created/pushed yet).
