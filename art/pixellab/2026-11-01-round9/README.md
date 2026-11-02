# Round 9: daylight and browser player

Bloomwild now keeps its daylight exposure as the sun and native shadows move.
The title and upright cliffs/columns receive the same fill treatment as the ground.
See [the independent visual audit](daylight-audit.md) for 26 directly reviewed stills
and matched luma measurements.

The final build also resets new runs to WASD unless Colemak is explicitly selected.
The browser shell adds Player Card metadata, sound activation, download progress,
error recovery, and touch controls without changing the pinned Macroquad bundle.
Its files use a release query to bypass older browser caches.

- [Exact source pins](sources.json): 459 build inputs.
- [Source verification](source-verification.json): no mismatches.
- [Normal verification](normal-verification.json): all 78 frozen pairs match.
- [Night-region isolation](night-isolation.json): all 25 matched PNGs unchanged.
- [Browser checks](browser-checks.json): isolated Chromium, X-style cross-origin
  sandbox, keyboard/touch, mixed input, fullscreen refusal, and download failure.
- [Build hashes and scope](verification.json).
- [Complete native test log](tests.log): 255 passed, 3 existing performance-budget
  failures, 7 ignored. The suite does not pass as a whole.

The browser test used software WebGL and local-response interception; its timing
is not a cross-device benchmark. The card mechanism is proven by POTETO’s live
tibo.dog example, but this does not prove acceptance of a new GLINTBLADE card
on every X client. No X post was created.

Historical round 8 recordings and hashes remain in their original directories.
The native/WASM binaries and matched captures are kept under
`out/pixellab/round9/selected/` on the working machine, outside Git.
