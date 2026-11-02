# Bloomwild daylight audit — round 9

The candidate passes the sampled whole-scene daylight review. Shadowed columns,
walls, arch faces, bark, foliage, hero and hanging cliffs receive enough fill to
fit the lit ground. Stone courses and cliff facets remain readable; cracks,
contact shadows and undersides retain depth. No further lighting edit was
justified by these images.

## Direct review scope

All **26 candidate PNGs** were opened and reviewed directly under
`out/pixellab/round9/daylight-candidate/views/`:

| Locations | Sun ticks | Images |
| --- | --- | ---: |
| temple, courtyard, grove, stair-contact, west-terrace | 90, 630, 1170, 1710 | 20 |
| landing, cliff-contact, falls | 90, 1170 | 6 |

Matched round 8 references are
`out/pixellab/world/round8-selected/bloomwild/<location>/bloomwild_t00090.png`.
The round 8 temple/courtyard Sun sequences were also sampled at frames 901,
1020, 1140 and 1380 during diagnosis.

The most useful comparisons are the white stairs beside the retaining wall,
the courtyard wall front and its grass shadow, the temple columns, the grove
trees and hero, and the arch above the hanging cliffs. At every sampled Sun
position, the candidate keeps these materials in the same daylight scene.
Matched stills retain prop silhouettes, cliff outlines and hero placement.
Moving cast shadows remain visible across the sampled Sun positions; this is
not a frame-by-frame motion or geometry audit.

## Measured before/after samples

Values below are mean 8-bit sRGB luma, `0.2126 R + 0.7152 G + 0.0722 B`, from
matched tick-90 crops. Coordinates are `(left, top, right, bottom)` with exclusive
right/bottom bounds, in the native 384 × 216 images. They describe image values,
not physical light intensity.

| Location / material | Crop | R8 | R9 |
| --- | --- | ---: | ---: |
| temple / pillar front | 98,124,108,140 | 46.8 | 71.2 |
| courtyard / wall front | 239,53,258,67 | 67.3 | 101.7 |
| courtyard / wall cast shade | 245,73,269,81 | 43.8 | 76.0 |
| west-terrace / retaining face | 33,97,70,111 | 36.0 | 57.1 |
| stair-contact / retaining face | 253,99,281,115 | 37.8 | 59.8 |
| grove / bark | 157,117,163,151 | 67.0 | 89.9 |
| grove / canopy | 232,85,273,99 | 67.9 | 82.1 |
| temple / hero | 186,96,198,113 | 72.6 | 91.1 |
| landing / arch front | 244,60,265,77 | 64.3 | 95.5 |
| cliff-contact / arch upright | 60,74,72,91 | 56.3 | 87.9 |
| landing / hanging cliff | 161,163,204,185 | 25.3 | 40.4 |
| cliff-contact / cliff front | 121,137,160,161 | 37.2 | 54.3 |
| falls / cliff front | 101,156,132,175 | 31.5 | 49.3 |
| temple / lit paving | 200,80,225,100 | 180.6 | 179.8 |
| courtyard / lit grass | 278,100,298,117 | 140.8 | 140.3 |
| stair-contact / bright stairs | 170,90,184,106 | 226.9 | 226.1 |

The fill lifts upright and shaded materials while preserving lit paving,
grass and bright stairs. Hanging rock remains darker than the ground, but its
faces no longer merge into a largely black mass.

## Build provenance and separate performance limitation

These are candidate captures made before the final WASD app/input change. The
coordinating build audit confirms the final build has identical lighting/color
code; these are not newly captured final-input-build PNGs.

The coordinating agent reports successful final native/WASM builds and **255
functional passes, three unchanged performance-budget failures and seven
ignored tests**. The suite does not pass as a whole. This visual audit neither
resolves those performance failures nor measures browser performance. Its
verdict covers the 26 stills and stated comparisons, not every gameplay frame
or material in the game.
