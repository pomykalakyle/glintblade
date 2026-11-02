# Independent post-fix normal, contact and shadow audit

**Bounded PASS. No concrete P1/P2 remains in the retained Cave Distributor or Warden material/contact/shadow scope reviewed here.** The formerly flat landmark normal field and downward-facing Warden pier fronts are corrected. This does not certify the excluded altar or the aesthetic value of every added object.

Final combined native: `aa1dea6bfb1f3e951a1f448008dec8d3617fcb826d4ebd6c16a574a6b81fd11b`.
Final library: `4664d37ac739a45a88762854bdbe709662bc1830fb7b7e5967b65ba78a177bc9`.
The complete light/pose review used interim native `8e1c39899936c87835c82c067b9cb569ab3200ad694d24321959fa88ac59cc19`, library `9913cd6b865c7ded540f57ae40df1c7fb19a04d82f48ea67198035a7932cfbab`. All reviewed runtime Rust and both subjects' sprite/metadata/normal files are byte-identical in the final freeze. One actual distributor scene and ten live Warden frames were recaptured using the final binary and are PNG byte-identical to the reviewed interim build (`final-parity.json`).

## Independent method

No production edits, Cargo builds, staging, commits, pushes or subagents. Temporary Rust probes link directly to the frozen libraries with rustc. The old published `883a7ba3c53d586196944dfaf3a4510a391b8fe537206e7d32f11e322dedf068` library is the controlled comparison.

The review inspected native albedo independently of the author's face labels; compared actual runtime north/east/south/west/overhead point lights; inspected actual `Frame.surfaces` normals and heights; compared retained/removed physical casters on a ground receiver; and sampled actual moving sun at ticks 0, 540, 1080, 1620 and 2160. White-material plates separate normal response from baked color. Original-material plates and actual game scenes remain separate evidence.

All 70 nonempty Warden atlas cells were rendered under all five point directions (350 pose/light renders). All 102 physical cells, including blank cells, were checked for normal UV, alpha and emission correspondence. Native pose mosaics were inspected at 1×; named source faces and selected actual game crops were inspected at integer 3×. Actual game review includes 30 assembled Warden samples through tick 1800, plus cave hero approach, front/context, distant and four timed context samples. This is sampled gameplay, not every intervening tick.

## Distributor

The large header front, left end, narrow feeder supports, exposed coping/foot tops, buttress steps and open channel now behave as distinct faces. World-west light illuminates the visible west returns while east light leaves them dark; overhead light illuminates the cistern/coping and stepped caps rather than washing the entire wall. Front light favors the broad masonry front. The arch's inner returns face opposite directions, and its transparent passage remains a true light opening. I found no mirrored side response or obvious top/front polygon inversion in the inspected native source and moving-light plates.

The original first cell had zero strong upward pixels. The corrected native cell has 1,635 pixels with world Z > .7, 2,017 strongly west-facing pixels, 78 strongly east-facing pixels and 4,302 strongly south-facing pixels. It has no materially downward-facing pixels. Counts demonstrate distinct planes; they do not independently establish artistic quality. `round13_landmarks_distributor-directional-1x.png` is the direct same-light old/new comparison.

All 456 opaque pixels in the authored native bottom support band lie over actual dry Floor receivers of height 0/kind 1. See `distributor-native-foot-receivers.csv`. The visible block feet meet that dry floor in actual approach/context captures; no new gap or water support was observed. The back-pointing feeder is an overhang, not a separate ground foot.

The actual cave before/after is **subtle under dim ambient lighting** (`distributor-full-approach-compare3x.png`). Correcting its normals does not establish a major aesthetic improvement. In particular, a normal fix should not be described as adding a visibly different landmark composition.

## Warden

The left tower's east return and matching projecting buttress returns light from the east; narrow west bevels reverse that response. Tower/pier roofs and lintel/sill tops respond predominantly from above. The broad lower front faces now respond to front light and do not point down into the floor. Folded piers carry the same face directions with their source pieces; they do not switch to a whole-silhouette radial normal. All 70 nonempty pose fields have zero opaque pixels with world Z < -.1.

`all-poses-{E,S,W,Up}.png` covers the intact/damaged closed/open fold rows and the two brace keys. Raw-cell fold plates intentionally hold the draw anchor fixed to compare material direction. They are not a simulation of the whole boss's grounded fold path.

The assembled live frames show the corrected stone surfaces together with the existing head, crystals, fists, cracks, heart, beams and reveal. Actual native/3× samples show attached foot contact, coherent front/side response as the blade/heart sources move, and no bright detached foot or new lighting seam. Open-heart/pylon/brace states through phase 2 were observed. These samples do not replace the other reviewer's full phase-3/death coverage.

## Shadow and packaging controls

Retained-vs-removed-caster comparisons show attached casts, point-source reversal and arch transmission. The full shadow/caster coverage descriptions are exactly unchanged from the old binary for all selected controlled subjects and lights: 30 contact-test lines and 27 sun-test lines (`caster-and-controls-check.json`). This rules out a changed caster as the cause of the new material response. It is a preservation check, not proof that a 2D silhouette reconstruction is a unique solid model.

Three unchanged control families (existing aqueduct, cave waterworks and foundry) each produce 20 byte-identical old/new directional files. Thus the subject changes are local normal corrections, not a global exposure change.

Both interim and final frozen libraries expose all 81 intended normal PNGs byte-for-byte. For Distributor (8,032 opaque pixels), Warden (591,878 atlas opaque pixels) and sapling (580), runtime UV vectors, alpha and emission flags have zero mismatches; all expected normal pixels are finite/unit within i8 quantization. The scene normal/height buffer evidence is `actual-runtime-surface-buffers.png`.

## Scope limits

The old unapproved altar was excluded from this audit. The final floor inlay requires its own visual/normal signoff. Garden and press are restored assets; the Summit frame is removed. This report does not reapprove their rejected new versions. Cardinal face normals remain a coherent pixel-art approximation rather than an exact metric reconstruction of the drawn solid. No claim is made about invisible rear geometry.

Machine-readable verdict and file references: `audit.json`.
