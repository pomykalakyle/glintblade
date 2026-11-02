# November 2 beauty pass

The initial visual acceptance below was superseded by Kyle's physical-sanity review.
Three landmark integrations were rejected, semantic normals repaired, and the raised-looking
altar replaced with a genuinely foreshortened flush inlay. The original sources remain for
provenance. See [the correction record](../2026-11-02-perspective-review/README.md) for current
selection and validation. PNG alpha, vector length and collision parity did not establish
correct face directions or convincing floor perspective.

Four waterworks landmarks, a sculptural Warden with 75 logical poses, a native stone altar,
and eleven connected Bloomwild planting beds. PixelLab originals, requests, selections and
normal maps are retained in `landmarks/`, `warden/` and `altar/`. `vegetation/` reuses the
approved native leaf atlas without changing its source pixels, normals or relief.

The combined change includes the coordinated ten-finding audit: wider foreground context,
Warden stone visibility, awake enemy contrast, landscape touch controls, title instructions,
an independent young sapling, aqueduct step paint and Cinder mineral facets. The sapling's
source archive is in `../2026-11-02-audit-fixes/`.

Final narrow meadow correction removes nine of 377 actual runtime roots. The nearby dense
bed retains 37 of 39 roots. Entire moving leaf fans stay outside the existing encounter
clearings. On the exact 720-tick replay, the lower attacker is visible throughout all 32
windup frames, first damage remains tick 65, and sleeping camouflage is pixel-identical.
Seven later windup windows also retain readable cues. Gameplay traces and audio events match.

Validation:

- 374 functional Rust tests pass; seven ignored. All three unchanged 6/8 ms performance
  gates were also executed and still fail. No budget was relaxed. Separate native sampling
  covers 1,080 frames across six art views and three actual boss phases; this is not browser FPS.
- Native and WASM release builds pass with all 84 authored normal maps. Every one of the
  387 embedded asset byte sequences is present in the final WASM module.
- Offline rebuilding produces 115 byte-identical files. All 99 sheets, 1,720 logical frame
  references and 84 normal masks pass contract checks. Nineteen asset-audit regression tests
  and two source-freeze tests pass.
- The exact final native candidate completes four regions, three shards, all Warden phases,
  victory and the return prompt in 14,914 ticks through actual inputs. Invulnerability is
  used for coverage; this does not establish ordinary-player difficulty.
- Fresh still and consecutive-motion reviews cover all four regions, actual gates, Warden
  cues, physical shadows, native emission, grass cuts, feet and source-pixel visibility.
  Gate-only validation covers 108 consecutive frames; context checks cover 204 depth frames
  and 54 live boss frames. No remaining confirmed P1/P2 was found in these reviewed scopes.
- Earlier combined browser suites passed 384 checks and a separate 101 checks. The exact
  final module passes a fresh keyboard/touch/pause/resize smoke test. These are headless
  Chromium checks, not a cross-device benchmark.

`release-validation.json` records exact artifacts and local evidence paths. Full frozen
inputs, raw captures, reports and the interactive before/after gallery are retained under
`out/beauty-pass-2026-11-02/` in the working project; those large QA artifacts are not committed.
`docs/media/` videos predate this pass. Earlier intermediate captures are versioned evidence,
not the publication build.

Rebuild assets offline:

```sh
uv run --with pillow python tools/pixellab/build_assets.py
uv run --with pillow python tools/pixellab/audit_round13.py
```

The historical `landmarks/offline-verification.json` receipt predates final selection and
lists a rejected distant vista. The current selection explicitly rejects both vistas; no
vista asset is exported or loaded. The Summit normal receipt lacks original request arguments;
its selected-file binding, native dimensions and alpha parity are verified. A distant initial
Warden roar can crop at the top under the existing hero-following camera policy.
