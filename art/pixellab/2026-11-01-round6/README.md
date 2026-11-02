# Round6: full enemy art and broad scene beauty pass

Kyle’s priority: make the game as beautiful as possible. Existing enemy archetypes and combat rules remain.

Selected native build: `out/pixellab/round6/selected/glintblade`, SHA256 `0d9244d6acdb90161bb29d3174352a70c22cfe2fea44772add6056414fd1f212`. Its `sources.json` pins 267 source/asset files. No sprite overrides are used by the selected captures.

## Selected art

- Twelve complete PixelLab alternate skins: oatcap/camellia, birch/bracken, geode/scallop, moth/lantern, ivory/glassback, copper furnace/ceramic kiln. 610 alternate authored frames; 305 base frames. All 18 appearances occur among the original 38 enemy placements.
- Complete row/direction/timing/anchor coverage. Preserve planted support, exact imp seed masks, hound/totem combat emission and firing outlets. Mineral/wisp bodies use semantic charge progression around their new anatomy.
- Independent review rejected/repaired a recolor-like mushroom, cropped mineral domes, disappearing hound shoulder armor, snapping moth wing folds and airborne top-edge contacts. Native recipes retain raw generated outputs and source pixels; no resampling.
- Bloomwild: quieter meadow illumination, singular broken mosaic/fluting/ivy/damp approach details, larger distant title geography.
- Glasswater: crest-derived crystal sources, finer light bands, submerged depth terraces and faint receding rift masses.
- Cinder: flush worn copper work beds and unequal pigment/wear accents around the presses, sparse cooling fans.
- Summit: restrained radial observatory masonry and worn astrolabe inlays; quiet combat center.

## Reproducibility and account

Prompts, raw candidates, receipts, assembly recipes, selections and hashes are under `enemies/botanical`, `enemies/fire-spirit`, and `bloom-floor`. All generations used the connected paid PixelLab account through MCP. This round used 170 included generations (100 botanical/mineral, 60 fire/spirit, 10 floor studies), no credit purchase. Account snapshot: 639 used / 1,361 remaining in this cycle.

`uv run --offline --with pillow python tools/pixellab/build_assets.py --out <new-directory>` rebuilds the selected environment and all alternate atlases without network calls. See `asset-rebuild.json` for comparison.

## Review and checks

- Independent full art-frame and source review: `enemies/independent-review.json`, `enemies/independent-code-review.json`.
- All-region baseline beauty audit and implementation: `beauty-audit/`; additional review/implementation: `out/pixellab/round6/scene-independent-review/`.
- Final independent scene and actual gameplay review: `final-visual-review/`. These reports distinguish all still/pose coverage from sampled campaign frames; nobody claims to have watched all 14,498 frames or performed a human difficulty/audio audition.
- Independent physical, combat, lighting and clustered-death checks: `technical-review/`.
- `cargo test --release -- --nocapture --test-threads=1`: 161 passed, 3 manual tests ignored; one documentation example ignored. The explicit Cinder signature timing also passed separately. Renderer mean 4.08–4.86ms, p95 4.15–4.92ms; explicit caldera p95 5.44ms.
- Cosmetic death debris no longer changes shared combat randomness; the source pose determines subsequent RNG. Three skins, all six archetypes, two death phases and empty/nearly full particle buffers have matching subsequent RNG/projectiles.
- Full real-input QA route: 14,498 native frames, 241.63 seconds, original 60Hz timing, all four regions, all three shards, every Warden phase and victory. Uses invulnerability for coverage. Original game assets/event-timed audio: 1,452 events, no missing sounds. See the recording manifest rather than treating scene studies as combat evidence.

## Viewing

- `out/pixellab/round6/enemy-gallery.html`: all 18 looks, native 3× pose viewer with actual durations/directions.
- `out/pixellab/review-round6/gallery.html`: portable all-region before/after scene and map viewer.
- `out/pixellab/review-round6/motion/`: four actual gameplay cuts at original 60fps, plus compact inline preview.
- `out/pixellab/recordings/whole-game-round6/gameplay.mp4`: full QA walkthrough, original audio, exact 2× native encoding.

`verification.json` records final immutable artifacts and scoped gate results. Work is local; no publication or GitHub push was performed.
