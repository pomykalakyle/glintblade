# Cinder native environment pass

Created 2026-11-01 · Updated 2026-11-01

The brief covers the whole volcanic region: the ash approach, lava crossing, old foundry,
upper river bank and caldera shrine. Generated sprites are judged in actual native game
frames, with lava motion, contact lighting, cast shadows, actor sorting and route visibility.

The baseline has a coherent cold/hot hierarchy but repeats broad simple cuboids on every
mesa and cliff, outlined rectangular paving, and generic small monuments. This pass keeps
the lava as the only bright landscape material and uses the actual Cinder `art.rs` palette.
`palette.json` lists the source colors; the shrine alone adds two existing copper shades.

Rebuild the selected sheets (no network, credentials or generation):

```
uv run --with pillow python tools/pixellab/build_cinder.py
```

`selection.json` is the packing source of truth. Every visible pixel is retained on its
original native grid. Marker packing translates opaque pixels to a common foot anchor;
there is no scaling, mirroring, interpolation or palette replacement.

First in-game review found that cliff candidates 1 and 4 formed busy narrow ribs and
stair-like courses. Those were rejected after the actual terrain capture. Candidates 2
and 3 have broader irregular planes, and their native top pixels enter only blocked
mesa caps. The source silhouette replaces the old island footer only into empty backdrop;
it cannot enter lava, falls, banks, bridge decks, shrine stairs or walkable lanes.

The original low broken stones remain as subordinate details. Tall foundry markers replace
six repeated pillars at the original feet and collision footprints. The two old statues
become distinct cold stone press monuments, retaining their original 36×9 collision feet.

Generation receipts and unmodified outputs are under `jobs/`. `ledger.json` records booked
and charged subscription generations. No purchases or plan changes were made.

Baseline native captures: `out/pixellab/cinder-baseline-native/`.
First integration review: `out/pixellab/cinder-pass1/`.
These are composition captures, not claims of gameplay coverage; the final shared replay
uses ordinary game inputs and captures every 60 Hz simulation frame.

The second native review found a crack lattice when generated paving covered every slab.
Final damaged-slab coverage is 34%; most of the court retains its quiet broad underpainting.
The two selected ledges use varied native 80px subregions to suppress the old full-width
repeating stamp. Three low mineral shelves sit sparsely inside blocked mesa caps, never
on the playable ash. The caldera gains a quiet copper-ring inlay inside its existing dais;
the outer radial masonry, lava contact, stair treads and collision remain authored.

Final source/art freeze: six sheets, 65 charged subscription generations, no pending jobs.
The packer reproduces all six PNGs and JSON manifests byte for byte.

Final native six-view and whole-map review: `out/pixellab/cinder-final/review.json`.
One concrete defect was handed to the lead before final release: the generated shrine
ring's opaque near-black center reads as a pit on a walkable shard floor. Preserve the
original floor within that ring and align its visual center at source pixel (63,43).
The lead will verify that correction with the shared final build and gameplay review.
