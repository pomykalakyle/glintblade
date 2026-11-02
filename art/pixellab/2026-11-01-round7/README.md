# Glint Blade: normal-based lighting, round 7

This pass adds native normal-map surface shading and source-specific moving sprite shadows. It preserves the selected round6 albedo art, enemy skins, animations, collision and combat.

## Selected rendering

- PixelLab generated OpenGL normal maps for78 native asset families. They use exactly the source atlas grid and alpha; the engine decodes signed vectors without color/gamma processing. Upright sprites map to world `(nx,nz,ny)`, ground to `(nx,-ny,nz)`, and horizontal mirrors negate Nx.
- Every accepted visible sprite pixel writes its normal, height, material kind and draw-instance owner alongside its color. Animation, camera quantization, alpha/reveal, mirrors, foreground ownership and boss kneel clipping share the color draw path. Exact native terrain UVs retain normal metadata across all four regions.
- Actual emitters have a physical height separate from their projected Y. Direct light uses world-space distance, normal incidence and per-emitter alpha visibility. Emission, outlines, ambient/contact shading and UI are kept separate. A small bounded reference-centered sunlight term preserves the garden's existing exposure.
- Night scenes exclude the old frozen nearest-source casts, point-facing bevels and actor shadow proxies. Contact/AO and the broad molten-bank/court reception remain. Bloomwild retains its fixed sun shadows. Normal mode also removes the old drifting cloud-light overlay.
- A thin billboard initially produced a horizontal rail at column/hero contacts. Native foot-depth profiles now form finite2.5D volumes extruded along view rays (Y and Z change together), preserving the source silhouette. Hero sole selection excludes its sword; beetle legs use the full native support; floating sprites retain their height; separated crowns share their true root. Kneeling boss parts use the visible clipped foot.
- Off-screen native objects are gathered before viewport rejection so their shadows do not disappear at a camera boundary. Conservative sphere/AABB and scanline rejection bound the exact alpha-ray work; no top-N caster limit or arbitrary shadow length is used.
- Point sources also test the exact native painted terrain height sheets. Empty spatial blocks are skipped analytically; terrain masks cache in world coordinates while elevated actors are tested live. Collision tiles never invent invisible wall geometry. A direction-aware boundary traversal fixes an exact negative-boundary hang and agrees with an independent 46,384-ray oracle.
- Bloomwild explicitly owns the bounded sun term; cave, furnace and summit scenes receive only their local normal lights. Cinder's keeper brazier retains its real source, and every Glasswater crystal/sprig source follows existing glowing crest pixels, including native variants whose old emissive masks were empty.

This is a2.5D sprite renderer. The existing hand-painted light/value design remains part of the art. Native objects and actors supply finite point-light shadow volumes; terrain supplies exact painted height sheets. Neither represents unseen sides or a complete3D mesh. Same-owner sprite surfaces omit self-occlusion to preserve authored internal shading; nearby authored emissive ink associates a light with its housing. Bloomwild's sun retains its fixed static casts.

## Validation and evidence

`out/pixellab/round7/selected-r3/` preserves the final native executable, library, example, WebAssembly build, build/test logs and source/asset hashes. `out/pixellab/world/round7-r3/` contains40 native scene studies,4 overviews and the title. `out/pixellab/recordings/whole-game-round7-r3/` records the real60Hz full campaign using the existing input pilot with god mode; this validates rendering/progression, not difficulty balance. Earlier `selected/` and `selected-final/` candidates remain as historical evidence rather than the final selection.

The quiet release suite passes 199 tests, with three explicit/manual tests ignored. A separate Cinder timing run also passes at all four signature views. The completed campaign has 14,498 native frames, all four regions, three shards, every boss phase and victory. Its state trace, audio events and progression match R2 exactly. The fixed-light enemy study checks 18 looks over 240 frames; 4,241 held or identical source poses remain byte-identical. Native/WASM artifacts and all 455 build inputs are pinned in the selected directory.

The [shareable gameplay GIF](../../../out/share/glint-blade-gameplay.gif) contains four two-second excerpts from this final build: eight seconds, 768×432, 4.8 MB, with the original HUD. Its adjacent JSON records every sampled source frame. Final evidence is indexed in [verification.json](verification.json), [technical-review.json](technical-review.json) and [enemy-motion-review.json](enemy-motion-review.json).

`examples/normal_trials.rs` renders the actual engine with an orbiting source and native columns/hero. Its optional `sheet` argument provides an otherwise identical thin-caster comparison. `out/pixellab/round7/footprint-trial/` preserves both frame sequences and contact comparison. Source-local visibility tests cover finite native foot depth, transparent holes, mirrors, elevated receivers, source identity and3D spherical bounds.

Normal generation and exact offline rebuild details live in [normals/README.md](normals/README.md). The selected maps cost27.4 included generations, with no credit purchase. Near-static pose correspondence fixes unwanted normal breathing; larger authored pose deformations remain genuine changing geometry and are reviewed separately.

Native-only diagnostic: `GLINT_LIGHTING=legacy` selects the prior lighting path for comparison. Standalone `Frame` callers retain legacy behavior unless they set `surface_lighting=true` after `begin`; the world renderer enables the new pass by default. Legacy emitters without explicit height use18 native pixels; hero/enemy/boss/prop and authored decor emitters use their actual attachment heights.

The first full-campaign recording caught a fractional-hover clamp panic at tick4822. A one-pixel foot span could become a ULP narrower than1, reversing separate half-pixel bounds. Center/half-width clamping plus a regression fixes it; the incomplete first capture is preserved separately, and final validation is rerun from tick1.
