# Broken column foot repair

Kyle's close-up identified an actual source-art defect: both broken Bloomwild
columns continued as straight 29-pixel-wide shafts through the last row of their
48×56 cells. Ground contact shading could not finish the missing stone geometry.

Two PixelLab masked-inpainting studies cost 20 subscription generations. The first
added an architectural bevel but cropped the tall plinth again. A second study
used 8 extra transparent staging rows and produced complete rounded, chipped
stone feet. No additional generation was used.

The selected lower 14 native rows from the second study replace the old lower
shaft. The original upper 40 rows remain byte-identical. Tall and short columns
use separate generated foot variants, retain the original 48×56 cell/24,53 anchor,
and leave 2 transparent rows below a visibly closed lower face. There is no
resizing, palette filtering, drawn replacement geometry, or added cast shadow.

`selection.json` gives the exact raw source rectangles and original sprites.
`pixel-origins.json` records every changed pixel and its literal native origin;
the two bottom rows are explicitly cleared staging margin. Raw inputs, masks,
outputs, prompts, receipts and rejected studies are preserved here.

Rebuild with `uv run --with pillow python tools/pixellab/build_bloomwild_structures.py`.
The packer verifies the source composition, frozen upper rows and lower gutter,
then regenerates exact opaque structural-base collision spans. Both new feet are
37×10, and sort at anchor+1 rather than the old cropped anchor+3. All 7 other atlas
cells and their collision metadata remain exact. An offline candidate rebuild
matches the exported PNG/JSON byte-for-byte.

An independent reviewer inspected the native sources, integer enlargements and
actual title/temple captures using the old immutable game binary with these
replacement assets. The cutoff is fixed. The wider foot also exposed an existing
temple drum overlap; combined placement and final executable checks are recorded
by the root review separately.

The account changed from 449 used/1551 remaining to 469 used/1531 remaining.
These are subscription generations; no purchase or credit charge was made.
