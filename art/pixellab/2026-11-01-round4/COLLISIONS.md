# Collision and support review, round 4

Trees and ruins used approximate collision boxes from their previous art. Tree B could be entered 18 native pixels past its visible root. Broken pillars extended three rows below their old collision box. The short entry pier could be walked through at y=598 because its old anchor included an eight-row staging gutter.

The new support spans follow the opaque structural base of each native drawing, with source-pixel anchors, actual root gaps and matching flip transforms. Tall crowns/facades remain passable behind their grounded bases. Contact shading and y sorting follow the visible foot. Native support anchors use the renderer’s floor rule, including fractional Glasswater positions. Existing footprint=None, alive=false and breakability semantics remain intact.

All four regions now accept a native-pixel physical surface derived from their painter’s material classifications. The full player feet box meets the same rounded shore, bridge deck or void edge that is drawn. The coarse map remains available for structure/navigation.

Validation: seven focused world tests passed, including 576 real Player::update walk/dash approaches through 36 solid variants, both flips and four cardinal directions. A raster-versus-collision test covers fractional anchors and source gaps. All 44 exported support animations were in bounds, opaque and nonoverlapping. All 66 exported asset files rebuilt byte for byte. A Glasswater lighting regression verifies that refining collision dimensions does not remove crystal keys.

The rc2 authored-map review used 11 real-input approach scripts. Tree B left contact changed from x=238.09 to x=220.09; the broken pillar front contact moved from y=618.41 to y=621.41; the entry pier stopped the player at x=156.95 instead of allowing traversal to x=284.09. The aqueduct’s right support now stops x=396.41 rather than x=387.41. Its central passage remains traversable. At world170,612 the hero now renders in front of the entry pier’s true foot instead of behind its old gutter anchor.

Raw captures, commands, traces, final contact sheets and binary hashes are in out/pixellab/round4/collisions/. Canonical comparison manifests use before-valid-aqueduct and after-rc2; the earlier invalid aqueduct starts remain archived but are superseded. These focused probes use peaceful/god QA flags and do not establish difficulty balance. Root’s subsequent independent review and final full-suite/build results are recorded separately.
