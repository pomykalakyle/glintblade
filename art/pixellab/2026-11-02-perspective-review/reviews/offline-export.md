# Corrective offline export check — PASS

The full exporter reproduced **106 of 106 outputs byte for byte** in an isolated copy. No live source, assets, tools, examples or archived art changed during the run: all 2,158 pre/post file hashes match.

- Regenerated subset: 51 sprite families (102 PNG/JSON files) and four normal maps.
- Live inventory: 96 sprite sheets; all 81 normal/albedo pairs have identical dimensions and alpha coverage, with zero mismatched pixels.
- Independent `audit_round13.py --require-round13`: zero contract failures.
- Deleted families were tested with nine deliberately seeded stale outputs. The full exporter deleted garden, press and summit PNG/JSON/normal files; none exist in live assets or runtime references.
- Distributor albedo is unchanged from release `8894b0b`. Its original RGB is intact at every coordinate; only the documented aperture alpha changes differ from the original source. Semantic face normals reproduce exactly.
- Warden albedo is unchanged from release `8894b0b`. All 75 logical poses reproduce the accepted native registration/patch/damage/fold recipe; the normal correction does not alter albedo.
- Altar is the complete native candidate 21 at 72×32, anchor (36,16), bbox [4,2,68,31]. Every RGBA source pixel is unchanged, and every opaque normal is (128,128,255), world up for a floor receiver.
- All 482 frozen input hashes match both the final source snapshot and current live files. Native `aa1dea6bfb1f3e951a1f448008dec8d3617fcb826d4ebd6c16a574a6b81fd11b` and WASM `c9657cae0053d2f1e5178243b538965fd630526657de14bbf937436cb4f062c8` artifact hashes are confirmed.

Python subprocesses were guarded against network access. The exporter operated only on copied `tools`, `art` and `assets`; this matters because the sapling exporter writes provenance artifacts even when an alternate output directory is supplied.

This is an export/provenance check. It does not establish rendered perspective, physical grounding, shadows or beauty; the controlled runtime reviews cover those separately. The atlas harness reports the retained pre-existing unreferenced opaque cells in `bloomwild_trees` and `hud_bossbar_fill`, plus the expected absence of a campaign report in this offline-only invocation.

Evidence: `audit.json`, `live-atlas-audit.json`, `frozen-source-parity.json`, `live-before.json`, `live-after.json`, `aggregate.log`, and the isolated `workspace/` and `aggregate/` directories.
