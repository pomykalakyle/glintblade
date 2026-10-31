# Pixel lint fixtures

Written 2026-10-30.

Run from the project root:

```sh
uv run --no-project --with pillow --with numpy python3 tools/pixel_lint_tests/test_pixel_lint.py
uv run --no-project --with pillow --with numpy python3 tools/pixel_lint.py tools/pixel_lint_tests/deliberately_bad.png --palette assets/concept/palettes.json
```

The second command intentionally returns exit code 1 because its image contains a semi-transparent pixel.
The suite regenerates all fixture PNGs and manifests in this directory.
It also invokes the unchanged public CLI on the live sprite directory, including from a different working directory.

| Fixture | What it exercises |
| --- | --- |
| `deliberately_bad` | Three 32x32 cells containing alpha 100, dense isolated pixels, near-duplicate paint, hundreds of gradient colors, off-palette paint and doubled staircases beside 1px details |
| `clean` | Native clustered sprite with a deliberate one-pixel emissive eye |
| `hidden_rgb` | Same visible sprite with different RGB values under alpha zero |
| `indexed` | Same sprite in indexed PNG format with transparency |
| `upscaled` | Exact 2x replication, reported as a possible scale problem |
| `solid` | A flat rectangle that must not be called a proven scale error |
| `empty` | Fully transparent image, reported as an error without crashing |
| `invalid_cell`, `invalid_row`, `invalid_durations`, `outside` | Invalid geometry, negative row, zero duration and out-of-bounds animation |
| `custom` | Off-palette emissive paint requires a note to become an unscored palette exception |
| `many_colors` | 16,384 colors, bounded pair comparisons and full nearest-palette checking |
| `frame_native`, `frame_clean`, `frame_broken` | Native frame, exact 2x frame and one deliberately corrupted display block |

Calibration on 2026-10-30 covered all 39 then-current sprite sheets.
The final pass had no format errors and two review warnings: `hud_shard.png` uses undocumented `#2a1d6a`, and `summit_tiles.png` has touching `#435078`/`#4c5a80` near the contrast review cutoff.
The bad fixture scores 11/100; the clean, indexed and hidden-RGB fixtures score 100/100.
The real sheets score 98 to 100, which measures mechanical findings only.

To limit false alarms, isolated eye/glint details, a few scattered fragments, documented custom paint, and ambiguous lighting statistics do not automatically cost points.
The lone-pixel budget scales with visible cell area; dense 5x5 speckle still gets a separate warning.
Common run factors and doubled staircase windows are evidence to inspect, not proof of mixed resolution.
Color-distance and count budgets are project heuristics described in [ART_RESEARCH.md](../../ART_RESEARCH.md).
