# Fresh-load N/E actions

The previous keyboard translation fix was tested after switching the game to
Colemak. A new browser load still starts with WASD, where N/E were inactive.
X/C worked because those were unconditional aliases.

N now slashes and E now dashes in either layout, including immediately after a
fresh load. WASD remains the default movement layout; Tab selects Colemak WARS
on the title or pause screen. Existing J/K and X/C bindings remain available
as before. No desktop keyboard or system shortcuts were changed.

The browser asset URLs now use `controls-r12` to bypass the older cached build.
The published game footer has no art credit.

[Keyboard checks](colemak-checks.json) pass all 23 cases. Four new cases exercise
firmware/software N/E before any Tab selection. The saved action PNGs were
reviewed directly: the fresh WASD title is visible before N produces a slash
and E produces a dash. Existing selected-Colemak movement, release and mixed
key cases still pass. Captures remain under `out/pixellab/round12/colemak/`.

These key-event assertions are paired with direct visual review of actual game
actions; they do not claim to capture Kyle’s physical keyboard hardware.
