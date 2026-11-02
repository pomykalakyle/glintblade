# Native young tree for audit finding 8

The legacy young tree kept flat rose strips beside the dimensional native tree
family. This replacement is a genuinely young native design, generated at
48x64 with the existing mature PixelLab source as a material/style reference.
It occupies its own atlas and never changes the mature tree family.

`request.json` and `submission.json` record the generation contract. Untouched
candidate PNGs are archived as `raw-NN.png`; `selection.json` records the final
candidate, generation rationale and native normal source. The selected source
above y52 is copied at identical UV coordinates, with no resizing, rotation,
palette remapping or filtering. The existing young tree's last eight opaque
root rows are retained at their exact world positions. Its collision spans,
14x8 footprint and root normal map stay identical too.

The 48x128 atlas contains one 48x64 trunk cell and one canopy cell, each with
anchor (24,58). Animations are `e_trunk` and `e_canopy`; both are single fixed
frames. Existing canopy/reveal/shadow drawing remains applicable. Only the
young-tree selector at map mark (76,17), world (1224,280), uses this sheet.

Rebuild offline:

```sh
uv run --with pillow python tools/pixellab/audit_sapling.py
```

`asset-audit.json` records native pixel/root identity and exact opaque collision
checks. Root handles the combined Rust build and actual game beauty/collision
captures after parallel source owners finish.

Suggested game probes (replace RENDERER with the combined candidate path):

```sh
RENDERER zoneview bloomwild --at 1224,272 --warm 90 --scale 1 --no-hero --out sapling-overview
RENDERER zoneview bloomwild --at 1224,272 --warm 90 --scale 3 --no-hero --out sapling-overview-3x
RENDERER shot --zone bloomwild --pos 1224,260 --god --ticks 15 --scale 1 --out sapling-behind
RENDERER shot --zone bloomwild --pos 1212,280 --god --script R30 --ticks 30 --scale 1 --audit --out sapling-contact-west --report sapling-contact-west.json
RENDERER shot --zone bloomwild --pos 1242,280 --god --script L30 --ticks 30 --scale 1 --audit --out sapling-contact-east --report sapling-contact-east.json
```

Legal contact positions can stop short of the exact visible root; the old
footprint and every collision rectangle are preserved, rather than widened
to encompass the new crown. Mature-tree source, atlas and normal hashes must
remain unchanged by this task.
