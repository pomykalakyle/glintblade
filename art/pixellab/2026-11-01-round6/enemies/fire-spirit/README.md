# Round 6 fire and spirit variants

Six native PixelLab alternate skins: ivory rib-armored and jasper glassback hounds;
moon moth and hanging lantern wisps; twin-flue copper furnace and ash ceramic kiln.
The original archetypes and their behavior remain unchanged.

- Shipped: `assets/sprites/{ember_hound,wisp,spitting_totem}_{v1,v2}.{png,json}`.
- Coverage: 56 frames per hound, 18 per wisp, 30 per totem; 208 alternate frames.
- Inputs: immutable `source-*.png/.json`; raw PixelLab `*_v*-raw.png`.
- Requests, seeds and job IDs: `requests-all.json`. Per-job receipts: `*-receipt.json`.
- Actual charge: 60 included generations; no credit purchase. One Pro Flash totem
  attempt was rejected before submission because its 288px height exceeds 256px.
- Selected output hashes and exact offline rebuild proof: `selection.json`.
- Frame-level masks, registration, changes and hashes: `coverage.json`.

Rebuild: `uv run --offline --with pillow python tools/pixellab/round6_fire_variants.py`.
Use `--out DIR` for a reversible export. All 12 PNG/JSON files reproduced byte-exact.
No image resampling occurs. Integer zooms in review sheets are display-only.

Native operations: hound registration preserves the authored foot baseline;
the glassback carries one unchanged native jasper plate across every side pose,
fixing the generator's shell shrinking between idle and running. Totem chassis
alpha is constrained to its first idle silhouette to remove invented windup sparks.
The original native firing outlet/backplate and every exact emissive pixel return
in every hound/totem frame. The totem's emissive centroid therefore remains exact.
Wisps preserve the native bright heart and dark face slit while the outer spirit
body changes. One wing-edge column is cleared to leave a native atlas gutter.

Independent review found two motion defects after the first asset freeze: the
ivory hound lost its shoulder armor in running/airborne side poses, and the moth
switched instantly from spread wings to a folded charge with tips touching the
top cell border. The packer now carries one unchanged native ivory shoulder into
those ten dynamic poses. Two native wing positions close the moth's wings over
the first two charge frames; its folded upper tips move inward by one native row.
The 208-frame coverage report, contacts, inspector and all hashes reflect these
repairs. A second export reproduced all 12 shipped PNG/JSON files byte-exact.
An independent scan of all 610 enemy alternate frames also caught three airborne
hound poses touching the top cell edge. Their native upper caps now retain a
one-pixel gutter, without moving feet or firing cues. Every carried shoulder
pixel remains exact after its integer registration. No alternate frame adds an
opaque border pixel relative to its original pose.

Review: `first-looks-1x.png`, `first-looks-3x.png`, and `*-all-poses-{1x,3x}.png`.
`animation-inspector.html` uses every original row and its authored duration at
60 ticks/second, three skins in sync, direction selector, mirror and single-tick step.
Its source is `tools/pixellab/round6_fire_review.py`.

Direct review covered all authored poses and the targeted rigid-shell correction.
Integrated gameplay, lighting, placement, attack FX, runtime skin continuity and
independent visual review remain the parent agent's responsibility before release.
