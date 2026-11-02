# Sculptural Weir Keeper

Selected design B: an abandoned garden weir that stood up. Thick square tower
returns, projecting buttresses and a recessed valve replace the old flat facade.
The tall slotted left tower, snapped right crown, sluice planks and culvert piers
remain the identity. The separately animated head, fists, crystals and heart are
unchanged.

## PixelLab sources and spend

| Source | Job | Included generations | Decision |
| --- | --- | ---: | --- |
| Design A | `470443b9-7c71-439a-bd25-927a28d1d8c6` | 6 | Rejected: still a flat facade, softer small bevels. |
| Design B | `0a00bd99-3d91-4d06-9631-4c3af7c00bbc` | 6 | Selected: clear massive stone planes and deep returns. |
| Brace | `b588ae63-937b-4f96-9414-4737fcac5c59` | 6 | Selected lower buttresses: strain, exposed inside faces. |
| Opening | `85a59a1b-6983-4230-b581-88a826eee7cc` | 6 | Selected central sluice: planks split around a dark valve chamber. |
| Three Pro OpenGL normal maps | See normal receipts | 1.5 | One exact registered input for each native structural key. |

Total: **25.5 included generations**, no credit purchase. Untouched original PNGs,
request descriptions, seeds, completion receipts and exact registered normal
inputs are archived here. `baseline-*` preserves the prior production art.

## Deterministic native assembly

```sh
uv run --with pillow python tools/pixellab/round13_warden.py \
  --out /tmp/warden/sprites --normals-out /tmp/warden/normals \
  --review /tmp/warden/review
```

The exporter never generates, resizes, rotates, interpolates or antialiases art.
Its source rectangles and literal structural damage cuts are in the script and
review/audit.json. Thirteen exact slate colors selected from B merge its adjacent
near-identical shades; no cyan, emission or new decorative colors are introduced.

- Cell **120×112**, anchor **(60,109)**. Existing layout/collision is untouched.
- B's footer is moved up one native pixel as a rigid piece. Its complete dark
  contour finishes at row109, with two transparent bottom rows.
- B owns upper attachments and registration seams. Brace changes lower
  buttresses/apron; opening changes only the central sluice chamber.
- Four fold rows each contain all17 compression depths. Two native culvert
  piers move up by depth and splay outward by at most2px. Their upper faces
  overlap the apron as rigid stone joints; no source row is stretched.
- Static poses alias their fold row's frame0; the two brace poses use separate
  rows. Six physical rows allocate102 registry cells instead of187. All75
  logical albedo/normal frames are byte-identical to the original11row pack.
- Runtime still moves the upper body by its existing `body_dy`. Pier motion
  cancels that displacement at the feet. Head/heart/crystals retain their old
  paths and anchors; all attack and punish-window clocks remain unchanged.
- Growing cracks use the exact same pier provenance and skip covered apron
  pixels. Intro/death dust samples the selected pose at its rounded, trembled
  world anchor. Existing burst timing is untouched.
- Structural losses create damaged counterparts. Fixed seats, collar and valve
  center survive. The existing growing crack effect remains in the fight.

Normals retain PixelLab's generated geometry. Connected constant-color stone
faces average their generated vectors into planes while contours retain bevels.
Unchanged3×3 patches inherit B's exact normal to avoid a moving light seam when
poses switch. Damage and folding apply the identical native remapping to art and
normal data; final alpha always comes from albedo.

## Review scope

Viewed the raw outputs and native/integer-zoom contact sheets, assembled head,
heart, crystals and fists, maximum16px crouch and three normal-light directions.
`review/assembled-keys-*` is a native registration diagnostic, not a game capture;
heart and fist placements are representative rest/open examples. The root task
owns live battle captures, final terrain lighting and motion review.

Independent atlas audit: all75 declared frames pass; every fold's footer plus
runtime body displacement yields exactly0px ground shift, upper30 rows stay
fixed within their damage family, gutters are at least1px, normals have exact
albedo alpha and unit forward-facing vectors. Five duplicate visible poses have
matching normals. Offline PNG/JSON/normal rebuilding is byte-identical. Rust
parsing passed; three focused torso state/ground/fracture invariants are included for the
root's test build. No shared build was run by this worker.
