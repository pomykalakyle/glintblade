# Independent physical normals and lighting re-audit

The four new landmark normal maps do **not** encode the top and side planes visible in their drawings. This is a real material-response defect, despite correct alpha masks and loaded maps. The altar has a separate discrepancy between its depicted vertical riser and its physical zero-height treatment. Sapling normals respond plausibly. Warden macro planes merit refinement, but the evidence below does not establish a new Warden blocker by itself.

## Artifact and method

- Read-only audit. No production edits, cargo builds, staging or commits.
- Exact published native candidate: `883a7ba3c53d586196944dfaf3a4510a391b8fe537206e7d32f11e322dedf068`.
- Frozen release library: `4a770b0915b2c4a4f24806d9493411347a3eadcb1b48561fc291e2b22faff8c6`.
- Temporary `probe.rs` and `runtime_uv.rs` compiled directly with rustc against that library. All tested embedded normal PNGs are byte-identical to current files (`manifest.json`).
- Real runtime `sprite()`, surface stamping, normal decode/axis mapping, and `surface::apply()` were used. Point lights tested north/east/south/west and overhead; white Lambert-style material exposes the vector response without baked albedo shading. The plates also retain the original albedo in separate PPMs.
- Object source RGB, normal PNGs and coordinate grids inspected at integer zoom. Existing aqueduct, cave waterworks and foundry serve as controls, not proof that every old normal is ideal.
- Isolated light tests omit post-grade, reveal and gameplay lighting. They do not establish final in-world visual severity under every source.
- The second light-plate row retains the caster. There is no ground receiver; identical rows are expected and **do not prove ground-shadow correctness**. Ground-contact and real shadow review must come from the separate geometry/shadow audit.

## P2 N1: all four new landmark materials behave like shallow relief on a south-facing board

Runtime coordinate convention is X right, Y south on the floor, Z up. Sprite normals map PNG `(R,G,B)` to world `(R,B,G)` after signed decoding. The transform is internally consistent. The defective result comes from the normal maps themselves.

Every opaque pixel in each new landmark is within 20 degrees of world-south. There are no normals that represent an upward top plane or a strong side plane.

| Asset | Opaque pixels | Mean world Y | Maximum angle from +Y | Maximum world Z | `Nz > .7` | `abs(Nx) > .7` |
|---|---:|---:|---:|---:|---:|---:|
| Garden aqueduct | 7,263 | .9835 | 18.57° | .2471 | 0 | 0 |
| Cave distributor | 8,032 | .9825 | 17.52° | .2545 | 0 | 0 |
| Cinder press | 9,888 | .9799 | 17.58° | .2638 | 0 | 0 |
| Summit frame | 7,659 | .9817 | 19.40° | .2783 | 0 | 0 |

Concrete source coordinates, all opaque:

- Garden dark tower side UV `(50,24)` is world `[-.004,1.000,.004]`, essentially the same direction as a front wall. Its visible lintel top `(98,32)` is `[-.004,.994,.106]`.
- Cave cistern/header top UV `(79,19)` is `[-.028,.988,.154]`; a horizontal receiver should be predominantly +Z. The chute bed `(129,105)` even tilts down: `[.192,.975,-.114]`.
- Press left plinth top `(40,124)` is `[-.193,.979,-.067]`; front `(40,135)` and side `(20,130)` are both `[-.192,.977,-.098]`. Three visibly distinct faces receive effectively one plane direction.
- Summit left lintel top `(55,16)` is `[-.004,.992,-.129]`, facing slightly downward despite being the visibly lit cap.

The five actual runtime light probes reproduce that flattening. Means include an ambient floor of .10 and the same practical-point setup; compare the spatial plates, not just these whole-object means:

| Asset | North | East | South | West | Overhead |
|---|---:|---:|---:|---:|---:|
| Garden | .181 | .210 | .980 | .240 | .264 |
| Distributor | .182 | .225 | .971 | .235 | .274 |
| Press | .185 | .243 | .880 | .245 | .238 |
| Summit frame | .180 | .228 | .981 | .235 | .247 |

