# Silent gameplay showcase

Written 2026-10-30; updated 2026-10-31.
Capture real game frames without a window, audio device, playback or external service.
Python uses only its standard library; `ffmpeg` and `ffprobe` must already be installed.

```sh
SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX26.5.sdk cargo build --release
python3 tools/showcase/capture.py
```

The default gallery lives in `out/samples/capture_tooling/WIP_GALLERY.md`.
It includes a title PNG, two PNGs per region, a combat PNG, all three Warden phases, overload and victory PNGs, and two muted MP4s.
Every PNG is an unretouched engine frame enlarged to 1152x648 with nearest-neighbor pixels.
Each still also has a `_native1x.png` companion: the original 384x216 engine PNG captured with `--scale 1`, copied byte-for-byte before the temporary sequence is deleted.
These native stills are never inferred by downsampling.
`composite_scaled` uses exact nearest-neighbor scaling only when its bloom buffer is zero; its alternate bloom path interpolates at the requested scale.
The showcase always captures at scale 1, then enlarges those native frames with ffmpeg, preserving the existing 3x stills and videos regardless of that engine branch.
The videos sample every four simulation ticks, encoded at 15 fps for real-time playback.
`ffprobe` verifies codec, dimensions, frame count, duration and the absence of audio streams.
Temporary frame sequences are removed after encoding; PNGs, MP4s and a provenance manifest remain.
Each run hashes and executes a private copy of the release binary so concurrent rebuilds cannot mix versions in one gallery.
The copy is guarded like `out/style_v2_baseline/glintblade`: a shell wrapper accepts only `shot` or `zoneview`, and the original executable bytes live in its `.real` companion.
Never run the `.real` companion directly, even for help.
`--build` uses a private temporary `CARGO_TARGET_DIR`, leaving the shared release target alone.
The prior manifest and gallery page are invalidated at startup and published again only when every capture succeeds.

After the final art is embedded, the night lead can regenerate the gallery:

```sh
python3 tools/showcase/capture.py --build --final --out out/final
```

`--final` always builds from frozen inputs, so `--build` is optional in that mode.
The final bundle retains a uniquely named `renderer-*` directory with the guarded renderer, its `.real` companion, SHA256SUMS and a source snapshot.
Only build sources and the runtime asset directories/extensions embedded by `build.rs` are copied, respecting `GLINT_ASSETS` if set.
The source snapshot also includes this capture script and README.
The tool records source HEAD, dirty file status, hashes of every copied source/runtime asset, build command/environment and compiler versions.
If source HEAD, dirty status or input bytes change during the freeze, it fails before building rather than claiming a mixed snapshot.
The build and captures then use only that frozen snapshot and private renderer; later shared-main edits cannot change the gallery.
The manifest checks renderer and frozen-input hashes again before publishing.
WIP captures without `--final` do not claim that checkout source hashes describe an already-built renderer.
A WIP `--build` run is labelled `live_checkout_build`; a reused binary remains `unverified_existing_binary`.
Git identity is recorded only when the capture source is the repository root, so retained snapshots cannot borrow a parent checkout's HEAD.
A `GLINT_ASSETS` override records its own scoped Git state separately from source state; unversioned asset copies are identified as untracked and remain pinned by their content hashes.
Retained renderer directories are mode 0755 so copied bundles remain readable and replayable by another user.

`REPLAY.sh` and `REPLAY.md` are published with successful final captures.
Keep the entire output directory together and run `sh REPLAY.sh` on a compatible Mac to regenerate raw native sequences and engine reports under `replay/`, without a checkout or build.
Every renderer command explicitly uses `shot`; no window, audio device or playback is involved.
The manifest keeps the actual invocation records intact and adds relative replay commands, per-still capture/frame mappings, SHA-256 hashes, and the existing ffmpeg still/video recipes.
Run each recipe from the bundle directory after the raw sequence replay to reproduce the published images and muted videos.
The retained renderer embeds its runtime assets; Cargo dependencies, macOS libraries and the compiler are not vendored.
The source snapshot supports auditing/rebuilding with the existing toolchain, while the retained renderer is the exact rendering replay artifact.
On reruns, prior final manifest/gallery/replay files are invalidated first; prior uniquely named renderer snapshots remain available.

The Warden pilot walks toward exposed weak points and taps the normal sword through `Game::tick`.
The gallery starts outside awakening range at 720,520, waits 300 ticks through ordinary empty input, then walks in.
Recording starts at tick 300, after the arrival banner, and the phase-one still is anchored to `warden_awake + 300` rather than a fixed simulation tick.
The raw developer pilot keeps its existing near-boss start and zero delay unless these options are supplied.
The pilot uses `--god` to keep the capture reliable; no health, phase, death or victory state is injected.
The overload still is taken before the burst, when the damaged Keeper is lit from within and Glint remains clear.
The full boss video preserves the actual shatter, scene fade and victory presentation.
The phase-two PNG shows its exposed-heart opening after its shields have broken.
The phase-three PNG follows damage below the preceding phase's health floor, proving combat in that phase rather than just a transition.
The combat capture must defeat at least one enemy or the script fails.
Milestone ticks, exact commands, seed, god-mode flag, ending scene, capture time and binary SHA-256 are saved in the manifest.
Original command records keep their temporary output directories for provenance; final-mode replay recipes already replace those with usable paths.

Individual headless runs:

```sh
./target/release/glintblade shot --pilot warden --pos 720,520 --pilot-delay 300 --from 300 --god --until-victory --ticks 24000 --every 4 --scale 1 --out out/warden-frames --report out/warden.json
./target/release/glintblade shot --zone bloomwild --script 'W190 R35 A1 W20' --from 192 --every 4 --scale 3 --out out/combat-frames
SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX26.5.sdk cargo test --release tools::tests::warden_pilot_reaches_real_victory -- --nocapture
```

`--until-victory` fails if the victory prompt is not reached within the tick limit.
The pilot belongs only to the developer `shot` frontend and cannot affect normal windowed play.
It relies on the current Warden hurtbox part-ID ranges; changed boss mechanics may require an updated pilot.
The gallery's region coordinates and combat script are composed samples, not a complete traversal of the four regions.
The WIP captures reflect the art embedded when the binary was built, so regenerate after art changes.
