# Round6 botanical and mineral enemy variants

The previous native morphs mostly changed color and added small growths. This
pass ships two complete alternative appearances for each of three archetypes:

| Archetype | Variant 1 | Variant 2 | Poses per look |
|---|---|---|---:|
| Sproutling | Broad oatcap fungus, dark gills, rose lichen | Open camellia petals, dark throat, sage sepals | 72 |
| ThornImp | Forked birch mask, lichen mantle | Broad bracken mask, pale raffia shoulders | 57 |
| CrystalBeetle | Hollow quartz geode | Ribbed scallop fossil valves | 72 |

All 402 new poses come from PixelLab native full-sheet edits. Native crops,
palette correction, exact support-row restoration, and the imp's held-object
mask are reproducible in `tools/pixellab/round6_botanical_variants.py`.
No native sprite is scaled. Integer enlargements are review artifacts only.

Originals and packed inputs are archived here; `jobs.json`,
`sproutling-v1-retry.json`, and `beetle-padded-retries.json` preserve prompts,
seeds and queued responses. The receipt files record all nine completed jobs:
100 subscription generations, $0 purchased credits. `selection.json` pins the
selected/rejected jobs, all output hashes and all-pose change statistics.

The first mushroom was rejected for mostly recoloring the old seed. Both first
beetle sheets were rejected after direct review exposed dome tops bleeding
above their 26px cells. The selected beetles use 32px padded generation cells
and native crop offsets of 2px (geode) / 0px (scallop), then restore the exact
original support rig. All introduced outer-border touches are eliminated.

`qa.json` checks every new pose: nonempty, changed, exact original support rows,
identical metadata, no introduced border cutoffs, no lost unique animation
poses, and exact imp seed masks/centroids. Beetle appearance may change its
crystal anatomy; every attack and locked charge pose still reaches the hot
iris emission tier, while dazed crystal pixels use only dim violet.

`rebuild-verification.json` records a byte-exact rebuild of all 12 sibling
PNG/JSON files in a separate destination. No enemy Rust/behavior code was
modified by this art pass.

Native and 4x all-pose directional sheets, per-animation 4x GIF loops, and
1x/6x comparison samples live in `out/pixellab/round6/botanical/`.
All nine directional sheets were directly inspected. Integrated gameplay and
lighting review belongs to the root's final candidate review; these source
checks do not claim a full in-game audit.
