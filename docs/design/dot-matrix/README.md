# Design reference: dot-matrix aesthetic

Source: user-supplied images, 2026-11-19 (chat session `quick-panda`). Request: redesign the
Monitor Lock UI in this style. Status: **in discussion** (grill-with-docs session), nothing implemented yet.

## The images

| File | What it shows |
|---|---|
| `01-dot-grid-texture.jpg` | Dense grid of small grey/white dots on near-black, randomised brightness — a "dot-matrix panel" background texture. |
| `02-coral-diamond-dots.jpg` | A diamond/rhombus cluster of coral-orange dots on a sparse dark dot grid — the hero shape; reads as a dot-rendered glyph in the app's accent colour. |
| `03-dot-matrix-microwave-icons.jpg` | Stock sheet ("UI Design \| Icon design dot matrix", WAACS): white dot-matrix icons (low/medium/high power waves, defrost, defrost bread, heat drink) on black, each with a plain sans-serif caption below. Shows the icon-design language: crisp white dots, dark unlit dots visible, generous spacing, simple geometric glyphs. |

## Style language (as read from the images)

- Near-black background (not pure black; a dark grey/blue tint is visible in the microwave sheet).
- Everything rendered as a **regular grid of dots**: lit dots (white or coral) form the figure,
  dim dots stay visible as the "off" state of the panel.
- One warm accent: **coral/orange** (≈ #F08060-ish, see image 2). The icon sheet itself is monochrome white.
- Flat, no gradients or glow on the dots themselves (any glow in the sheet is background lighting, not per-dot).
- Captions: plain, light sans-serif, lowercase-ish, small, centred under the icon.

## Open design questions (tracked in the grill session, not decided)

- Scope: main window only, or also tray/window/taskbar/exe icons?
- How literal: full dot-grid background + dot-rendered glyphs, vs. palette-only?
- Which glyph for the lock (padlock vs. the diamond shape vs. current monitor icon)?
- Colour per state (locked = coral, unlocked = white/grey?)
- Any animation on state change?
