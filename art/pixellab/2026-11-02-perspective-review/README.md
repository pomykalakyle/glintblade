# Physical sanity and art-fit correction

Kyle rejected the visual fit of the first round13 additions. Fresh independent reviews
confirmed that the prior acceptance was too focused on alpha, collision, embedding and
glitches. Those checks did not prove that depicted geometry stood on the floor or lit correctly.

| Object | Finding | Current decision |
| --- | --- | --- |
| Garden landmark | Flat material; tower extends beyond retained plinth | Restore the original native arch |
| Large foundry press | About twice the old size; weak floor-depth read, dominates canal | Restore the small original press |
| Summit frame | Competes with dais; shallow front-elevation feet | Remove it |
| Cave distributor | Coherent source geometry, but every generated normal nearly faced south | Keep shape; author real roof, wall and side normals |
| Warden | New sculptural poses fit; some broad pier fronts inherited downward bump normals | Keep art/poses; author physical face normals |
| Altar | Deep badge/riser depiction on a zero-height floor | Use native72×32 flush inlay,64×29 opaque footprint, floor normals+Z |
| Sapling and planting | Sapling rooted; denser planting is a style tradeoff | Keep; no claim that density alone establishes beauty |

All four rejected original landmark normal maps had zero opaque pixels with world-up
Z>.5 despite visible roof/cap planes. The garden received zero positive directional-sun
diffuse at tick0, including those caps. Existing neighboring column caps pointed strongly
upward. The defect was semantic geometry, not a decode, alpha or file-loading failure.

The retained distributor uses36 named native face layers with independent known-face
samples. Warden normals use authored structural masks for all poses/folds. Their albedo
stays byte-identical. The inlay is a native shallow PixelLab output, not a compressed image;
all16 deep replacement candidates were rejected before choosing shallow candidate21.
Two corrective generation calls used20 subscription generations and made no credit purchase.

The paving thread separately repaired disconnected32px stone patches, bridge apron receiver
normals, a missing dry-ground path connection and bridge bond seams. Both teams used the
same frozen combined renderer. Paving sources and matched evidence: [paving](../2026-11-02-paving/).

Validation on the corrected combined source:

- 380 functional Rust tests pass; three unchanged 6/8ms performance gates fail in the earlier full run; seven ignored.
- 28 Python export/audit regressions pass. All 106 regenerated exports are byte-identical;
  96 sheets and 81 normal-map pairs pass contracts. Rejected exports cannot reappear.
- Actual runtime cardinal/overhead lighting, surface buffers, ground contacts and cast
  shadows inspected: 70 nonempty Warden cells under five light directions, plus live poses.
  Source/UV/alpha/emission checks match the embedded maps. No global exposure change.
- Actual input tour:14,914 ticks, four regions, three shards, all Warden phases, victory.
  Invulnerability establishes coverage, not ordinary-player difficulty.
- Final altar approaches from all four sides and real Prism pickups inspected. Actual
  floor normal/albedo buffers and 25 scene captures over a full36-second sunlight cycle pass.
- A fresh input replay caught a detached crown on the restored small press. A scoped whole-shape
  fade fixes it while retaining the full physical caster. All 162 final motion frames and 42
  critical enlarged frames pass; distant and nohero controls remain intact.
- Exact final WASM passes 59 headless keyboard/touch/pause/resize checks without page errors.

The cave's ambient-scene change is subtle; it is a material correctness fix, not a claim
of a dramatic artistic gain. A frontal wall can be valid in this game's projection.
These reviews apply to the supplied views, motion and light states, not every possible frame.
Historic `docs/media/` videos and the first round13 release record predate this correction.

`audit.json` records exact hashes and evidence paths. Full captures and the local interactive
comparison are under `out/perspective-reaudit-2026-11-02/`. The images in `scenes/` are matched
native before/corrected game frames; all use the same scene camera and warm tick90.
