# Design reference: dot-matrix aesthetic

Source: user-supplied images, 2026-11-19. Status: **implemented** (spec #8, tickets #9-#12, branch `dot-matrix`).
Decisions: `docs/adr/0001-dot-matrix-design-language.md`. Terms: `GLOSSARY.md`.
Source of truth for pixels: `src/dotmatrix.rs` (palette, font, padlock, renderer, wave, ring; pinned by tests).

## Reference images (user-supplied)

| File | What it shows |
|---|---|
| `01-dot-grid-texture.jpg` | Dense grid of grey/white dots on near-black: the "dot-matrix panel" texture. |
| `02-coral-diamond-dots.jpg` | Coral-orange dot cluster on a sparse dark dot grid: a dot-rendered glyph in an accent colour. |
| `03-dot-matrix-microwave-icons.jpg` | Stock icon sheet: white dot-matrix icons on black, unlit dots visible. |
| `04-real-dot-shape.png` | The user's sample of a "real dot" (squircle, see ADR 0001). |
| `current-icon-256.png` | The app icon before the redesign. |

## Mockups (`mockups/`)

| File | Status |
|---|---|
| `04-glyph-a-padlock.png`, `06-padlock-states.png`, `07-window-mockup.png`, `08-picker-ring.png` | Approved. Rendered by `tools/render_v2.py`. |
| `05-glyph-c-monitor.png` | Rejected option (monitor glyph, old coral colour; the user picked the padlock). |

Re-render: `python docs/design/dot-matrix/tools/render_v2.py` (writes into `mockups/`).
