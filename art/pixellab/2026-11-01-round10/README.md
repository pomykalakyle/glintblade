# Round 10: browser fit and Colemak

The browser previously drew 384×216 inside 480×336, leaving 48px on each side and
60px above and below. Browser presentation now uses fractional contain scaling
with nearest filtering. The complete 16:9 picture reaches the available edges.
Desktop native integer scaling is unchanged.

Fresh Player Cards request 480×294: 270px of game plus a 24px footer. Older posts
may retain their cached 480×360 shape; those now show 480×270 gameplay rather
than 384×216. Short touch frames use a 64px control dock, with all controls
outside the picture. A ResizeObserver keeps the render buffer synchronized with
CSS size changes.

Software Colemak can report `code: KeyJ, key: n`. The runtime previously read
KeyJ, so N/slash failed. The browser now forwards the typed letter as KeyN.
Releases stay mapped to the original logical key even if Shift or the layout
changes while held. Physical and touch holds use reference counts. Colemak-DH
remains W/A/R/S to move, N to slash, E to dash; WASD stays the default.

## Validation

- [Browser checks](browser-checks.json): scene fit, HUD corners, footer/touch separation, real fullscreen, startup/failure handling and input.
- [Geometry measurements](geometry.json): 11 sizes including old/new cards,
  proportional mobile 390×239, portrait, fullscreen and DPR 2.
- [Colemak checks](colemak-checks.json): 19 trusted CDP cases, including both
  firmware/software N/E, WARS, punctuation, changed keyup, shared holds and blur.
  Action PNGs were reviewed directly.
- [Source pins](sources.json) and [build/check scope](verification.json).
  Native/WASM builds pass; the native title PNG equals R9. The prior R9 native
  suite remains 255 passed, 3 existing performance-budget failures and 7 ignored;
  it was not rerun for this browser presentation and input patch.

Reproduce with fresh output directories after a WASM build:

```sh
uv run --with playwright python tools/pixellab/test_web_player.py --out out/web-fit-new
uv run --with playwright python tools/pixellab/test_web_colemak.py --out out/colemak-new
```

Requires installed Playwright Chromium and `web/glintblade.wasm`. These local tests publish nothing. Captures remain under `out/pixellab/round10/`, outside Git. X may cache card dimensions separately from the live game.
