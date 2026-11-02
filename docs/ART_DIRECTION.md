# GLINTBLADE art direction (2026-10-30)

Created 2026-10-30 · Updated 2026-11-01

## PixelLab environment pass (2026-11-01)

Kyle requested PixelLab MCP, stronger landscapes, autonomous art direction and parallel agents,
with visible samples throughout. PixelLab-generated assets are now allowed; this supersedes
the first night's code-only constraint below. Keep the native grid, clear silhouettes,
purposeful color clusters and restrained atmosphere.

The environment pass now covers all four regions. Bloomwild uses fractured island faces,
weathered arches and pillars, a rooted tree family, quiet 32px floor modules, and low cuttable
grass. Glasswater adds native cave material, crystal families, waterworks and fungi; Cinder
adds broad fractured basalt, foundry markers/presses and restrained paving; the Summit adds
native masonry, foundations, piers and an empty dais. Existing maps, water/lava simulation,
character animation and most secondary props remain authored. Keep routes readable.
Generated art must be reviewed in actual game frames and motion, including shadows,
canopy fading and cutting. A palette or linter pass alone does not establish quality.

Selected sources, palette, trials and receipts: `art/pixellab/2026-11-01/` and
`art/pixellab/2026-11-01-round3/`. The seven packers in `tools/pixellab/` reproduce 31 native atlases
and three legacy collision-metadata families; this packs and splits sources,
with documented native mask and palette repairs for rejected staging borders/texture bands.
It never resizes source pixels, and untouched generator outputs remain archived. The original young tree
remains. Seven mature/young tree silhouettes now keep whole crowns fixed; ambient loose
leaves and cuttable grass can move without translating the entire tree.

Round5 favors a continuous title headland over an isolated pedestal. Actual top contours
own cliff faces, side returns and beveled retaining-wall ends. Stone feet use actual opaque
support, source-gutter-independent shadows and narrow material contact. The Summit bridge
is a shallow seated arch. Bloom water stays blue-green with sparse reflections and unequal
falling strands; Cinder's native lava quilt distinguishes crust, orange heat and bright cores.
Broken-column sources must finish with a real lower stone face and transparent cell margin:
contact moss and correct collision cannot repair a shaft cropped at its canvas edge. Selected
PixelLab footers preserve each original upper40rows, use unchanged native source pixels, and
receive fresh exact support metadata and placement checks. Small stable enemy variants preserve native poses and attack emission. See
`art/pixellab/2026-11-01-round5/` for sources, trials and independently scoped reviews.

Round6 makes beauty the primary selection criterion. Six archetypes each receive two complete
PixelLab anatomical/material designs rather than tiny recolors or decorative growths. Review
every direction and pose: armor must keep its volume when running, wings must visibly fold,
and mineral domes must finish inside their native cell. Preserve feet, held-seed masks, firing
outlets, timing and readable charge progression. Death debris follows the selected skin while
the shared combat random stream follows the original pose.

Review whole scenes, too. Bloomwild's native albedo stays crisp under softer, quieter meadow
illumination. Singular mosaic/fluting/ivy details explain its architecture; repeated props do
not need identical surroundings. Glasswater's crystal sources follow their crests, with fine
light-value steps and submerged terraces that describe water depth. Cinder's flush copper
work beds and wear belong to the standing presses. The Summit's quiet radial masonry gives
its arena a sacred focal structure without competing with attacks. Native 1× and integer zoom
views, all-region maps, complete pose coverage and actual combat are separate review gates.
Sources, prompts, repairs and scoped reviews: `art/pixellab/2026-11-01-round6/`.

Round7 uses PixelLab OpenGL normal maps for 78 native asset families, with moving local
illumination and source-specific shadows from native sprite volumes and painted terrain height
sheets. Bloomwild retains restrained fixed sunlight; drifting cloud-light bands are removed.
Crystal sources follow actual glowing crests, and the keeper brazier lights nearby material.
Native per-column foot depth avoids a flat shadow rail beneath columns and Glint; floating
actors and split tree crowns keep their real ground attachment. Shadows preserve actor ink,
emissive pixels, visible stone/crystal bodies and sky exposure; collision-only companions
cannot cast. Native and browser build defaults match. Review evidence and model limits:
`art/pixellab/2026-11-01-round7/`.

Review at the full scene and map scales as well as the individual pixel scale. Complete
walkthroughs cover live enemies, portals, every shard, later Warden phases and the ending;
invulnerability is a recorded QA aid. Test real collision contacts against the visible
bases. Physics-only proxies must not create rendering planes or shadows. When a giant
and Glint cannot both fit 216 native rows, preserve active attack readability with explicit
offscreen cues instead of silently cropping the warning. Keep video controls below the
picture, use 60fps captures, and preserve integer pixel enlargement before video encoding.

