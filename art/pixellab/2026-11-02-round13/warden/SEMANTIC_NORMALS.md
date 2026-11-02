# Structural normal correction, 2026-11-02

The prior generated maps and connected-color averaging were rejected after the
perspective re-audit. Broad upright foot/front pixels pointed down towards the
floor; for example (34,105) had world Z=-.758. Averaging a wrong direction into a
flat patch did not establish a correct architectural plane.

This revision changes **only the normal atlas**. The complete albedo PNG and
animation JSON export remain byte-identical. Raw PixelLab normal maps stay in
this archive for provenance and are no longer exporter inputs. This document
supersedes the original README's claims about generated normal geometry.

`semantic-planes.json` explicitly assigns native registered polygons to stone
faces, with pose-specific masks where the braced buttresses and open sluice
change. The default surface is a vertical stone front/recessed chamber wall.
Caps, pier roofs and projecting sill tops face up; literal returns face east or
west and towards the viewer. Albedo brightness and outline distance do not infer
geometry. Damage only removes coverage; all 17 folds translate the same normals
with their native stone pieces. Registration stabilization applies to the fixed
upper attachments, seam columns and rigid piers, avoiding accidental transfers
between articulated surfaces that happen to share a slate shade.

World axes are X right, Y south on the floor, Z up. Upright OpenGL PNGs encode
world [X,Y,Z] as tangent [X,Z,Y]. Thus vertical fronts encode (128,128,255), roofs
(128,255,128), and east returns (230,128,204), before runtime normalization.
`semantic-face-overlay4x.png` shows green roofs, purple fronts, pink east returns
and cyan west returns over unmodified albedo. `semantic-world-lights3x.png`
compares left/front, right/front, frontal and straight-up world-space lights.
These are offline Lambert diagnostics, not substitutes for rebuilt game captures.

Independent enlarged-albedo samples used by the regression checks include:

| Native coordinates | Physical face | Expected world normal |
| --- | --- | --- |
| (45,103), (78,93), (34,105), (79,105) | Pier/front stone | [0,1,0] |
| (60,28), (22,28) | Lintel/tower vertical fronts | [0,1,0] |
| (24,5), (90,16) | Tower caps | [0,0,1] |
| (35,87), (79,88) | Exposed pier roofs | [0,0,1] |
| (33,18), (101,57) | Tower/buttress east returns | [.8,.6,0] |

The six tests in `tools/pixellab/test_round13_warden_semantics.py` check these
physical directions and responses to overhead/front/side lights, fixed
attachments, damage alpha, all fold depths, all atlas alpha and byte-identical
albedo/metadata reproduction. All 25 round13 Python tests pass. Exact before/after
normal and unchanged art hashes are in `semantic-normal-correction.json`.

Regenerate offline:

```sh
uv run --with pillow python tools/pixellab/round13_warden.py \
  --out /tmp/warden/sprites --normals-out /tmp/warden/normals \
  --review /tmp/warden/review
uv run --with pillow python -m unittest discover \
  -s tools/pixellab -p 'test_round13_warden_semantics.py'
```

Live surface receivers, directional lights, cast shadows, ambient lighting and
all boss phases require the root's newly rebuilt game review. This worker did
not change collision, anchors, body shapes, authored poses, runtime code,
head/fist/crystal/heart maps or combat behavior.
