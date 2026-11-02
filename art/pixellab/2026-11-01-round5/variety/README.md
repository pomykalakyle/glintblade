# Rooted gardens and native enemy morphs

Kyle's feedback: tree crowns slid back and forth; mature trees and intact
pillars repeated conspicuously; stone feet needed a softer ground contact;
enemies needed visible artistic variety.

## Trees

Direct inspection covered the previous full Bloomwild overview, native entry,
grove, courtyard and temple, actual campaign frames 30/70/110, and matched
stationary entry frames at ticks 20/70. The old packer translated the entire
canopy layer one pixel sideways between two frames. Trunks were fixed, but the
dense crown and dark branch clusters moved as one board.

All seven tree crowns now keep the same native geometry. Wind remains visible
in rooted grass, detached petals and ambient leaves. The original four mature
rose trees and sapling remain. One targeted PixelLab Pro call produced four
upright ivory-blossom companions; candidates 2 and 3 supplement the original
family as variants `f` and `g`. Open vertical branching and pale blossoms
provide a distinct silhouette and material, without scaling or recoloring.

- Job: `f3c0a77c-39e0-4585-87f6-ccdaf85a7b9a`.
- Cost: 10 subscription generations for four candidates.
- Prompts, submission/result receipts, untouched PNGs and selection reasons:
  [trees/](trees/).
- Export: `tools/pixellab/pack_trees.py`; offline rebuild remains part of
  `tools/pixellab/build_assets.py`.
- Native 80×96 pixels are only vertically aligned, then separated into trunk
  and canopy for the existing occlusion system. Their first composite is
  asserted equal to the aligned source. Opaque native roots provide exact
  support rectangles.
- Authored placement mixes seven silhouettes and avoids consecutive identical
  crowns. Temple columns retain a coherent colonnade but include contrasting
  short/tall broken survivors.

## Ground contact

Ground contact follows the actual opaque native support columns, affecting at
most two existing floor pixels below each foot. Irregular damp earth/moss is
restricted to grass; slab/path contacts use restrained warm mineral dust. The
operation skips transparent sky and water. Aqueduct piers receive the same
actual-foot contact without blocking the arch opening.

Bloomwild pillars also receive two coherent moss cushions on existing opaque
pixels in their actual lowest three rows. Alpha, emission, anchors, collision
and all higher sprite pixels remain unchanged. Using the regular sprite
renderer preserves actor reveal/dither and depth sorting.

## Enemy art

Four existing species each have their original appearance plus two stable
native-pixel field morphs. These are deterministic code variations of existing
art, not additional generated animation assets or new enemy behaviors.

| Species | Complementary silhouettes |
| --- | --- |
| Sproutling | Broad ivory fan blossom; taller forked fern and plum bulb |
| Thorn imp | Branching husk antlers; swept rose leaf crest |
| Crystal beetle | Split crest; outward fanned crystal tips, varied matte shell |
| Ember hound | Attached dorsal spines; lower basalt back plates, varied matte shell |

Each morph is derived once for every source pose. Spawn appearance does not
consume behavior RNG. Direction, pose timing, eyes, emissive attack pixels,
hitboxes and combat cues are retained. The same cells supply hurt/death bursts
and hound afterimages. First visual review caught floating hound spines; the
accepted implementation anchors every plate to that exact pose's opaque back
column. A connectivity regression now checks every added pixel against the
actual native pose, alongside source coverage and emissive-color preservation.

Native studies and reproduction harnesses are under
`out/pixellab/round5/variety/`; these are art studies, not full gameplay evidence.
