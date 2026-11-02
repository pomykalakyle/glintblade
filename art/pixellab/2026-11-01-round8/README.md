# Glint Blade: visible dynamic lighting, round 8

Round 8 makes the existing normal maps respond more visibly during ordinary play.
Bloomwild's sun moves over a 36-second arc; the same direction drives surface
shading and native silhouette shadows. Ground foliage now uses its actual surface
orientation. Narrow contact shading remains while the old fixed directional casts
are removed from the physical lighting path.

Glint's blade lights nearby surfaces in daylight as well as caves. Glasswater's
crystals and luminous enemies use their actual moving native pixels; existing
breakable crystal sprigs and defeated enemies remove their lights. Larger authored
crystal landmarks retain their original indestructible behavior. Foundry lighting
follows exposed hot patches in the actual flowing lava. The Warden's heart, fists,
shield crystals and beams change local light through their existing animations.

The art, normal-map data and combat archetypes are retained. The 78 selected
normal families remain frozen at the [round 7 selection](../2026-11-01-round7/normals/README.md).
Exact visibility and composition optimizations are checked against original
algorithms rather than reducing shadow coverage or update frequency.

## Selected local build: trial 8

The selected [trial 8 artifacts](../../../out/pixellab/round8/selected/artifacts.json)
preserve native SHA256 `6d0e45f42e4711bfb344e66b8e50b5a581f435ace23dfd098255a8361f1fb966`
and WASM SHA256 `677983e008ebad00145e63bb440b1fdbcb3327e670cca5b87949b2d5fbe8f233`.
The selected, target and local `web/glintblade.wasm` copies match. All
[459 source/asset pins](../../../out/pixellab/round8/selected/source-verification-after-build.json)
verified unchanged after the build. The [normal audit](normal-audit.json) passes
all 78 frozen families: exact source/normal hashes, dimensions and alpha.

The final [scene studies](../../../out/pixellab/world/round8-selected/manifest.json)
were reviewed directly: 15 Bloomwild views plus the title, nine Glasswater views,
nine Cinder views and seven Summit views. Glasswater's 1.22 tall-landmark light
gain remains selected; its nine final views are byte-identical to the approved
candidate. Lava radiant gain 3.0 remains selected after all nine Cinder views and
a 360-frame lava-bank capture: 24 camera-matched timestamps were measured and selected source frames inspected directly. These are scoped visual checks,
not a claim that every frame of the game has been inspected. All 45 fresh PNGs
(40 studies, four overviews and title) exactly match the independently reviewed
trial 7 output. The only trial 8 code change smooths the Warden prism
charge-to-fire handoff: exactly three frames in the 301-frame twin-beam sequence
change, with the remainder byte-identical. The premature heart opening and
one-frame early finale body cutoff are also fixed.

The ground-foliage motion audit scans all 480 courtyard frames across three leaf
patches and directly checks the largest measured adjacent change: highlights
remain readable, without a dark collapse or stencil edge. Temple and courtyard
casts move coherently during the shared 36-second sun arc.

## Checks and remaining performance limits

The quiet serial [release suite](../../../out/pixellab/round8/trial-8/full-tests.log)
reports **254 passed, three failed, seven ignored**. All executed functional tests
passed. The three failures are the unchanged performance gates: default rendering
(mean <6 ms / p95 <8 ms), live Bloomwild (mean/p95 <=8 ms), and sword-break frames
(<8 ms). The suite does not pass as a whole.

The unchanged Cinder path was separately measured in the trial 7 [timing run](../../../out/pixellab/round8/trial-7/cinder-timing.log)
also fails its existing 8 ms gate at the caldera: p95 8.918 ms. That run stopped
before the fall case. Real Warden firing measurements include rendering, HUD and
final grade, with live hazards and particles:

| Warden case | Mean | p95 | Maximum |
| --- | --- | --- | --- |
| Phase 2, one beam | 11.61 ms | 12.06 ms | 12.15 ms |
| Phase 3, twin beam | 14.05 ms | 14.57 ms | 14.83 ms |

These [render-only measurements](../../../out/pixellab/round8/trial-8/warden-timing.log)
exclude simulation updates and frontend overhead. They do not establish
end-to-end 60 fps. Browser runtime performance has not been measured.

The selected build completed the [full campaign](../../../out/pixellab/recordings/whole-game-round8-selected/gameplay.mp4):
14,498 frames / 241.63 seconds, all four regions, all shards and Warden phases,
then victory. The [comparison](../../../out/pixellab/round8/selected/campaign-comparison.json)
confirms identical gameplay fields, milestones and all 1,452 audio events to R7.
The sole trace difference is intentional finale camera framing at tick 13830:
`[528,207]` becomes `[528,204]`. The final handoff-fix campaign also matches
trial 7 state/audio exactly. Audio mix has no missing sounds.

The new [eight-second GIF](../../../out/share/glint-blade-round8.gif) contains actual
selected-build gameplay across four regions, 15 fps at normal speed, native HUD
only, nearest-neighbor 2x output. Its [manifest](../../../out/share/glint-blade-round8.json)
pins all 120 source frames. Existing round 7 GIFs remain unchanged.
See [verification.json](verification.json) for artifact hashes and exact review scopes.

## Rechecking and transfer

The offline [release helper](../../../tools/pixellab/verify_lighting_release.py)
supports `normals --out`, `snapshot --out` and `verify --sources [--out]`. Every
output must be a new JSON file. It checks frozen normal data without regenerating
art or overwriting round 7 reports, and rejects source inventory additions,
removals and hash changes. The source snapshot includes complete source/example
trees, compile-time map data, Cargo/build/toolchain files and all assets selected
by `build.rs`.

The [portable source and web bundle](../../../out/share/glint-blade-round8-source-and-web.zip)
includes all 459 pinned files, new untracked renderer files and normal maps, the
release helper, evidence and current compiled browser folder. Extract into a fresh
folder and follow `ROUND8-INSTALL.txt`. Hash verification requires Python 3.11+;
a new machine also needs Rust/Cargo and may download Cargo dependencies. A plain `git pull` or ordinary `git diff` does not transfer
this working tree; no Git push has been made. Build outputs, recordings and
`web/glintblade.wasm` are ignored by Git and need explicit transfer or rebuilding.
The selected native executable is a Linux build, not a Mac build.

Verify the saved `sources.json` on the destination, then rebuild and restart with
`GLINT_ASSETS`, `GLINT_SPRITES` and `GLINT_LIGHTING` unset for the default renderer.
For a local browser build, copy the new WASM to `web/glintblade.wasm` and compare
hashes. Source equality does not prove that a launched binary was rebuilt;
platform, toolchain and build environment also affect binary hashes.

This remains a 2.5D sprite renderer: native silhouettes, finite foot volumes and
painted terrain sheets approximate visible geometry. Hidden sides are not
reconstructed. Lava patches and the finale's beam lights approximate extended
emitters with local point sources; they are not a volumetric light transport
simulation. Existing painted shading remains part of the pixel art.

Campaign QA uses the real input pilot with invulnerability. It checks progression
and rendering coverage, and does not establish ordinary-player combat balance.
Static scene studies are separate from gameplay recordings; motion and audio
review claims must state the actual sampled scope.
