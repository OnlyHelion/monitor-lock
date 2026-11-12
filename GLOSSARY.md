# Monitor Lock

A Windows utility that keeps the cursor confined to a single monitor while a game is being played, so the game does not lose focus when the cursor is pushed to the edge of the screen.

## Language

**Lock**:
The active state in which the cursor is confined to a single monitor and cannot move to any other monitor.
_Avoid_: clip, cursor trap, capture, clamp

**Game monitor**:
The monitor on which the game window is displayed. The target of the Lock.
_Avoid_: primary monitor, active monitor, current monitor

**Game window**:
The window the app treats as the game: the focused window that fills its monitor.
_Avoid_: target window, active game

**Auto-follow focus**:
The activation rule: the Lock is engaged only while the game window has focus, and released the instant focus leaves it. It re-engages when the game window regains focus.
_Avoid_: always-on, manual lock, sticky lock

**Escape hatch**:
The means by which the user takes the cursor off the locked monitor: focus loss (e.g. Alt+Tab), which suspends the Lock. There is no temporary-release hotkey.
_Avoid_: unlock, release key, pause

**Monitor override**:
Pinning the Lock to a user-chosen monitor instead of following the game window's monitor. A pin drops the fills-monitor requirement and is click-driven: any focus change suspends the Lock (the pin stays armed); a click inside the pinned monitor re-engages it on any real window; a click outside it while suspended drops the pin and returns to auto. Shell UI (desktop, taskbar, Start, the Alt+Tab switcher) never locks.
_Avoid_: manual monitor, fixed monitor

**Monitor picker**:
A mode in which the cursor is free across all monitors, the monitor under the cursor is outlined with a thin yellow border, and a click pins that monitor as the Lock target.
_Avoid_: monitor selector, pick mode, monitor chooser
