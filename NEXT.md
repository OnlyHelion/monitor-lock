# NEXT — Monitor Lock
Updated: 2026-11-12 14:25 · OpenCode (qwen3.8-27b) · b624620 clean (master)

## Next step
Nothing open. Next release: push a new tag `vX.Y.Z` on master (the `Release` workflow builds and publishes monitor-lock.exe).

## State now
- master at b624620, working tree clean (git status).
- v0.1.1 is the latest GitHub release, published 2026-11-12 14:22 (gh release view: monitor-lock.exe + .sha256).
- v0.1.1 change: the exe embeds the amber tray padlock (build.rs + res/icon.rc + res/icon.ico via embed-resource; Cargo.toml 0.1.1).
- UNVERIFIED: user's hand-check that the exe shows the amber padlock (Explorer: right-click exe → Properties → General). If they want the GRAY padlock instead: regenerate res/icon.ico with rgb (136,136,136), rebuild, new tag.
- UNVERIFIED: pin-mode Alt+Tab release on this Win11 build; if Alt+Tab still holds the lock, read the foreground class during Alt+Tab and add it to `is_shell_ui_class` (src/win32.rs).

## Don't retry
- Hovering the Monitor Lock window changes the clip (suspected winit); the workaround is closing to the tray. Details in the ledger.
- In .rc files `#` is a preprocessor char (RC1021), not a comment — use /* */.

## Pointers
- docs/ledgers/monitor-lock.md (full history of the fixes and user reports)
- Icon sync guard: `app::icon_tests::the_ico_is_the_amber_tray_icon` (src/app.rs); the padlock art lives in `padlock_rgba` — regenerate res/icon.ico if it changes.
