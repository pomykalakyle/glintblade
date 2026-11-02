# Glasswater distributor: projection and ground-contact ink

The final pass preserves the corrected square-on cistern, outlets and inspection
arch, restores candidate2's native lower stone volume, and replaces only its
authored ground-contact ink with adjacent masonry material. Top and side outlines
stay. Artificial contact AO, apron bedding and radiating cracks are removed;
the real alpha/normal geometry still receives light and casts shadows.

The original round13 structure at world1304,252 had diagonal masonry returns and
feet that fought the cave floor projection. Its earlier authored-normal correction
could not change that projection. Four immutable native144x112 PixelLab candidates,
request and receipt remain in this folder. Candidate2 supplies the calm broad
masonry; source-coordinate repairs open the header basin/outlets and retain the
failed coping, dry sediment and overflow stain. No source pixel is scaled,
rotated, blurred or interpolated.

Kyle rejected the subsequent hand-built U caps, rectangular root courses and
contact-only visual acceptances. The staged rubble/shoulder experiment is also
REJECTED under his explicit direction to correct bottom ink rather than devise
new bases. Its generated input/output and receipts are retained as evidence,
but its pixels and terrain hooks are absent from the final runtime/export.
The final lower faces use the original candidate2 pixels, with only the documented
detached-chip removal and the four explicit contact-ink bands changed.

`recipe.json` identifies the native front sole bands at x31..56/y96..99 and
x103..128/y96..99, plus recessed inner pier soles x58..66/y93 and x91..101/y93.
Only their dark ink colors change to existing masonry0x424956. This is not a
lowest-opaque-column or global-black-pixel eraser. Cavity/arch edges, raised cap
edges and every lateral/top outline outside those authored bands remain intact.
Alpha and geometric normals are byte-identical before/after the ink correction.

The retained native cell/anchor is144x112 /79,103, and world anchor1304,252.
Restoring the original feet replaces the rejected constructed rows95..102:
actual support is now rows92..99, with the true foot/sort plane at worldy249.
The exact native opaque row spans drive collision and leave the arch traversable.
The upper78 native RGBA rows remain identical to the earlier corrected structure.
Authored planes follow the restored shallow native cap/front/inner return faces;
the color-only ink correction changes none of those vectors.

`contact: "none"` opts this one asset out of the generic baked contact ellipse.
The discarded native-contact helper is removed. Every other prop retains its
existing contact behavior. Physical alpha casters, normal shading and motivated
cast shadows are preserved. `place/distributor.rs` keeps only a shallow dry drain
through the arch, turning southwest; it adds no bed, shoulder, border or underlining
crack. All ground edits clip to dry zero-height Floor within1230..1375,225..267.
No final registration hook was added to glasswater.rs.

Rebuild only this trio:

```sh
uv run --offline --with pillow python tools/pixellab/distributor_rework.py
```

The existing scoped round13 delegation preserves this result during a future
rebuild. No umbrella export, unrelated atlas repack, shared browser/README output
or publishing operation was run.

Final actual native gameplay views, original/rejected/contact comparisons,
controlled directional lights, physics/reveal regressions, source/asset snapshot
and native/WASM build receipts:
`/home/krp/.t3/scratch/2026-11-02-fix-glasswater-stone-structure-perspective-0b961023/audit/contact-ink/verification.json`.
Prior contact-rework and rejected-shoulders artifacts are historical evidence,
not the final visual acceptance. Tests establish technical contracts; Kyle's
visual judgment remains separate.
