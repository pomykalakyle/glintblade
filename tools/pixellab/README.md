# PixelLab environment workflow

Created 2026-11-01 · Updated 2026-11-01

Thirty-one selected PixelLab sheets across Bloomwild, Glasswater, Cinder and Summit are
embedded in the default desktop and browser builds. Raw sources, selections and receipts
live in `art/pixellab/2026-11-01/`, `art/pixellab/2026-11-01-round3/` and
`art/pixellab/2026-11-01-round4/`. Rebuild the
PNG/JSON exports locally, without API calls or credentials:

```sh
uv run --with pillow python tools/pixellab/build_assets.py
```

For a candidate export, add `--out out/pixellab/candidates/rebuilt/sprites`. Packing preserves
the native pixel grid; it splits canopy layers, crops frames and registers roots. Documented
staging-border masks and restrained palette repairs are included in the region packers;
untouched originals remain archived. Broad original grass lighting is applied by the engine.

Through round4 the experiment used 429 subscription generations: 194 in the initial trials, 225
in the four-region expansion and review fixes, and 10 in the follow-up lava study.
This includes rejected terrain and concept
studies. The selected sources can be repacked without spending more.

The November 2 round13 pass adds native waterworks landmarks, a weathered
Bloomwild altar, registered Warden torso poses and authored planting beds.
Original PixelLab candidates, selected sources and deterministic registration
recipes live in `art/pixellab/2026-11-02-round13/`. The three `round13_*.py`
asset exporters are included in `build_assets.py`; isolated exports place their
normal maps beside the chosen sprite directory, or at `--normals-out PATH`.
`round13_vegetation.py` records the authored bed plan and can overlay the exact
native root positions exported by the grass invariant test. It does not change
the accepted terrain relief or generate new foliage.

`audit_round13.py` validates exported asset contracts and attached runtime
evidence. `round13_motion_audit.py` ranks temporal changes in actual native
captures for human/agent inspection. Neither tool treats a small numerical
difference or a successful asset check as a visual sign-off.

Round5 used 40 more included generations. Round6 used 170: 100 for the six botanical/mineral
skins (including rejected recolor/cropped-dome studies), 60 for fire/spirit skins, and 10 for
four native garden floor studies. The account balance after round6 is 639 used / 1,361 remaining
in this subscription cycle; no credit purchase was made. Round6 preserves twelve complete
alternate atlases (610 authored frames), assembly recipes and independent repairs under
`art/pixellab/2026-11-01-round6/enemies/`. The unified `build_assets.py --out <directory>` now
rebuilds these and the new garden floor details alongside all prior environment sheets.

## Captures

Build once with `cargo build --release`. On native builds, `GLINT_SPRITES` may point at a
directory of replacement PNG/JSON sprite sheets. The registry loads once per process; each
new capture or game launch sees the latest files. PNG-only replacements inherit the embedded
manifest, including collision footprints, anchors, animation names and draw layers. New sheets
need both files. Invalid overrides print a warning and leave the embedded sheet active.

```sh
python3 tools/pixellab/capture.py --tag baseline
python3 tools/pixellab/capture.py --tag trees-v1 --sprites out/pixellab/candidates/v1/sprites
```

Each new tag writes five fixed native Bloomwild frames, a full-zone overview, one actual gameplay
frame with HUD, an inline HTML gallery and a replay manifest under `out/pixellab/captures/TAG`.
The manifest pins the renderer and overrides by SHA-256. It fails if either changes while
rendering; existing tags are never overwritten. Captures run without a window or sound.

Example atlas contracts for compatible visual replacements:

| Atlas | Cell | Anchor | Required rows |
| --- | --- | --- | --- |
| bloomwild_trees | 80×96 | 40,90 | a–g, each trunk then canopy; identical columns keep whole crowns fixed |
| bloomwild_ruins | 48×56 | 24,53 | pillar, broken tall, broken short, drum, low wall, tall wall, statue |

Tree canopies stay separate from trunks so the player can walk beneath leaves and the renderer
can fade only the leaves covering Glint. Source-local `collision_rects` describe the opaque
structural base, including root gaps and separated arch piers. The same support controls contact
shading and sorting; source anchors use the renderer's floor rule at fractional world positions.
The painters install their finished native material masks for shore, cliff and bridge collision.

`build_bloomwild_structures.py` also applies the archived broken-column foot repair. It
checks that the upper40native rows are unchanged and that two transparent rows remain below
the completed footing, then rebuilds collision from the new opaque base. The source artwork,
literal footer registration recipe and rejected cropped trial are in
`art/pixellab/2026-11-01-round5/columns/`.

The shipped browser game keeps embedded assets. To ship a selected candidate later, place its
validated PNG/JSON files in `assets/sprites/` and rebuild rather than relying on this native overlay.

