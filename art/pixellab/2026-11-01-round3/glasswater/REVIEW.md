# Glasswater full-region review

Created 2026-11-01 · Updated 2026-11-01

Baseline: six actual native 384×216 views and a stitched whole-zone overview under
`out/pixellab/round3/glasswater/baseline/`. The reviewed positions are lake approach
(300,416), island (500,440), gallery (550,102), bridge (850,250), grove (1200,230),
and heart pool (1170,700). Same time: 90 ticks. These are engine-rendered images,
not generated mockups.

## Baseline findings

- Most dry ground is one dark value with sparse long rectangular slab outlines.
  At native size the floor is either empty or shows thin boxes; those boxes do not
  explain the cave terrain.
- Walls have repeated cuboid facets, while tall wall tops collapse to blank polygons.
- The rift has long unarticulated ledge faces around an otherwise readable bridge.
- Small crystals repeat procedural needle shapes; the four big crystals are better
  but have an entirely separate construction style.
- Three waterworks distributor fins are narrow, flat and mechanically repetitive.
- The cool ambient leaves most material hierarchy nearly invisible. The animated
  water, shoreline masks, reflections and route are worth keeping.

## Integration contract

- Native source pixels, binary alpha, no scaling/rotation/blur.
- Generated walls and floor are material fills clipped to the existing dry masks.
  Collision geometry, animated water, ripple field, shore lapping, island mirror,
  bridge and abyss holes remain separate and protected.
- Generated crystal roots keep native anchors and y sorting. Pale broad faces are
  material; only the very top pale cap emits.
- Actual views and movement must be re-reviewed after source and asset export.
  An asset sheet does not establish scene quality.


## Full-scene rejections and repairs

1. Applying wall-top art by edge distance made contour bands. Kept separate native top and
   vertical-face samplers tied to world coordinates instead.
2. The first floor samples added a dark one-pixel frame despite the prompt. It produced
   an obvious grid in the real gallery. Repaired only the outer three rows/columns to
   the main slate; retained source interiors and source files.
3. Wall candidate 4 made distracting horizontal bands when cropped to variable face
   heights. The final face uses the broad vertical fractures from candidate 3.
4. A cropped 64px wall-top texture showed periodic square framing in the real bridge.
   Rejected it and moved to continuous native 128px planes.
5. The first 32px floor distribution put details in every cell, making the grove too
   evenly marked. 60% of cells now stay blank, with independent stable variant choices.
6. Generated fungi need more room than the original tiny patches. Moved the grove
   patches off crystal bases and gave the native rock roots small collision footprints.

Final paid use: 76 subscription generations, 0 pending; no purchases. `ledger.json`
contains actual charges for each request. The failed art directions remain documented
rather than disappearing from the quality record.

7. The unrestricted128px strata looked passable in native camera views but the whole-zone
   overview exposed diagonal zebra repetition. Reduced the macro to three existing slate
   swatches and restricted it to attached wall/ledge beds and20px upper caps. This keeps
   open combat floors quiet and breaks the repeated whole-map pattern.

## Final scope and known limits

The pass covers every Glasswater chamber: shared receiving stone/vertical faces and
rift ledges; eight large and eight small native crystal variants; three waterworks
structures; four native shelf-fungus families with grounded roots. The cool ambient
rises from0.72 to0.84 so the mineral/body hierarchy survives actual gameplay size.
Shore shapes, water colour fields, reflection/ripple simulation, bridge geometry,
encounters, objectives and route remain authored by the region.

The layout itself remains the original map. Small sprigs, boulders and individual
fungi remain procedural secondary detail. Some wall-top masses intentionally stay
flat/dark instead of spreading repeating texture across every background polygon.
Native geometry does not imply every room has a new bespoke architectural landmark.


## Final engine verification

- Captured and inspected all six native views and the stitched whole-zone overview from
  the immutable `out/pixellab/round3/candidate-v1/glintblade` with default embedded assets,
  no overlay. Outputs: `out/pixellab/round3/glasswater/final/`.
- The revised material domains remove the whole-cave zebra pattern; floor remains calm
  through combat lanes and rock detail attaches to walls/ledges. Large wall-top masses
  deliberately stay quiet. Crystal roots, fungus roots, ledge faces and waterworks feet
  all remain grounded in the reviewed native views.
- Four-second actual input audit: gallery wading, return, approach crystal and two slashes.
  Water footsteps, both deflect hits, moving ripples/reflections and0 deaths recorded in
  `final/water-motion/report.json`. These30 audit frames are7.5fps inspection samples,
  not a presentation clip; the root agent records final gameplay at60fps with audio.
- A shared UI issue remains outside this region's ownership: the arrival title banner
  covers the central crystal/attack area during this short entry audit. Sent to root.
- Eight atlases validate native dimensions, binary alpha and base/anchor contact.
  Python packer compiles and `git diff --check` passes. The root runs final native/WebAssembly
  builds and the complete gameplay/test suite after independent all-region review.