## What Kyle said
Kyle's top priority for this demo: "making it look extremely good."
Kyle looked at the AI-generated concept art in `assets/concept/raw/` and said: "A lot of these images just scream AI slop to me... They're also clearly not pixel art when you zoom in and look at them. There are all these weird artifacts and edges... I don't want it to have the AI slop look. That's important to me."

So:
- First-night constraint, superseded by the PixelLab pass above: every in-game pixel was placed by code or hand-authored grids rather than using the rejected concept images.
- Don't use the images in `assets/concept/raw/` as a style target. At most they hint at subject matter (what a region contains). Their busy, glossy, everything-sparkles look is exactly what Kyle rejected.

## The look we want
Real, hand-crafted pixel art with restraint, the kind that holds up zoomed in. Mood reference: the restraint of Hyper Light Drifter (big calm color fields, few colors per area, strong value structure, small sharp details at focal points, light used sparingly so it means something) and the living pixels of Noita. Don't copy either game's designs.

Craft rules (the core of what separates real pixel art from AI imitations; add your own judgment on top):
- One pixel density everywhere. Every asset is authored at 1:1 at 384x216. Never scale, rotate or blur sprites; never mix a 2x2 "big pixel" look with 1x1 detail.
- Hard edges only: pixels are fully opaque or fully transparent. No anti-aliasing via soft alpha, no blur, no noise texture.
- Shade in clean clusters, not speckle. Avoid pillow shading (shading inward from the outline), banding (parallel same-width color bands), orphan single pixels, and jaggies on curves (keep line segment lengths stepping evenly: 1-1-2-2-3, not 1-3-1-2).
- Few colors, chosen on purpose: use the region ramps in `assets/concept/palettes.json`. A character uses about 8 to 14 colors; a prop about 4 to 10. Hue-shift ramps (shadows toward blue/violet, lights toward warm yellow), don't just darken or lighten.
- Light comes from the upper left. Consistent everywhere.
- Outlines: characters and enemies get a dark, hue-tinted outline (not pure black) for readability, with selective lighter outline on the lit side if it reads better. Terrain and ground details don't get outlines.
- Silhouette first: every character must read as a solid shape at 1x. Detail goes where the eye should go (faces, eyes, weapon); keep the rest calm.
- Glow is rare and meaningful: the blade, eyes, crystals, lava, magic. Not every rock sparkles. No random twinkles sprinkled for "magic".
- Dithering only where it earns its place (sky gradients, soft light falloff, large shaded surfaces), in clean ordered patterns.
- Environments: clear walkable paths, a few strong landmarks, calm ground with occasional detail (not uniform noise), tiles that don't visibly repeat, edges and transitions designed (grass-to-stone, shore, cliff lips), and depth from value: dark at the frame edges and in cliff faces, light on paths and focal points.
- Animation: readable key poses, anticipation and follow-through, designed smear frames, no mushy in-betweens.

Pixel art craft guides worth reading if you want a refresher: Pedro Medeiros (Saint11) tutorials, Slynyrd's Pixelblog, Lospec's palette and tutorial pages.

## Palettes
`assets/concept/palettes.json` holds the ramps for the hero, FX, UI and each region (bloomwild, glasswater, cinder, warden = the summit), plus each region's ambient light and enemy colors. Use them. You may adjust a color for beauty; note it in your manifest's "notes".
Native-resolution palette and lighting tests are in `assets/concept/` (`palette_*.png`, `testscene_*_native.png`, `lighting_preview_*_native.png`): useful for checking ramps, not as a style target.

## Scale (native pixels at 384x216)
A starting chart; adjust within reason if a design needs it.
- Glint (hero): about 18 px tall, cell 32x32 for room for poses.
- Sproutling about 12 px; thorn imp about 18 px; crystal beetle about 18x12; wisp about 10 px body plus glow; ember hound about 24x16; spitting totem about 16x30.
- The Warden: 100 to 130 px tall assembled.
- Tiles: 16x16. Trees 40 to 70 px tall. Pillars 24 to 48 px. Crystals 8 to 48 px. Urns and pots 10 to 14 px.
- The screen shows 24x13.5 tiles: big things are a few tiles, the hero is about one tile.

## Changes Kyle approved after the AI-slop research (2026-10-30, about 20:10)
Research into how people spot AI pixel art (full checklist: ART_RESEARCH.md once written) found that, once pixels are clean, AI art is given away by its look: a constant golden-hour glow, teal/gold/purple candy color, smooth off-grid bloom, and a recognizable "Claude look" for code-drawn pixel scenes that stacks atmosphere (lanterns, sparks, smoke, fireflies, waterfalls, moon, warm/cold contrast) on every scene. Kyle approved two changes:
1. Glow stays on the pixel grid. The engine now renders glow as stepped, ordered-dithered halos at 384x216 (no smooth bloom). Don't fake soft glows in sprites or code with many near-identical colors either.
2. Each region gets one or two signature effects, not a stack, and every light has a visible, motivated source (a crystal, lava, a brazier, the blade). No global warm or golden grade over a whole region; keep zone.grade close to neutral and let the palette carry the mood.
   - Bloomwild: tall grass that sways and gets cut, and the waterfalls pouring off the island edge. Ordinary clear daylight, not golden hour. No fireflies, petal storms or light shafts.
   - Glasswater: water that ripples, and crystals as the light sources. Little else.
   - Cinder: the lava (flowing, lighting its banks) and embers that rise from it. No ash snow or heat haze on top.
   - Summit: a sparse, still starfield, and light from the Warden's heart and the arena braziers. No aurora, no visible moon.
