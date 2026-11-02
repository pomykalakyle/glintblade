# Glasswater central ravine

Scope: the central rift beneath the bridge, x704..1008/y128..832. The standalone
distributor and its floor apron belong to another thread. Existing wall caps,
water boundaries, collision, bridge masonry/clearance and all routes are retained.

The old abyss used three detached untextured polygons. `glasswater/ravine.rs`
now continues the final native natural-bank silhouette downward, with a small
1:8 inward batter at side returns. The bridge/soffit cannot seed these faces.
Lower faces render only into actual missing-ground pixels. Depth is precomputed
at region build; no per-frame ray marching, extra emitters, fog or particles.

PixelLab Pro job `01b8b641-76e1-49d2-a6d9-642276148af6` supplied four128px
slate-face candidates (10generations). Selected0 and3 for their broad slabs and
fractures;1's curved strata and2's dense crossbars were rejected. Raw candidates,
request, seed, palette, receipt and reference path are preserved here.

Pro OpenGL normal job `c97a5aef-8a5d-4fbf-9745-45a9006b6ea9` supplied the
registered256x128 source normal (0.5generations). All surfaces are authored
upright rock: tangent X/Y are restrained to0.7, forward Z has a0.75 minimum,
then vectors are normalized. Maximum facet tilt is24.017degrees. Normal vectors
are data; no gamma/palette transform is applied.

An initial direct128px vertical repeat was rejected after the lower-rift render
exposed a straight horizontal seam. Two128x704 native strips now use connected
minimum-error cuts through32px overlaps of the selected source pixels. The exact
same source coordinates remap the normal atlas. Neither color nor normal sources
are resized, mirrored or rotated. Runtime YUV is worldY-128, with no vertical
wrap inside the rift. The dark center and asymmetric bank shapes remain visible.

Rebuild **only this asset pair**, without network calls or the umbrella exporter:

```sh
uv run --with pillow python tools/pixellab/glasswater_ravine.py \
  --out assets/sprites --normals-out assets/normals \
  --receipt art/pixellab/2026-11-02-ravine/install.json
```

Near hanging ledges retain source-registered light-reactive upright normals and
their established height0 convention. Deep continuations deliberately remain
distant backdrop, excluded from direct lights/receivers/casters. The renderer's
unsigned terrain heights do not describe below-floor depths; adding positive
heights here would create false raised occluders. No shared renderer changes.

Private matched native views, full-region context, actual input crossing/edge
replays and verification reports live in this thread's scratch `audit/` folder:
`/home/krp/.t3/scratch/2026-11-02-improve-glasswater-cavern-and-ravine-c04975f8/audit/`.

## Walking-edge transition follow-up

Kyle liked the deep rock but identified abrupt90degree folds where walkable
floor met the new face. The old native ledge depth only measured downward from
north edges, so lateral rims had no shallow transition at all.

`glasswater/ravine/rim.rs` now follows actual2D FLOOR/VOID contacts. Unequal
geological runs carry3..9px floor shoulders into three descending cut planes.
The largest native fractures cross the fold; normals tilt outward with the
authored slope. Existing raised caps, bridge masonry, water and movement masks
are protected. Below-floor stone retains the renderer's established height0
convention and cannot become a raised shadow caster. No new bitmap assets,
global lighting effects or renderer changes were needed.

Native matched bridge/ledge/rift/lower views, two light phases, full overview,
and three exact before/after real-input reports are in `audit/rim-transition/`.
All48Glasswater tests (including2new shoulder geometry/locality tests), native
and WASM builds pass. The crossing, bridge-edge and lateral-brink reports match
the pre-change renderer exactly, including trace/audio/events. The approach
frame confirms feet stop where the shoulder visibly turns down.
