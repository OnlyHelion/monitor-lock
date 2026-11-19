# 0001 — Dot-matrix design language on every surface

**Status**: accepted (2026-11-19)

## Context

The app's visuals (egui window, tray/taskbar icons, exe icon, picker overlay)
were default widget chrome. The user asked for a redesign based on three
reference images (dot-grid texture, dot-matrix microwave icons — saved in
`docs/design/dot-matrix/`): an LED dot-matrix panel aesthetic, applied to
**everything** (user, round 1 Q1).

The app has four icon destinations fed from one source (`src/icon.rs`), one
egui window, and one hand-rolled Win32 overlay. A design-language decision
therefore constrains all of them at once, and the icon test
`ico_file_matches_the_art` pins the exe icon to the code.

## Decision

Every surface is drawn on one regular grid of equal "real dots" on a
near-black panel:

- **Grid**: pitch 5 px at 1x (3 px dot + 2 px gap), window 640×400, one shared
  grid — glyph, text, buttons and separators all snap to the same cells.
  Off-dots (unlit) stay faintly visible across the whole panel.
- **Real dot**: a circle with clipped corners (superellipse, exponent 4),
  0.78 of the pitch — per the user's attached sample
  (`04-real-dot-shape.png`); not a perfect circle, not a square pixel.
- **Colour**: locked = #FFB347 (user-specified), released = white/gray
  #E8E8EC, suspended = the same orange but HOLLOW (1-dot outline + lit
  keyhole). Palette: BG #0E0F13, off-dot #26282E, gray #888888, dim line
  #3A3D45, soft text #60636C.
- **Glyph**: 18×18 padlock (shape approved in round 2). Text is a 5×7
  uppercase dot font; button labels are short (PICK / AUTO / EXIT) so they
  fit one row. Rows of the same section line up on the grid (2 empty grid
  rows between the three status rows; buttons share the left edge with the
  heading and DETAILS).
- **Motion**: the UI is static. A Lock state change plays one one-shot
  ≤300 ms transition: a small localized orange gradient wave radiating from
  the padlock, then static. The only continuous animation is the picker
  ring: a dotted orange ring with two chasers orbiting on opposite sides.
- **Icons**: the same language at icon scale — the monitor's screen becomes
  an 18×18 dot panel showing the padlock (orange engaged, white released);
  one pure source for tray, title bar, taskbar and exe (regenerated .ico
  enforced by test).

## Consequences

- One rendering path (`src/dotmatrix.rs`, pure and unit-tested) feeds the
  window and defines the icon and overlay geometry; the approved mockups
  (`docs/design/dot-matrix/mockups/`) show exactly what the code will
  render, because they were generated from the same geometry.
- The egui window loses native widgets (no text selection, no theming);
  buttons are hit-tested in grid coordinates and the panel is one uploaded
  texture per frame.
- Dots are sized in physical pixels, so the grid snaps to the display DPI
  (pitch = round(5·DPI scale)); the layout is defined in cells, not pixels.
- The chaser orbit is quantized to the 50 ms picker tick — an LED refresh
  cadence, accepted as part of the aesthetic.
- Hard to reverse in practice: all four icon destinations, both UI surfaces
  and the pinned .ico are built on this language; treat it as the app's
  visual identity.
