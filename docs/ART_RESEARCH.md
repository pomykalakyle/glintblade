# GLINTBLADE art review checklist

Written 2026-10-30 from the three research reports and the approved changes in [ART_DIRECTION.md](ART_DIRECTION.md).
Use the first ten checks on every sprite and screenshot, then the relevant specialist checks below.
Mark a check inapplicable when it needs scene context or motion that the asset alone cannot show.

## The ten checks that matter most

1. Keep every layer on the same native pixel grid.
   Check sprites, ground, UI, shadows and effects at 1x and 8x nearest-neighbor; use integer display scaling and whole-pixel positions on the 384x216 canvas ([Saint11: consistency](https://saint11.art/blog/consistency/)).
2. Keep sprite alpha binary and edges crisp.
   Run the linter, then inspect silhouettes on both bright and dark region ground for semi-transparent fringes, blur and background-colored halos ([Derek Yu: basics](https://www.derekyu.com/makegames/pixelart.html)).
3. Make the silhouette readable before adding decoration.
   Fill each actor with one color at 1x; identify its role, facing and weapon, then find it immediately in a gameplay screenshot ([Saint11: silhouette](https://saint11.art/img/pixel-tutorials/Silhouette.gif)).
4. Give the eye somewhere quiet to rest.
   Squint at a screenshot: action and the route should lead, with broad calm ground between detail patches ([Slynyrd: top-down tiles](https://www.slynyrd.com/blog/2019/8/27/pixelblog-20-top-down-tiles)).
5. Use a small, purposeful set of colors.
   Check visible RGB colors per cell and sheet, merge indistinguishable shades, and use project ramps or explain custom hex colors in manifest notes ([Derek Yu: similar colors](https://www.derekyu.com/makegames/pixelart2.html)).
6. Build readable color clusters instead of accidental speckle.
   Inspect the linter's lone-pixel examples at 8x; eyes and glints can earn one pixel, while texture should resolve into designed shapes ([Saint11: clusters](https://saint11.art/pixel_art_articles/article2/)).
7. Apply one rendering language across all assets.
   Compare a contact sheet for upper-left base light, projection, tinted outlines, material ramps and detail scale; differences need a visible reason ([Saint11: consistency](https://saint11.art/blog/consistency/)).
8. Make every light explainable and keep its halo on the grid.
   Point to its emitter, inspect stepped/dithered falloff, and ensure nearby action remains readable in the dark ([CrossCode: lighting](https://www.radicalfishgames.com/?p=1437); [approved project rule](ART_DIRECTION.md#changes-kyle-approved-after-the-ai-slop-research-2026-10-30-about-2010)).
9. Give each region one or two signature effects rather than a pile of atmosphere.
   Compare with the lanterns, warm windows, sparks, smoke, rain, moon and waterfall stack in this [code-generated scene](https://www.spritefusion.com/blog/opus-5-5-for-pixel-art-scenes); use the approved region list below.
10. Make details survive inspection and mean something.
    At 1x and 8x, name what each prominent cluster depicts, how it attaches and why it belongs; simplify invented-looking fittings and ornamental mush ([Miguel Nogueira's critique](https://80.lv/articles/should-you-hire-an-ai-artist-or-a-concept-artist)).

## Pixel-level tells

- [ ] Avoid accidental doubled lines and staircase discontinuities.
  Check diagonal runs for a regular rhythm and curves for gradually changing run lengths; a local reversal should describe an intentional corner ([Saint11: jaggies](https://saint11.art/pixel_art_articles/article2/)).
- [ ] Separate mixed pixel density from ordinary big clusters.
  Compare repeated 2x2 staircase steps with adjacent 1px details; a solid 2x2 square alone proves nothing about scale ([Saint11: resolution choices](https://saint11.art/blog/consistency/)).
- [ ] Remove banding that makes a contour look thick or stair-stepped.
  Look for adjacent shade strips whose endpoints align or whose contours hug the outline; stagger or reshape the bands ([cure: banding](https://pixeljoint.com/forum/forum_posts.asp?TID=11299)).
- [ ] Keep flat planes flat rather than filling them with interpolated shading.
  Inspect walls, blade faces and stone tops for many tiny color increments; choose a few deliberate shade regions ([Saint11: shading](https://saint11.art/img/pixel-tutorials/Shading.gif)).
- [ ] Keep any internal anti-aliasing deliberate and sparse.
  Toggle intermediate-color pixels off at 1x; remove them if they weaken the edge, and avoid blending exterior edges toward a background the sprite will leave ([Pixel Parmesan: AA](https://www.pixelparmesan.com/anti-aliasing-fundamentals-for-pixel-artists)).
- [ ] Use ordered dithering only where the transition earns it.
  Check for checkerboard static on small or animated sprites; save the project's dithered halos and large transitions for controlled patterns ([Pixel Parmesan: dithering](https://pixelparmesan.com/blog/dithering-for-pixel-artists)).
- [ ] Review rare colors as possible dirty-tool residue.
  Locate colors used in only one or two pixels; keep intentional accents, but merge accidental near-matches into an existing swatch ([Pixel Joint critique](https://pixeljoint.com/forum/printer_friendly_posts.asp?TID=24620)).

## Craft mistakes

- [ ] Shade the volume from a source instead of shading inward from every edge.
  Inspect a material for symmetric dark rings around a bright center; distinguish pillow shading from a real curved surface or emissive center ([Derek Yu: pillow shading](https://www.derekyu.com/makegames/pixelart2.html)).
- [ ] Keep outlines consistent and readable on actual ground.
  Compare actors on the lightest and darkest traversable tiles; tinted dark outlines and selective lit edges must preserve separation ([Lospec: outline color](https://lospec.com/articles/pixel-art-outlines-part-2-using-color)).
- [ ] Let interior separations support the form without cutting it into pieces.
  Compare interior creases with the exterior outline; use softer ramp colors or broken lines where a heavy border flattens the sprite ([Saint11: outlines](https://saint11.art/img/pixel-tutorials/Outlines.gif)).
- [ ] Make shading colors work together across materials.
  Inspect ramps for purposeful hue shifts and distinct values; avoid mechanically darkening a base color at every step ([Slynyrd: palettes](https://www.slynyrd.com/blog/2018/1/10/pixelblog-1-color-palettes)).
- [ ] Reserve intense saturation and brightness for emphasis.
  View a whole gameplay frame and locate the loudest colors; ground and walls should not compete with the blade, danger and landmarks ([Pixel Parmesan: color relationships](https://pixelparmesan.com/blog/color-theory-for-pixel-artists-its-all-relative)).
- [ ] Choose tinted darks and material colors rather than default pure primaries or neutral gray.
  Sample suspicious swatches in context; white blade cores and other documented exceptions are allowed ([Derek Yu: naive coloring](https://www.derekyu.com/makegames/pixelart2.html)).
- [ ] Keep object proportions and ground-plane projection coherent.
  Align the hero, a door, a chest and a pillar on their anchors; compare top faces, apparent height and support points ([Slynyrd: top-down objects](https://www.slynyrd.com/blog/2019/9/18/pixelblog-21-top-down-objects)).
- [ ] Hide terrain seams and repeated dominant motifs.
  Tile a sample 6x6, inspect at a distance, then offset it half a tile to expose joins; use sparse variants with compatible edges ([Slynyrd: top-down tiles](https://www.slynyrd.com/blog/2019/8/27/pixelblog-20-top-down-tiles)).
- [ ] Design texture patches and transitions rather than scattering individual pixels.
  Identify the cluster vocabulary in grass, bark and stone, with bare space between patches and a calmer neighbor beside a busy texture ([Slynyrd: texture](https://www.slynyrd.com/blog/2018/2/15/pixelblog-2-texture)).
- [ ] Keep cliffs and cast shadows legible and conservative.
  Compare the brighter walkable top with the darker face; shadows must agree in direction and avoid concealing actors or the route ([Slynyrd: top-down tiles, part 2](https://www.slynyrd.com/blog/2023/3/26/pixelblog-43-top-down-tiles-part-2)).
- [ ] Preserve anatomy, equipment and material clusters through animation.
  Onion-skin consecutive frames and the loop join for changing limb thickness, popping ornaments, wandering highlights and unintended foot sliding ([Slynyrd: character animation](https://www.slynyrd.com/blog/2025/3/24/pixelblog-55-top-down-character-animation)).
- [ ] Make attacks readable through key poses and timing.
  Step through wind-up, fast swing, follow-through and recovery; use a crisp palette-colored smear and longer holds where weight needs them ([Slynyrd: attack animation](https://www.slynyrd.com/blog/2025/5/23/pixelblog-56-top-down-character-attack-animation)).
- [ ] Audit mirroring for intended asymmetry and light direction.
  Compare left/right weapon hands, scarf attachments and highlights; the engine mirrors right-facing frames, so judge the resulting images rather than assuming a correct mirror ([Slynyrd: directional attacks](https://www.slynyrd.com/blog/2025/5/23/pixelblog-56-top-down-character-attack-animation)).
- [ ] Render typography as readable native-grid shapes.
  Read actual HUD strings at 1x and check consistent glyph strokes, spacing and baseline; decorative marks must not masquerade as broken text ([Glyphs: pixel fonts](https://forum.glyphsapp.com/t/pixelate-a-font/10197)).

## Aesthetic slop tells, including the reported "Claude look"

These are review prompts about a look, not reliable evidence of who made an image.
The research journal records community recognition of a "Claude" scene style in [this discussion](https://www.reddit.com/r/PixelArt/comments/1xjdxcz/oc_a_night_to_remember/); the accessible [creator's example](https://www.spritefusion.com/blog/opus-5-5-for-pixel-art-scenes) documents the corresponding effect stack.
That is a specific reported pattern, not a demonstrated universal model fingerprint.

- [ ] Avoid treating atmosphere as a checklist of decorations.
  Inventory ambient effects in one screenshot; remove additions outside the approved region identity rather than adding a lantern, moon, fog or particle layer to fill space ([documented generated example](https://www.spritefusion.com/blog/opus-5-5-for-pixel-art-scenes)).
- [ ] Avoid applying the same warm-glow/cold-dark recipe to every place.
  Compare region thumbnails; daylight, crystal cave, lava highland and still summit should remain distinct without a shared golden wash ([HLD: environment-specific palettes](https://blog.playstation.com/?p=126359)).
- [ ] Make asset families share decisions, not just a similar surface texture.
  Compare all stone, cloth and foliage together for the same highlight logic, projection and edge treatment; matching subject matter alone is insufficient ([Robin Sloan: art direction](https://www.robinsloan.com/lab/art-directing-ai/)).
- [ ] Keep construction and decoration physically intelligible.
  Trace straps, supports, bridges, stairs and doors to their ends; remove meaningless fittings and routes that lead nowhere ([concept-art critique](https://80.lv/articles/should-you-hire-an-ai-artist-or-a-concept-artist)).
- [ ] Build specificity into landmarks instead of generic fantasy ornament.
  Ask what a ruin reveals about this island and its damage; choose a readable remnant over another arbitrary rune or spike ([HLD: world through remnants](https://blog.playstation.com/?p=126359)).
- [ ] Give repeated architecture a coherent rhythm and deliberate breaks.
  Compare pillar spacing, joints and wear; repetition may be structural, while broken pieces need a reason rather than random differences ([Saint11: ruins](https://saint11.art/img/pixel-tutorials/Ruins.gif)).
- [ ] Avoid stiffness and accidental near-symmetry.
  Locate the load-bearing leg, grip and center of weight in key poses; symmetry is acceptable when intentional, and asymmetry should persist through motion ([Slynyrd: character animation](https://www.slynyrd.com/blog/2025/3/24/pixelblog-55-top-down-character-animation)).

The approved region effect inventory is a project constraint, not an external finding ([ART_DIRECTION.md](ART_DIRECTION.md)):

| Region | Signature effects | Review against |
| --- | --- | --- |
| Bloomwild | Swaying/cuttable grass and waterfalls | Clear daylight; no fireflies, petal storms or light shafts |
| Glasswater | Water ripples and crystal light | Let these carry the scene; avoid extra ambient layers |
| Cinder | Flowing lava lighting its banks and rising embers | No extra ash snow or heat haze |
| Summit | Sparse still stars and heart/brazier light | No aurora or visible moon |

Keep `zone.grade` close to neutral and let region palettes carry the mood.
Some palette-file prose still describes the old golden-hour/firefly concept; the approved direction above takes precedence.

## What beautiful games do: review in context and motion

- [ ] Compose a stage for the action instead of giving every patch equal contrast.
  Check a grayscale screenshot and thumbnail for a clear route and focal hierarchy; use dark framing where it helps rather than forcing every room's center to be bright ([Heart Machine art director](https://www.creativebloq.com/3d/video-game-design/from-pixels-to-polygons-making-hyper-light-breaker-s-cel-shaded-world)).
- [ ] Simplify large masses and let the viewer complete them.
  Inspect canopies, clouds and rock faces for calm shapes with a few meaningful edge cuts; remove detailing that does not improve their read ([Heart Machine shape language](https://www.creativebloq.com/3d/video-game-design/from-pixels-to-polygons-making-hyper-light-breaker-s-cel-shaded-world)).
- [ ] Verify critical contrast in the darkest playable conditions.
  Capture combat in shadow: projectiles, telegraphs, the hero and pickups must still separate locally from the ground ([CrossCode developer example](https://www.radicalfishgames.com/?p=1437)).
- [ ] Review sprites under the lighting they will actually receive.
  Place each asset on its real ground beside the hero, once near a light and once in shadow, before approving it ([Sea of Stars developer interview](https://mobilesyrup.com/2023/09/11/sea-of-stars-level-design-sabotage-studio-thierry-boulanger-interview/)).
- [ ] Preserve gameplay visibility behind tall scenery.
  Walk behind trees, pillars and the boss; inspect occlusion and anchor sorting for disappearing actors and depth pops ([Saint11: top-down tricks](https://saint11.art/img/pixel-tutorials/TopDownTricks.gif)).
- [ ] Make movement describe the material.
  Watch vegetation tips move while roots stay planted, and small water highlights move coherently rather than boiling like noise ([Saint11: vegetation](https://saint11.art/img/pixel-tutorials/Vegetation.gif); [water](https://saint11.art/img/pixel-tutorials/Water.gif)).
- [ ] Stage effects so the impact remains readable.
  Step through flash, strike and fade; sparks and debris should reinforce the hit rather than obscure it ([Saint11: light magic](https://saint11.art/img/pixel-tutorials/LightMagic.gif)).
- [ ] Restore calm ground after destruction.
  Compare the end of a fight with a frame five seconds later; debris should settle and fade without masking paths or warnings ([Noita developer account](https://www.gamedeveloper.com/game-platforms/road-to-the-igf-nolla-games-i-noita-i-)).

## Automated pass and sign-off

```sh
uv run --no-project --with pillow --with numpy python3 tools/pixel_lint.py assets/sprites --palette assets/concept/palettes.json
uv run --no-project --with pillow --with numpy python3 tools/pixel_lint.py path/to/frame.png --frame -v
uv run --no-project --with pillow --with numpy python3 tools/pixel_lint_tests/test_pixel_lint.py
```

The score measures weighted mechanical findings, not beauty or AI authorship.
ERROR means invalid format/data or a demonstrated grid defect; WARN means a measurable pattern worth reviewing; `-v` exposes unscored context and ambiguous art heuristics.
Color budgets and distance thresholds are local review heuristics, not published laws: characters 16 colors/cell and 22/sheet, props 14/22, UI 16/24, tiles 10/32, with four extra cell colors for cells at least 48x48.
Near-duplicate review uses CIEDE2000 below 2.5; touching-color review uses 3.5; palette matching checks exact swatches and reports nearest perceptual distance.
Lone pixels are counted with both four- and eight-neighbor definitions; warnings focus on isolated fill/detached pixels exceeding 1% of visible cell area and a floor of four pixels (three for tiles), or three such pixels clustered within 5x5.
Only visible pixels count; documented custom colors remain visible in verbose reports, and the palette check uses all paint ramps rather than inferring a mandatory region from filenames.
The earlier researchers' measurements of 60 compressed Steam screenshots are exploratory composition prompts only; their color/value percentages are not universal thresholds or approval gates.
Sprite palette membership does not imply that every lit screenshot must contain only those RGB values: native-grid lighting can change colors ([Saint11: color consistency](https://saint11.art/blog/consistency/)).
After the automated pass, reviewers still inspect 1x readability, 8x clusters, an in-region screenshot and motion; record the cell or screenshot location and the reason for each requested change.
