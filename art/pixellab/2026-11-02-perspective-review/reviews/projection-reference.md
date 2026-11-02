# Projection and reference calibration

Verdict: the four new landmark normal maps fail a basic plane-consistency check. Garden and Summit also lose much of the neighboring architecture's visual mass. This is not a blanket failure of every new asset, and a south-facing wall is not inherently the wrong camera angle.

## Evidence and limits

Inspected accepted baseline Bloomwild columns, aqueduct arch, slab altar, Glasswater waterworks column, Cinder pillars and court, Summit pylons/dais; compared all four new landmark atlases, altar, Warden body and composited sapling. Inspected actual world renders as well as source art and normal PNGs.

Fresh unmasked scene studies use the exact published native binary in `out/beauty-pass-2026-11-02/publication-candidate/glintblade`, `zoneview --no-hero --warm 90 --scale 1`, with Bloomwild335,674 / Glasswater1304,220 / Cinder600,660 / Summit720,244. These are environment studies, not gameplay or gate validation. No builds or production source/assets changed. The earlier 40-view scenes and accepted baseline before-images remain separately labeled; they are not passed off as new runtime captures.

Annotated reference: `annotated-acceptance-reference.png`. Fresh scene sheet: `landmark-context-unmasked.png`. Full source/normal comparison: `normal-reference-grid.png`. Exact samples: `normal-plane-samples.json`.

## What the game actually projects

Renderer world axes are X right, Y south along the floor, Z up; screen_y = Y - Z. Ground X/Y and upright Z are already in projected native pixels. The renderer does not impose a single physical camera elevation or isometric 2:1 floor grid. Inferring an exact 30/45-degree camera solely from code would be false precision.

Accepted art establishes the visual convention:

- The floor's main masonry edges run horizontally and vertically in screen space, with irregular/radial local arrangements. Uprights rise vertically; their contact footprints spread along the receiving ground, rather than following an arbitrary image bottom.
- Cylindrical column caps and broad block bases expose a clear top plane; narrow frontal walls may expose much less. A frontal arch is already present in accepted baseline. Rotating every new wall to three-quarter view would be unjustified.
- A foot's screen Y includes height and projected depth. Matching the lowest opaque row to an anchor proves registration only; it does not prove plausible support, base depth or contact shadow.
- Top, side and front surfaces must have distinct corresponding world normals. The visible cap of a column should respond as upward-facing even when its image is only a few pixels deep. A broad global gradient cannot substitute for those planes.
- New structures should retain readable contact mass and local material response at native1x beside the actual neighbors. Matching alpha, collision, pixel counts and filenames does not establish that they look seated.

## Normal convention and concrete failure

Upright sprite normal conversion is `(R, B, G)` after decoding [-1,+1]; floor art is `(R, -G, B)`. Consequently upright top planes need strong positive green; upright south-facing front walls are blue; east/west faces need signed red. Altar is terrain art and correctly uses the other convention.

Representative accepted samples, converted by the runtime convention:

| Source UV | World normal | Interpretation |
|---|---|---|
| Bloom column24,8 | (-.012,.584,.812) | Clearly upward-facing cap |
| Glasswater waterworks49,13 | (-.325,.301,.896) | Clearly upward-facing cap |
| Summit pylon29,15 | (.625,.397,.672) | Sloped exposed top facet |
| New garden29,16 | (-.004,.998,.059) | Visible cornice cap is almost a south-facing vertical wall |
| New garden78,30 | (.004,.984,.176) | Upper channel plane also nearly vertical |

Across every occupied pixel in each landmark:

| New landmark | Max world Z | Pixels with world Z > .5 | Opaque pixels |
|---|---:|---:|---:|
| Garden aqueduct | .247 | 0 | 7263 |
| Glasswater distributor | .254 | 0 | 8032 |
| Cinder press | .264 | 0 | 9888 |
| Summit frame | .278 | 0 | 7659 |

All four maps consist predominantly of gentle blue-purple gradients spanning the whole object. Obvious cistern rims, capstones, plinth tops and anvil top receive no genuinely upward normal. The entire object is treated largely as a front-facing panel. The emitted PNGs can have correct dimensions, alpha and unit-length vectors and still be geometrically wrong. This explains a real part of their dark/flat integration. It cannot be repaired by simply rotating the whole normal map: different visible planes need different normals.

Minimum normal acceptance: choose explicit cap/front/side UV witnesses per asset; cap/upward bevel Z generally>.6 / >.4, front vertical |Z|<.2, signed X on actual left/right facets. Use geometric categories, not a blind fraction of all pixels, because a thin wall can legitimately have few top pixels. Check moving opposing lights in native scenes and require corresponding planes to exchange brightness while albedo cracks remain surface marks. Correct-source/normal alpha is still necessary, but insufficient.

## Ranked findings

1. HIGH confidence objective defect: all four new landmark maps omit their upward cap/base/anvil planes. Correct or replace those maps before any beauty pass or projection signoff. The old Warden and new Warden demonstrate that this pipeline can carry properly segmented plane normals; the issue is these four authored maps, not a necessary engine limitation.
2. HIGH confidence aesthetic integration regression: Garden's tall smooth arcade and Summit's broad frame read as much flatter structures than adjacent volumetric columns. The wrong normals amplify this, but their albedo also uses large uniform faces and thin base/cap silhouettes. Garden baseline had chipped brick mass, heavy feet and richer local construction; Summit baseline had an open dais framed by deeply modeled slotted pylons. The replacement's stronger outline/flat planes are conspicuous at1x. This is an art/shape judgment supported by side-by-side views, not proof that a south-facing wall is geometrically impossible.
3. MEDIUM confidence composition regression: Summit's new frontal frame is placed immediately in front of the existing socket/dais and repeats the neighboring slotted pylons' role. It makes the platform's central steps/socket harder to read and reduces the earlier quiet opening. Simply passing the floor mask does not establish that this addition improves the composition. Recommend remove/reconsider the frame if corrected-lighting A/B still reads as an extra wall.
4. LOW confidence altar projection concern, not a confirmed P2: the new central mortise is approximately13x14px, while the accepted old inset glyph was40x30px; the new small hole reads more head-on/top-down. The outer octagonal floor silhouette remains close to baseline (old~65x49, new60x50), and old/new both paint a shallow hanging front at height0. Therefore height0 or broad octagonal shape alone does not justify rejecting the altar. Its broader cracks/dark front are aesthetic tradeoffs; inspect its final cardinal relighting but do not falsely call it upright.

## Minimum useful change

First fix the four landmark normal maps semantically; do not spend another round merely proving their masks/colliders match. Re-render unmasked native before/after scenes with the same sun and local lights, plus hero approach/exit.

Then judge Garden and Summit against their actual baseline art. If their contact mass and quiet monumentality remain weaker, restore the baseline Bloom arch and remove the redundant Summit frame rather than adding more detail or transforming every asset. Keep Glasswater distributor and Cinder press provisionally: their source art does communicate functional machinery and has visible volume that correct normals can support. Their floor/shadow specifics require the separate contact audit; this report does not certify them.

New Warden has plane-varying cyan/green/magenta normal regions aligned with its architectural mass, unlike the four maps above. Composited sapling is an upright branching plant with volumetric normals and an explicit root, not an obvious tilted floor object. This report establishes no fundamental new projection defect in either; full per-pose/contact/source audits remain separate.

Do not call all additions a beauty improvement on the basis of a no-clipping pass. Require the before/after native scene to demonstrate an improvement in the focal object and its neighbors, after the objective normal failure is fixed.
