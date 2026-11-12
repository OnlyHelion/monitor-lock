# Monitor Lock

Keeps your mouse on one monitor while you play, whether the game is borderless, windowed-fullscreen or just a window. Windows only.

Multi-monitor players know the problem: you flick the mouse at the edge of a game and the cursor lands on your second screen, and your next click defocuses the game. Monitor Lock confines the cursor to the game's monitor while the game is focused, and lets go the moment you leave.

## How it works

Monitor Lock lives in the system tray and has two modes.

### Auto mode (default)

The lock engages when the **focused** window fills a monitor (at least 98% of its full bounds, taskbar area included). A maximized browser does not count, and a window that fills a monitor but is not focused never locks.

- Focus the game: the cursor is confined to that monitor.
- Alt+Tab (or anything that takes focus from the game): the cursor is free at once.
- Focus the game again: the lock re-engages. If the cursor was on another monitor it is moved back to the middle of the game's monitor.

### Pinned mode (backup)

For a windowed game with a title bar, or any case where auto does not engage, pin a monitor: tray icon, then **Pick monitor**, then click the monitor you want. Pinning drops the "fills the monitor" requirement, so any window can be locked to it.

Pinned mode is click-driven so you are never stuck:

- Any focus change (Alt+Tab, the Win key, the taskbar, the desktop) **suspends** the lock. The pin stays armed.
- While suspended, focus alone does not re-lock. **Click inside the pinned monitor** to lock again.
- **Click outside the pinned monitor** while suspended to drop the pin and go back to auto.
- **Back to auto** in the tray menu or the window also drops the pin.

The desktop, taskbar, Start menu, search and the Alt+Tab switcher never lock in either mode.

## Using it

Run `monitor-lock.exe`. A small window shows the lock state, the target monitor and the pin. Closing the window hides it to the tray. Left-click the tray icon (or choose **Open**) to bring it back. Use the tray menu or the **Exit** button to quit.

| Tray icon | Meaning |
|-----------|---------|
| Amber padlock | The lock is engaged |
| Gray padlock | The lock is released |

The **Details** section in the window shows the foreground window, the detected game window, your monitors and diagnostics.

There is no panic hotkey. Alt+Tab plus one click outside the pinned monitor always gets you out.

## Games that fight over the cursor

Some games and tools (for example upscalers such as Magpie) reset the cursor clip every frame. Monitor Lock re-applies its clip and also clamps every mouse move that would leave the target monitor, so the cursor stays put. The **Details** section counts how many times another program reset the clip. A large number there is normal for those setups.

## Build

You need the Rust toolchain on Windows.

```
cargo build --release
```

The binary is `target\release\monitor-lock.exe`. Run the tests with `cargo test`. A few tests use the real cursor and clip for a moment, so do not touch the mouse while they run.

## Troubleshooting

- **A crash leaves no window:** release builds have no console. Panics are written to `monitor-lock-panic.log` next to the exe, and the cursor is released.
- **The cursor feels stuck on one monitor:** exit from the tray. Exiting always releases the clip.
- **Auto does not engage for a borderless game that only covers the work area:** it looks the same as a maximized window, so use **Pick monitor** instead.

## Project layout

- `src/core.rs`: the lock logic, a pure state machine with unit tests.
- `src/seam.rs`: the one interface between that logic and Windows.
- `src/win32.rs`: the real Windows implementation (monitors, clip, focus, mouse guard).
- `src/app.rs`, `src/overlay.rs`, `src/main.rs`: the tray, the window, the monitor picker outline and the polling loop.
- `GLOSSARY.md`: the project's vocabulary.