The south-light plate lights almost the entire silhouette uniformly. Side lights make only faint relief, while visibly broad top planes never acquire a real +Z face response. Existing Bloomwild aqueduct has 606 strong top and 1,442 strong side pixels; cave waterworks has 187/1,080; existing foundry press has 104/535. The difference is not a decoder issue.

**Recommendation: FIX all four normal fields before accepting them as physical landmarks.** Define the actual broad top/front/side planes first, and use the existing detailed normal only as a small perturbation within each correct macro plane. Check the posed press ram's plane separately. If the source art is rejected for projection/composition reasons by the other critics, REMOVE that instance rather than spending effort repairing its normals.

Relevant evidence: `*-source-normals-4x.png`, `*-coords-6x.png`, `*-runtime-lights-2x.png`, `stats.json`, `face-samples.json`, `light/controlled-light.csv`.

## P2 N2: altar shows a raised south riser but has zero physical altitude

The altar's semantic normals are much better: default top is +Z, front south is +Y, bevel and mortise normals reverse sensibly as the light moves. At UV `(35,50)` the drawn south face is world `[.004,1.000,.004]`. Upward and side-light plates behave coherently.

However, `paint.rs::altar()` paints **every** pixel through `gset_surface(..., false, 0.0)`. The visibly raised south face at source rows approximately 47–53 is not a raised surface or caster. The lip and mortise also have no physical height separation. This is an intentional implementation contract, but it does not validate the new art's depicted thickness: it is physically a flat decal.

**Recommendation: FIX the art/physical representation mismatch.** Either draw a genuinely flat inset/inlay matching the original walkable floor, or introduce a carefully scoped shallow relief/edge model with the correct receiver heights and contact occlusion. Do not call a six-pixel-looking riser physically validated merely because all of it remains on dry walkable SLAB. This audit does not require adding player collision.

## Warden: working orientation/lookup, broad fronts still inherit rounded normals

The body is not affected by the all-front landmark failure. Its first awake cell contains 639 strong top and 1,249 strong side pixels, and real E/W/up lights clearly separate crowns and sides. All 102 physical atlas cells were checked, including blank cells and articulated frames: 591,878 opaque normal pixels match runtime UVs exactly, with no alpha or emit mismatch.

Some apparently vertical broad front patches tilt downward substantially:

- Left pier front `(35,96)`: `[-.216,.874,-.435]`.
- Right pier front `(76,96)`: `[-.176,.887,-.426]`.
- Left foot front `(34,105)`: `[-.130,.640,-.758]`.
- Right foot front `(79,105)`: `[-.067,.678,-.732]`.

This makes lower stone read more rounded/underside-facing under source movement. Some foot normals may intentionally describe a chamfer; this read-only audit cannot infer a complete 3D model from those six rows. Existing approved sprites also have rounded inferred normals, so these samples alone do not prove a new production blocker.

**Recommendation: KEEP the animation and registration; FIX/POLISH semantic front and foot planes if the body is retained.** Judge those exact patches under low fire versus overhead key before accepting generated normals as plane truth. No confirmed emitter bug in this body atlas (it correctly contains zero emissive pixels; head/fist/crystal assets are separate and were outside this atlas audit).

## Sapling: KEEP normals

The split trunk/crown recombine into a narrow volumetric trunk and three rounded flower clusters. E/W lights swap branch/crown edges, overhead lights the flower tops, and south lights fronts. That is plausible for the depicted round surfaces. Existing root rows are kept by the exporter. There are no loaded-vector, alpha, or emissive flag mismatches across its two cells. This is a normals signoff only; perspective/placement is covered by the other critic.

## What passes mechanically

For all seven new families, the runtime uses the embedded intended map, source coordinate, cell and signed convention. Across all tested cells: zero UV-vector mismatches; zero alpha mismatches; zero normal/emissive flag mismatches; all seven asset families have zero unexpected emitted pixels. Decoded lengths range roughly .993–1.007 after i8 quantization, which is normal quantization error, not damaging clamping or missing normalization.

These mechanical passes do not override N1/N2: a perfectly loaded wrong plane is still a wrong plane.