Also from the research: detail needs hierarchy (calm areas, focal points), every detail should be nameable and make physical sense, and rendering rules must be the same across all assets.

## Kyle's feedback on the first full pass (2026-10-30, 22:35): style v2 is coming
Kyle, looking at the assets: "they still don't seem beautiful to me... it almost feels like it looks like Mario, which I don't like. Just vibe- and theme-wise it should be closer to Hyperlight Drifter I think. But not too close. Come up with your own style that makes sense."
So the bright, saturated, rounded, cheerful look (scoop-shaped pink trees, grass-green fields, cute creatures, chunky cream logo) is out. A new house style ("style v2") closer to Hyper Light Drifter's mood and theme, but GLINTBLADE's own, is being defined now and will be written here. Until it lands, don't start big art redraws; after it lands, everything moves to it.

## STYLE V2 (decided by the lead, 2026-10-30 ~23:20): this replaces the earlier look
Three explorations answered Kyle's "looks like Mario, should be closer to Hyper Light Drifter, but not too close, your own style" (out/style/A, B, C; side-by-side in out/style/compare_ABC_v1.png). Chosen: **A, "The Long Quiet"**, with two grafts.
- Base: read out/style/A/STYLE.md in full and follow it. Melancholy gardens and immense abandoned waterworks; wind-combed trees, slotted stone fins, lean creatures; dusty sage, dried rose, ash mauve, chalk stone; restrained daylight; quiet paths. Cyan belongs to Glint and the Prism (blade, eyes, shards) and nowhere else. Mock scenes: out/style/A/mock_bloomwild_3x.png, mock_cinder_3x.png.
- Palettes: assets/concept/palettes_v2.json (copied from out/style/A/palettes.json) replaces palettes.json for all new and re-styled work. Lint against it.
- Graft 1 (from C, "Hatchlight"): Cinder goes much darker: soot-black and ash ground where the lava river is the only bright thing, and long, crisp cast shadows that point away from the main light source, used as a composition tool (see out/style/C/mock_cinder_3x.png). Keep A's restraint elsewhere; don't adopt C's hatching texture everywhere.
- Graft 2 (from B, "Broken Sluice"): strict accent discipline (one sharp accent per region, used rarely) and large flat shapes with low texture noise; detail only at focal points.
- Out: lawn green, round scoop trees, candy pink, cute round creatures, the chunky cream logo, bright mid-key "cheerful" lighting, busy textures.
- Everything moves to this style: region ground painting and grades, tiles and props, enemies' and the hero's palettes (silhouettes and animation can mostly stay), the Warden, title, logo, fonts' colors and HUD. Use the reference studies (assets/reference/*/NOTES.md) for craft; this section decides the look.

## LIGHTING V2 (Kyle, 2026-10-30 ~23:15; direction by the lead)
Kyle: "I wonder if part of the reason why I don't like much of what you've shown so far is lighting-related... The goal is to make it as beautiful as possible, and isn't lighting a core thing?" Yes: our frames are lit flat (one even ambient level, a few small glows, no cast shadows, no contact shading, little value contrast, no warm-light/cool-shadow split). Lighting is now a core track. Principles (pixel-honest, on the grid, motivated sources only):
- A key light per region with a clear direction and color: Bloomwild a low, warm sun from the upper left; Glasswater the crystals; Cinder the lava river; Summit braziers and the Warden's heart, plus cold starlight fill. Shadows take the complementary cool hue, not just darker values.
- Cast shadows: props, ruins, cliffs, trees and characters cast crisp, long, stepped shadows away from the key light (projected silhouettes on the ground), used as a composition tool (style C's graft). Characters keep a small contact shadow too.
- Contact and ambient occlusion: darkening where walls meet floors, under props, in crevices and at cliff bases, in stepped bands.
- Form lighting: cliff faces, ruin blocks and big props show lit faces and shadow faces consistent with the key light; rim light on silhouettes facing a strong source (lava, crystal).
- Value structure per screen: a lit focal area, deep shadow masses, mid-tone rest; enough contrast that the frame reads at a glance. Darker overall than the current pass, except where the light lands.
- Atmosphere in depth only where it explains space (e.g. the cloud sea below the island edge fading to haze), not stacked effects.
- Glow stays stepped and on the grid; no smooth bloom, no circular light discs around Glint in daylight.
