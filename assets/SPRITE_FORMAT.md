# Sprite sheet format (read by the Rust engine)

Every sprite sheet is two files with the same base name in `assets/sprites/`:

- `<name>.png`: RGBA PNG, fully transparent background. Pixels are either fully opaque (alpha 255) or fully transparent (alpha 0); no semi-transparency, no anti-aliasing. Drawn at 1:1 at the native 384x216 resolution (each pixel you draw is one game pixel).
- `<name>.json`: the manifest, for example:

```json
{
  "cell": [32, 32],
  "anchor": [16, 28],
  "emissive": ["#d8fbff", "#5ff2ff"],
  "anims": {
    "idle_down":  { "row": 0, "frames": 4, "fps": 6 },
    "run_down":   { "row": 1, "frames": 6, "fps": 12 },
    "attack1_right": { "row": 7, "frames": 4, "fps": 18, "durations": [3, 2, 3, 5] }
  }
}
```

- `cell`: [width, height] of one frame. The sheet is a grid of cells; an anim uses `frames` consecutive cells in row `row`, starting at column 0.
- `anchor`: [x, y] inside a cell that sits on the entity's ground position (usually the middle of the feet / base). The engine draws the cell so that this pixel lands on the entity position, and uses it for y-sorting. An anim may override it with its own `"anchor"`.
- `emissive`: exact colors (hex) that glow: drawn unlit at full brightness and fed into bloom (blades, eyes, crystals, lava, magic). Keep them few and deliberate.
- `fps`: playback rate. Optional `durations`: per-frame lengths in 60 Hz ticks, overriding `fps`.
- Directions: author `_down`, `_up` and `_right` variants where facing matters. The engine mirrors `_right` for left. Anything direction-agnostic has no suffix.
- Extra fields are allowed (the engine ignores what it doesn't know). Useful ones: `"footprint": [w, h]` (solid collision box centered on the anchor, for props), `"breakable": true`, `"light": {"color": "#ffb36b", "radius": 40, "offset": [0, -20]}` (a point light the prop casts), `"layer": "canopy"` (drawn above characters), `"scarf_anchor": [x, y]` (hero only, see the hero brief), `"notes": "..."`.

Several small sheets are better than one giant one: e.g. `bloomwild_props.png` for small props and `bloomwild_tree.png` for a large tree with a bigger cell.

Previews: render each sheet scaled 6x or 8x (nearest neighbor) over a background color from its region, plus animated GIFs for animations, into `assets/previews/`. Look at them yourself and fix what reads badly before you finish.