Optional ground materials use a new `bloomwild_materials` sheet with native `cell: [32,32]`.
Named animations `grass`, `slab` and `path` may have any valid row and number of frames. Source
transparency preserves the previous floor pixel; opaque pixels replace it exactly. Optional
`coverage: 0..1` on an animation (or the whole sheet) controls sparse whole-tile placement.
Modules are selected in stable world coordinates, avoiding identical immediate neighbors when
there is more than one variant. They clip to their matching physical material, keep a three-pixel
border at water/cliffs/stairs, and draw before the final altar, island rims, props and cast shadows.

Optional `preserve_fields: true` on the material sheet retains the previous broad grass daylight
and shade plan as native per-pixel light modulation (factor 0.72–1.16); stone/path colors stay
unchanged. Compare identical scenes with `capture.py --tag fields-on --sprites PATH --fields on`
and `--tag fields-off --sprites PATH --fields off`. Each capture freezes exact input atlases under
its output folder, so the comparison does not mutate the artist's working files.

When this profile is present, `live_grass_density` sets the retained fraction of existing
cuttable grass placements (18% in the selected pass; 36% if omitted), with shorter blades. Optional sheet `bloomwild_grass`, cell 32×32, anchor 16,28, anim `body`, supplies
native tuft variants. The renderer keeps roots fixed while upper leaf clusters sway by whole
pixels; collision, cutting, bending and persistent stubble stay in the original game loop.

## Review checks

Use the actual 384×216 game views and a movement/slash replay when judging an art change.
Check the arch opening, canopy fade, rooted whole-pixel sway, persistent cut grass, and
waterfall contacts as well as isolated sprite quality. Candidate atlases remain optional;
the rest of the game retains its existing maps, actors and secondary authored surfaces.

```sh
cargo build --release
cargo build --release --target wasm32-unknown-unknown
cargo test --release -- --test-threads=1
```

The single-thread test run keeps the existing frame-time assertions from competing with
each other. Run it after CPU-heavy builds and video exports finish.

## Whole-game review and motion

`capture_world.py` writes 26 matched native scene studies, or 40 with `--detail`, plus all
four stitched overviews and the actual title screen. The `--binary` argument makes
before/after replay reproducible.

```sh
python3 tools/pixellab/capture_world.py --tag final --detail
uv run --with numpy python tools/pixellab/record.py --tag campaign --pilot tour --god --until-victory --ticks 24000
python3 tools/pixellab/make_review.py --before before-round3 --after final
python3 tools/pixellab/make_motion.py --recording out/pixellab/recordings/campaign --out out/pixellab/review/campaign-motion
```

`record.py` runs actual game inputs at 60Hz, saves every frame, and mixes original sound/music
from emitted tick timestamps at frontend gains. It encodes at exact 2× before 4:2:0 compression
to retain native chroma resolution. No synthetic camera panning or damage injection occurs.
The automated campaign's `--god` flag is recorded: it verifies routes, pickups, transitions,
combat progression and ending coverage; it does not establish normal difficulty balance.

`make_review.py` produces a portable comparison viewer with scene, map and integer-zoom
controls. `make_motion.py` cuts four short samples from the real recording, retaining
60fps, original speed and synchronized audio. Samples are selected from the current trace's
region/position data rather than old route tick offsets. Its compact inline player keeps controls
below the image; the accompanying MP4 retains the full recording's quality settings.

`refresh_media.py` updates the README screenshots and clips from matching world, recording
and motion manifests. It validates source hashes, backs up the current dirty media, preserves
unmapped files, and records every installed file. Contact-only replays use
`collision_review.py --binary PATH --tag NEW-NAME`.

The round5 lava atlas quilts unchanged native126×126 interiors from two128×128 PixelLab
originals, excluding their uniform one-pixel staging borders. Minimum-error cut seams select
whole source pixels without interpolation, mirroring or palette changes. `lava_quilt.py`
records every pixel's source coordinate; `build_cinder.py` reproduces the atlas offline.
Prompts, receipts, all four candidates and the selection are preserved in
`art/pixellab/2026-11-01-round5/cinder/lava-study/`. Flow coordinates stay continuous where
the foundry canal joins the river.

Native lighting diagnostics accept `GLINT_LIGHT_TRIAL=off`, `quiet`, `balanced` or
`dramatic`. These vary the new slow weather and source-specific ground occlusion while
retaining the existing static lighting. Actual alive native scenery supplies the silhouettes;
collision proxies do not. Ground shadows exclude actors, visible native body pixels,
vertical faces and background sky. The browser embeds the selected default.

A source may sit outside the 216px viewport when the giant and Glint are too far apart.
The game frames active beam sources where possible and uses a native edge cue otherwise.
The full giant is not guaranteed to fit. Real player-pose reveal keeps foreground tree
wood from hiding Glint; support-mask colliders follow visible native stone bases.
