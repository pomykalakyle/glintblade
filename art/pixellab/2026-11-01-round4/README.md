# Contact and transition review

Kyle requested another integrated pass on objects, cliff-to-ground transitions, water,
lava, bridges and the title. This pass covers all four regions. Earlier source art and
receipts remain in the first two PixelLab archives; this directory records the follow-up.

For example, the same rightward movement beside Bloomwild's tree B previously reached
x238.09 through its visible roots. The native support now stops Glint at x220.09. The
entry arch's stone pier now blocks him, while separate northward checks pass through its
opening and the aqueduct opening.

| Finding | Result |
| --- | --- |
| Old prop boxes missed roots and stone bases | Source-opaque support spans, correct flipped/fractional transforms and sorting |
| Painted shore/corner differed from collision | Finished native surface owns movement collision in all four regions |
| Bloomwild cliff art covered grass or formed a continuous gray strip | Clipped native rock with broken turf, soil and root contacts |
| Prop shadows painted into sky cuts | Preserve all 4,049 checked eroded sky pixels |
| Cinder arch had a molten gap beneath its landing | Real land-seated abutments and protected deck pixels |
| Glasswater looked hatched and produced oversized wakes/reflections | Distinct shallows, current marks, local wakes and a short actor reflection |
| Lava drafts repeated blobs, stripes or identical branch shapes | Independently selected PixelLab basalt crust and hot cracks with continuous native-cell flow |
| Title was an awkward crop of the level | Complete bounded outcrop with one tree, one ruin and Glint |

Implementers reviewed their own work; separate physics and visual reviewers then found
and checked integration problems. The visual review rejected several lava drafts rather
than treating generated or procedural assets as automatically acceptable.

See [COLLISIONS.md](COLLISIONS.md) for source support contracts and reproducible checks.
[verification.json](verification.json) pins the selected executable, captures, test results and
recording; [physics-review.json](physics-review.json) and [visual-review.json](visual-review.json)
retain the separate reviewers' exact scope and limits. The new raster came from PixelLab MCP;
its [full prompt](cinder/lava-study/request.json), [receipt](cinder/lava-study/submission.json)
and [native crop selection](cinder/lava-study/selection.json) are preserved alongside all four
untouched originals. One request used ten subscription generations.

Headless route recordings use invulnerability for coverage; they do not
establish ordinary-player difficulty balance. Focused water clips use normal health.
Five fixed scenic cameras center on walls or water; both matched capture sets omit Glint at
those points. The other 35 staged poses were checked for clear feet. Movement proof comes
from the actual-input clips and contact probes.
