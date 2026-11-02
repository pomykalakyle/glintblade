# Native normal maps, round 7

Selected: 78 sprite/terrain families, 85 native generation parts, 1,142,569
opaque pixels, 1,251,221 PNG bytes. Original albedo assets and metadata are
unchanged. Final PNGs live in `assets/normals/<sheet>.png`; atlas dimensions,
frame grids, alpha masks and anchors exactly match `assets/sprites`.

## Data convention

PixelLab OpenGL RGB encodes view-space vectors: red +X right, green +Y up,
blue +Z toward viewer. RGB is data; do not sRGB-decode or interpolate it.
Decode `rgb/127.5 - 1`, then normalize. Horizontal sprite mirrors negate Nx.

Root's chosen world convention is:

- Upright sprites: `(nx, +nz, +ny)`.
- Ground surfaces: `(nx, -ny, +nz)`.

The independent column check in `convention-check.json` confirms that world
overhead light favors the cap (.716 diffuse) over the shaft (.024); light
from the front favors the shaft (.733) over the cap (.522).

## Generation and selection

PixelLab `create_normal_map` lite/pro supplied geometry normals. Native
inputs and unprocessed outputs are in `inputs/` and `raw/`; exact requests,
jobs, hashes and receipts are in `selected-generation-receipts.json`.
`queue.json` records every native crop and its restoration coordinates.
Large hero/terrain atlases split at native cell boundaries to satisfy the
normal tool's 512px side limit. No resizing or interpolation was used.

Bloomwild tree trunk/canopy pairs were composited for a coherent shared
volume, then restored into both original layer cells using their exact
original alpha masks. Both source atlas columns are retained.

Ground maps use weak surface detail with outline rounding and part shapes
disabled, avoiding enormous fake domes over the entire terrain canvas.
Lava's three native chunks have no boundary ridge: mean normal-vector
change at cuts is .0066–.0068; the map's normal Z mean is .99994.
`liquid-seams.json` records the actual selected data.

The three Glasswater crystal families and Warden crystal use planar
authored color patches: connected same-color areas of at least eight pixels
receive the mean generated geometry normal, preserving the silhouette
bevel. This removes smooth plastic lighting over the art's hard facets.
It does not convert luminance into height or change albedo.

## Temporal repair

Raw PixelLab output gave a near-static hound's whole skull different
geometry during an eye pulse. Selected maps transport normals on unchanged
native anatomy: >=90% pose correspondence under a +/-2px translation reuses
matching pixels; larger pose changes reuse only exact 3x3 material patches
with >=55% whole-pose correspondence. Changed parts retain their generated
geometry. Exact duplicate and exact mirrored source cells are canonicalized.

Checks cover 1,209 duplicate cells and 18 exact mirrored cells. Across 67
near-static idle comparisons, the worst p95 diffuse change under fixed
left/right lights falls from .41814 to .00576. The bad hound pair falls to
zero. `idle-coherence.json` also reports larger authored pose changes;
those may legitimately change surface orientation and remain separate.

## Verification and freeze

`asset-audit.json`: all 78 selected families have exact alpha/dimensions,
encoded unit-vector length .98–1.02, front-facing normals and mirror-light
equivariance. `selected.sha256` pins every final PNG; its SHA256 is
`646c096e1fc3341119fa7a5a2da6d9ecf358e4a4a17ecccac5d1c921812f0953`.
`freeze.json` and `selected.json` record aggregate and per-family hashes.

Offline deterministic rebuild and checks:

```sh
uv run --offline --with pillow --with numpy python tools/pixellab/round7_normal_assets.py pack
uv run --offline --with pillow --with numpy python tools/pixellab/round7_normal_assets.py audit
uv run --offline --with pillow --with numpy python tools/pixellab/round7_normal_assets.py coherence
sha256sum -c art/pixellab/2026-11-01-round7/normals/selected.sha256
```

Small reviewed panels and an interactive normal-only asset trial live in
`out/pixellab/round7/normal-trials/`. Geometry stayed fixed while the test
light changed direction; visible animation comes from authored source poses.

Generation allowance used: 27.4 included generations. Remaining: 1333.6.
Paid credits spent/purchased: $0. Receipts preserve rejected size attempts
and rate-limited jobs as well as successful selected jobs. No generations
beyond this queue were requested.
