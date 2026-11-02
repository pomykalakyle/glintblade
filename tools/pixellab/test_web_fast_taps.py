#!/usr/bin/env python3
"""Check actual browser actions for held keys and taps shorter than one game frame.

Run after assembling the browser build with prepare_web_build.py:
  uv run --offline --with playwright python tools/pixellab/test_web_fast_taps.py \
    --web-dir web --out out/pixellab/round13/fast-fixed
Use --public --expected-build COMMIT to verify the deployed Pages engine instead.
Only the outer X-style wrapper is local in public mode. No publishing.

For the unmodified R12 WASM, --expect baseline accepts the reproduced input regression
only when held Colemak N/E works and deterministic same-frame N/E taps fail.
The JSON still records every final-contract failure, including QWERTY N/E.

There is no test HTTP server. Local files are served by Playwright route handlers
at their normal HTTPS origin inside the demonstrated X Player Card sandbox.
CDP supplies trusted events; JS-dispatched events are never used as test inputs.
Action evidence comes from the actual game's slash/dash audio source starts,
matched against the repository's decoded WAV durations. Key forwarding alone
does not count as a successful action.
"""

from __future__ import annotations

import argparse
import asyncio
import base64
import hashlib
import json
import mimetypes
import pathlib
import time
import urllib.parse
import wave

from playwright.async_api import async_playwright


ROOT = pathlib.Path(__file__).resolve().parents[2]
BASE = "https://pomykalakyle.github.io/glintblade/"
WRAPPER = "https://x.com/glint-fast-taps-independent-check"
SANDBOX = "allow-popups allow-same-origin allow-scripts allow-presentation"

PARSER = argparse.ArgumentParser(description=__doc__)
PARSER.add_argument("--web-dir", type=pathlib.Path, default=ROOT / "web")
PARSER.add_argument("--wasm", type=pathlib.Path, help="Override only glintblade.wasm")
PARSER.add_argument("--public", action="store_true",
                    help="Use actual GitHub Pages resources; no local game asset interception")
PARSER.add_argument("--expected-build", help="Require this exact published build label/commit")
PARSER.add_argument("--out", type=pathlib.Path, required=True,
                    help="A fresh output directory; existing paths are refused")
PARSER.add_argument("--expect", choices=("fixed", "baseline"), default="fixed")
PARSER.add_argument("--fast-repeats", type=int, default=4)
PARSER.add_argument("--max-gap-ms", type=float, default=1.0)
PARSER.add_argument("--deterministic-repeats", type=int, default=2)
PARSER.add_argument("--realtime-supplement", action="store_true",
                    help="Also run nondeterministic measured sub-ms CDP pairs")
PARSER.add_argument("--adjudicate-existing", type=pathlib.Path,
                    help="Re-evaluate an existing raw report with the corrected windup oracle; no browser")
PARSER.add_argument("--group", choices=("all", "qwerty", "firmware", "software"),
                    default="all")
ARGS = PARSER.parse_args()
WEB = ARGS.web_dir.resolve()
OUT = ARGS.out.resolve()
if ARGS.fast_repeats < 2 or ARGS.deterministic_repeats < 2 or ARGS.max_gap_ms <= 0:
    PARSER.error("Use at least two fast repeats and a positive max gap")
if ARGS.public and ARGS.wasm:
    PARSER.error("--public cannot override the deployed WASM with --wasm")
if ARGS.expected_build and not ARGS.public:
    PARSER.error("--expected-build requires --public")
OUT.mkdir(parents=True, exist_ok=False)
GAME_URL = BASE + ("?v=controls-r13" if ARGS.public else "")


def sha(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def wav_evidence() -> dict:
    result = {}
    for path in sorted((ROOT / "assets/sfx").glob("*.wav")):
        with wave.open(str(path), "rb") as wav:
            result[path.stem] = {
                "frames": wav.getnframes(), "sample_rate": wav.getframerate(),
                "seconds": wav.getnframes() / wav.getframerate(),
            }
    return result


WAVS = wav_evidence()
ACTION_NAMES = {
    "slash": ["slash_1", "slash_2", "slash_3"],
    "dash": ["dash", "dash_2"],
}
REPORT = {
    "expect": ARGS.expect, "web_dir": None if ARGS.public else str(WEB),
    "wasm": None if ARGS.public else str(ARGS.wasm.resolve() if ARGS.wasm else WEB / "glintblade.wasm"),
    "public": ARGS.public, "url": GAME_URL, "expected_build": ARGS.expected_build,
    "inputs": {}, "action_wavs": {name: WAVS[name]
        for names in ACTION_NAMES.values() for name in names},
    "duration_tolerance_seconds": 0.0001,
    "duration_collisions": {}, "groups": [], "checks": {}, "errors": [],
    "scope": "Headless Chromium, software WebGL; "
             + ("actual public GitHub Pages resources" if ARGS.public else "actual local WASM")
             + " in X's "
             "demonstrated iframe sandbox. No X post or desktop interaction. "
             "Required deterministic taps cancel the game RAF, freeze Date.now, "
             "deliver trusted down/up without a frame, optionally render at "
             "unchanged time (zero ticks), then advance 17 ms and render. "
             "Optional real-time taps qualify only with trusted down/up < 1 ms "
             "(or explicit max-gap option) and no intervening game frame. "
             "This tests a candidate cause, not the user's physical keyboard.",
}
for action, names in ACTION_NAMES.items():
    for name in names:
        REPORT["duration_collisions"][name] = [other for other, info in WAVS.items()
            if other not in names and
            abs(info["seconds"] - WAVS[name]["seconds"]) < 0.0001]
if not ARGS.public:
    for name in ("index.html", "player.js", "player.css", "mq_js_bundle.js"):
        path = WEB / name
        if path.exists():
            REPORT["inputs"][name] = sha(path)
    REPORT["inputs"]["glintblade.wasm"] = sha(pathlib.Path(REPORT["wasm"]))
    manifest_path = WEB / "build-info.json"
    if manifest_path.exists():
        REPORT["build_manifest"] = json.loads(manifest_path.read_text())
        manifest_wasm = WEB / REPORT["build_manifest"]["wasm"]
        REPORT["inputs"][manifest_wasm.name] = sha(manifest_wasm)


PUBLIC_PROOF_INIT = r"""
window.publicWasmProofs = {};
const originalFetch = window.fetch;
window.fetch = async function(...args) {
  const response = await originalFetch.apply(this,args);
  if (response.url.startsWith('https://pomykalakyle.github.io/glintblade/') &&
      /\.wasm(?:\?|$)/.test(response.url)) {
    // Inspector response caches can evict the large WASM body. Hash a clone of
    // the very response the game consumes, without issuing another fetch.
    const clone = response.clone();
    publicWasmProofs[response.url] = clone.arrayBuffer().then(async bytes => ({
      bytes:bytes.byteLength,
      sha256:[...new Uint8Array(await crypto.subtle.digest('SHA-256',bytes))]
        .map(value=>value.toString(16).padStart(2,'0')).join('')
    }));
  }
  return response;
};
"""


INIT = r"""
window.fastTapProbe = {trusted: [], sounds: [], engineFrames: 0,
                       frameTimes: [], normalized: []};
for (const type of ['keydown', 'keyup']) {
  // Capture at window: the player's canvas bridge intentionally stops the
  // original event before later canvas listeners would see it.
  window.addEventListener(type, event => {
    if (!event.isTrusted || event.target?.id !== 'glcanvas') return;
    fastTapProbe.trusted.push({type, key:event.key, code:event.code,
      at:performance.now(), frame:fastTapProbe.engineFrames,
      repeat:event.repeat, trusted:event.isTrusted});
  }, true);
}
const sourceStart = AudioBufferSourceNode.prototype.start;
AudioBufferSourceNode.prototype.start = function(...args) {
  fastTapProbe.sounds.push({seconds:this.buffer?.duration, loop:this.loop,
    sampleRate:this.buffer?.sampleRate, at:performance.now(),
    frame:fastTapProbe.engineFrames});
  return sourceStart.apply(this,args);
};
"""

INSTALL_ENGINE_PROBE = r"""() => {
  const original = wasm_exports;
  const wrapped = {...original};
  wrapped.frame = function(...args) {
    fastTapProbe.engineFrames++;
    fastTapProbe.frameTimes.push(performance.now());
    if (fastTapProbe.frameTimes.length > 100) fastTapProbe.frameTimes.shift();
    return original.frame(...args);
  };
  for (const name of ['key_down','key_up']) {
    wrapped[name] = function(...args) {
      fastTapProbe.normalized.push({type:name,args,at:performance.now(),
        frame:fastTapProbe.engineFrames});
      return original[name](...args);
    };
  }
  wasm_exports = wrapped;
  return {exports:Object.keys(original), canvas:[canvas.width,canvas.height]};
}"""


def classify(sounds: list[dict]) -> list[dict]:
    result = []
    for sound in sounds:
        if sound.get("loop") or sound.get("seconds") is None:
            continue
        for action, names in ACTION_NAMES.items():
            matching = [name for name in names
                        if abs(sound["seconds"] - WAVS[name]["seconds"]) < 0.0001]
            # slash_3 has the exact duration of beetle_charge. A duration-only
            # match cannot distinguish those; exclude it as success evidence.
            matching = [name for name in matching
                        if not REPORT["duration_collisions"].get(name)]
            if matching:
                result.append({"action": action, "wav": matching, **sound})
    return result


def expected_timing(action: str | None, actions: list[dict], first_tick: dict) -> bool:
    """Slash SFX starts at its strike, after its three-tick anticipation.

    The source declares SWINGS[0].windup=3 and plays slash_1 only at t==strike
    (src/entities/player.rs). Require a real, bounded action start rather than
    incorrectly requiring that audio on the first accepted simulation tick.
    Nine further manual frames allows that designed windup; the entire 44-frame
    post-tap trace still has to contain exactly one matching action.
    """
    if action is None:
        return not actions
    return (len(actions) == 1 and actions[0]["action"] == action
            and 0 <= actions[0]["frame"] - first_tick["frame"] <= 9)


def adjudicate_recorded(report: dict) -> None:
    """Only correct the timing oracle; retain the original raw measurements."""
    report["oracle_revision"] = 2
    report["oracle_timing"] = (
        "Exactly one unambiguous action SFX, emitted no later than nine manual "
        "frames after the first +17 ms frame. Slash has a designed anticipation "
        "(SWINGS[0].windup=3) before its sound. No action at the unchanged-time "
        "render; check the entire following 44-frame trace for duplicates.")
    for group in report["groups"]:
        for case in group["cases"]:
            for trial in case.get("deterministic_trials", []):
                trial["action_at_expected_timing"] = expected_timing(
                    case["expected_action"], trial["actions"], trial["first_tick"])
                trial["success"] = (trial["qualified"] and trial["exactly_once"]
                                    and trial["action_at_expected_timing"])
            for zero_render in (False, True):
                kind = "zero-render-then-17ms" if zero_render else "direct-17ms"
                trials = [trial for trial in case.get("deterministic_trials", [])
                          if trial["kind"] == kind]
                if not trials:
                    continue
                suffix = "zero-render-then-tick" if zero_render else "direct-tick"
                report["checks"][f"{group['name']}:{case['name']}:{suffix}:actual-action-once"] = all(
                    trial["success"] for trial in trials)


async def main() -> None:
    async with async_playwright() as pw:
        browser = await pw.chromium.launch(headless=True, args=[
            "--no-sandbox", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"])
        groups = [
            ("qwerty", 0, [
                ("j-slash", "KeyJ", "j", "slash"),
                ("k-dash", "KeyK", "k", "dash"),
                ("n-disabled", "KeyN", "n", None),
                ("e-disabled", "KeyE", "e", None),
                ("x-alias", "KeyX", "x", "slash"),
                ("c-alias", "KeyC", "c", "dash"),
            ]),
            ("firmware", 1, [
                ("n-slash", "KeyN", "n", "slash"),
                ("e-dash", "KeyE", "e", "dash"),
                ("j-disabled", "KeyJ", "j", None),
                ("k-disabled", "KeyK", "k", None),
                ("x-alias", "KeyX", "x", "slash"),
                ("c-alias", "KeyC", "c", "dash"),
            ]),
            ("software", 1, [
                ("n-slash", "KeyJ", "n", "slash"),
                ("e-dash", "KeyK", "e", "dash"),
            ]),
        ]
        for group_name, layout, cases in groups:
            if ARGS.group != "all" and group_name != ARGS.group:
                continue
            group = {"name": group_name, "wanted_layout": layout, "cases": [], "served_wasm": []}
            REPORT["groups"].append(group)
            ctx = await browser.new_context(
                viewport={"width": 760, "height": 520}, device_scale_factor=1)
            if ARGS.public:
                await ctx.add_init_script(PUBLIC_PROOF_INIT)
            await ctx.add_init_script(INIT)

            async def route(request):
                url = request.request.url
                if url == WRAPPER:
                    await request.fulfill(content_type="text/html", body=(
                        '<meta name="viewport" content="width=device-width">'
                        '<style>body{margin:0;background:#111}iframe{border:0}</style>'
                        f'<iframe width="480" height="294" sandbox="{SANDBOX}" '
                        f'allow="autoplay; fullscreen; web-share" src="{GAME_URL}"></iframe>'))
                    return
                if url.startswith(BASE):
                    if ARGS.public:
                        await request.continue_()
                        return
                    relative = urllib.parse.unquote(
                        urllib.parse.urlsplit(url).path[len("/glintblade/"):])
                    asset = (WEB / (relative or "index.html")).resolve()
                    if not asset.is_relative_to(WEB):
                        await request.abort()
                        return
                    if relative == "glintblade.wasm" and ARGS.wasm:
                        asset = ARGS.wasm.resolve()
                    if not asset.is_file():
                        await request.fulfill(status=404, body="Missing test fixture")
                        return
                    content_type = ("application/wasm" if asset.suffix == ".wasm"
                                    else mimetypes.guess_type(str(asset))[0]
                                    or "application/octet-stream")
                    if asset.suffix == ".wasm":
                        group["served_wasm"].append({"url": url, "file": str(asset), "sha256": sha(asset)})
                    await request.fulfill(content_type=content_type, body=asset.read_bytes())
                    return
                await request.abort()

            await ctx.route("**/*", route)
            page = await ctx.new_page()
            responses = []
            page.on("pageerror", lambda error: REPORT["errors"].append(str(error)))
            page.on("response", lambda response: responses.append(response)
                    if response.url.startswith(BASE) else None)
            await page.goto(WRAPPER)
            game = page.frame_locator("iframe")
            start = time.monotonic()
            await game.locator("#play").click()
            await game.locator('body[data-state="ready"]').wait_for(timeout=120000)
            frame = next(item for item in page.frames if item.url.startswith(BASE))
            cdp = await ctx.new_cdp_session(page)
            await page.wait_for_timeout(2200)
            group["startup_seconds"] = round(time.monotonic() - start, 3)
            group["engine"] = await frame.evaluate(INSTALL_ENGINE_PROBE)

            if ARGS.public:
                manifests = [response for response in responses
                    if urllib.parse.urlsplit(response.url).path.endswith("/build-info.json")]
                REPORT["checks"][f"{group_name}:public-manifest-requested"] = bool(manifests)
                if manifests:
                    manifest_response = manifests[-1]
                    manifest = json.loads(await manifest_response.body())
                    digest = manifest.get("sha256", "")
                    valid_digest = (isinstance(digest, str) and len(digest) == 64
                                    and all(char in "0123456789abcdef" for char in digest))
                    hashed_name = "glintblade." + (digest[:16] if valid_digest else "invalid") + ".wasm"
                    group["public_manifest"] = manifest
                    REPORT["build_manifest"] = manifest
                    REPORT["checks"][f"{group_name}:public-manifest-valid"] = (
                        manifest_response.status == 200 and manifest.get("schema") == 1
                        and manifest.get("controls_revision") == 13 and valid_digest
                        and manifest.get("wasm") == hashed_name)
                    actual_build = await frame.evaluate("document.body.dataset.build")
                    group["page_build"] = actual_build
                    REPORT["checks"][f"{group_name}:public-build-agrees"] = manifest.get("build") == actual_build
                    if ARGS.expected_build:
                        REPORT["checks"][f"{group_name}:expected-public-build"] = manifest.get("build") == ARGS.expected_build
                    wasm_url = BASE + hashed_name
                    wasm_responses = [response for response in responses if response.url == wasm_url]
                    REPORT["checks"][f"{group_name}:public-immutable-wasm-requested"] = bool(wasm_responses)
                    if wasm_responses:
                        wasm_response = wasm_responses[-1]
                        proof = await frame.evaluate("(url)=>window.publicWasmProofs[url]", wasm_response.url)
                        group["served_wasm"].append({
                            "url": wasm_response.url, "status": wasm_response.status,
                            "bytes": proof["bytes"], "sha256": proof["sha256"],
                            "hash_source": "SHA256 of clone of actual browser fetch Response",
                            "content_type": wasm_response.headers.get("content-type"),
                        })
                        REPORT["wasm"] = wasm_response.url
                        REPORT["inputs"][hashed_name] = proof["sha256"]
                        REPORT["checks"][f"{group_name}:public-wasm-hash-matches"] = (
                            wasm_response.status == 200 and proof["sha256"] == digest)
                        REPORT["checks"][f"{group_name}:public-wasm-mime"] = (
                            "application/wasm" in wasm_response.headers.get("content-type", ""))
                group["public_responses"] = [{"url": response.url, "status": response.status}
                                             for response in responses]

            async def key(kind, code, letter):
                event = {"type": kind, "code": code, "key": letter,
                         "autoRepeat": False}
                if len(letter) == 1 and letter.isalpha():
                    event["windowsVirtualKeyCode"] = ord(letter.upper())
                await cdp.send("Input.dispatchKeyEvent", event)

            async def hold(code, letter, duration=100):
                await key("keyDown", code, letter)
                await page.wait_for_timeout(duration)
                await key("keyUp", code, letter)

            getter = await frame.evaluate("typeof wasm_exports.glint_key_layout === 'function'")
            setter = await frame.evaluate("typeof wasm_exports.glint_set_key_layout === 'function'")
            revision = await frame.evaluate("wasm_exports.glint_controls_revision?.() ?? null")
            group["layout_getter_available"] = getter
            group["layout_setter_available"] = setter
            group["controls_revision"] = revision
            if ARGS.expect == "fixed":
                REPORT["checks"][f"{group_name}:controls-revision-13"] = revision == 13
                REPORT["checks"][f"{group_name}:layout-exports"] = getter and setter
            if getter:
                group["fresh_layout"] = await frame.evaluate("wasm_exports.glint_key_layout()")
                REPORT["checks"][f"{group_name}:fresh-default-wasd"] = group["fresh_layout"] == 0
            if layout:
                if await game.locator("#layout").count():
                    await game.locator("#layout").select_option("1")
                    group["layout_selection"] = "published Controls dropdown"
                elif setter:
                    await frame.evaluate("wasm_exports.glint_set_key_layout(1)")
                    group["layout_selection"] = "actual WASM layout setter"
                else:
                    await hold("Tab", "Tab")
                    group["layout_selection"] = "trusted Tab on title; inspect title PNG"
            else:
                group["layout_selection"] = "untouched fresh default"
            if getter:
                group["selected_layout"] = await frame.evaluate("wasm_exports.glint_key_layout()")
                REPORT["checks"][f"{group_name}:selected-layout"] = group["selected_layout"] == layout
            if "build_manifest" in REPORT:
                REPORT["checks"][f"{group_name}:manifest-wasm-hash"] = bool(group["served_wasm"]) and all(
                    item["sha256"] == REPORT["build_manifest"]["sha256"] for item in group["served_wasm"])

            async def snapshot(name):
                # A manual draw is used only for the saved evidence image, never
                # between a tested key pair or as the action success oracle.
                data = await frame.evaluate("""() => new Promise(resolve =>
                  requestAnimationFrame(() => {wasm_exports.frame();
                    resolve(canvas.toDataURL('image/png'));}))""")
                (OUT / name).write_bytes(base64.b64decode(data.split(",", 1)[1]))

            await snapshot(f"{group_name}-title.png")
            await hold("Enter", "Enter")
            await page.wait_for_timeout(5400)
            dash_number = 0

            async def one_trial(code, letter, action, fast):
                nonlocal dash_number
                # Alternate the dash direction so repeated controls do not carry
                # the character away from the safe spawn or into a portal.
                if action == "dash":
                    direction = "ArrowLeft" if dash_number % 2 == 0 else "ArrowRight"
                    dash_number += 1
                    await hold(direction, direction, 80)
                await page.wait_for_timeout(700)
                await game.locator("#glcanvas").focus()
                await frame.evaluate("""() => {
                  fastTapProbe.trusted.length=0;fastTapProbe.sounds.length=0;
                  fastTapProbe.normalized.length=0;
                }""")
                barrier = await frame.evaluate("""() => new Promise(resolve =>
                  requestAnimationFrame(() => resolve({at:performance.now(),
                    frame:fastTapProbe.engineFrames})))""")
                if fast:
                    # Enqueue both CDP commands without awaiting the first round
                    # trip. Order on the one CDP session is preserved. Measured
                    # DOM timing, rather than the requested delay, qualifies it.
                    await asyncio.gather(key("keyDown", code, letter),
                                         key("keyUp", code, letter))
                else:
                    await hold(code, letter, 100)
                await page.wait_for_timeout(300)
                probe = await frame.evaluate("""() => ({
                  trusted:fastTapProbe.trusted,sounds:fastTapProbe.sounds,
                  normalized:fastTapProbe.normalized,
                  endingFrame:fastTapProbe.engineFrames})""")
                downs = [event for event in probe["trusted"]
                         if event["type"] == "keydown" and event["code"] == code]
                ups = [event for event in probe["trusted"]
                       if event["type"] == "keyup" and event["code"] == code]
                gap = ups[0]["at"] - downs[0]["at"] if downs and ups else None
                same_frame = bool(downs and ups and downs[0]["frame"] == ups[0]["frame"])
                actions = classify(probe["sounds"])
                qualified = (gap is not None and 0 <= gap < ARGS.max_gap_ms
                             and same_frame) if fast else bool(downs and ups)
                success = (any(item["action"] == action for item in actions)
                           if action else not actions)
                return {"kind": "fast" if fast else "held-100ms", "barrier": barrier,
                        "gap_ms": gap, "same_game_frame": same_frame,
                        "qualified": qualified, "actions": actions,
                        "success": success, **probe}

            for label, code, letter, action in cases:
                case = {"name": label, "code": code, "key": letter,
                        "expected_action": action, "fast_trials": []}
                group["cases"].append(case)
                case["held"] = await one_trial(code, letter, action, False)
                REPORT["checks"][f"{group_name}:{label}:held"] = case["held"]["success"]
                # These uncontrolled pairs are supplemental: the deterministic
                # clock below is required, regardless of their scheduling luck.
                if not ARGS.realtime_supplement:
                    continue
                needed = 2 if "alias" in label or "disabled" in label else ARGS.fast_repeats
                max_attempts = needed * 6
                while len([trial for trial in case["fast_trials"] if trial["qualified"]]) < needed:
                    if len(case["fast_trials"]) >= max_attempts:
                        break
                    trial = await one_trial(code, letter, action, True)
                    case["fast_trials"].append(trial)
                qualified = [trial for trial in case["fast_trials"] if trial["qualified"]]
                case["qualified_count"] = len(qualified)
                case["qualified_successes"] = sum(trial["success"] for trial in qualified)
                REPORT["checks"][f"{group_name}:{label}:enough-fast-pairs"] = len(qualified) >= needed
                REPORT["checks"][f"{group_name}:{label}:fast"] = (
                    len(qualified) >= needed and all(trial["success"] for trial in qualified))

            # Own the game's clock after all real-time controls are complete.
            # Calling wasm_exports.frame() directly does not request another RAF.
            # No source test hooks are used; env.now in the pinned bundle reads
            # Date.now()/1000, so this drives the real frontend accumulator.
            group["controlled_clock"] = await frame.evaluate("""() => {
              cancelAnimationFrame(animation_frame_timeout);
              animation_frame_timeout = 0;
              fastTapProbe.clock = {ms:Date.now()};
              Date.now = () => fastTapProbe.clock.ms;
              wasm_exports.frame();
              wasm_exports.frame();
              return {ms:Date.now(),frame:fastTapProbe.engineFrames,
                clockImport:importObject.env.now.toString()};
            }""")

            async def manual_frames(count=1, milliseconds=17):
                return await frame.evaluate("""({count,milliseconds}) => {
                  for (let index=0; index<count; index++) {
                    fastTapProbe.clock.ms += milliseconds;
                    wasm_exports.frame();
                  }
                  return {clock:Date.now(),frame:fastTapProbe.engineFrames,
                    sounds:[...fastTapProbe.sounds]};
                }""", {"count": count, "milliseconds": milliseconds})

            async def deterministic_trial(code, letter, action, zero_render):
                nonlocal dash_number
                await manual_frames(42)
                if action == "dash":
                    direction = "ArrowLeft" if dash_number % 2 == 0 else "ArrowRight"
                    dash_number += 1
                    await key("keyDown", direction, direction)
                    await manual_frames()
                    await key("keyUp", direction, direction)
                    await manual_frames()
                await game.locator("#glcanvas").focus()
                before = await frame.evaluate("""() => {
                  fastTapProbe.trusted.length=0;fastTapProbe.sounds.length=0;
                  fastTapProbe.normalized.length=0;
                  return {clock:Date.now(),frame:fastTapProbe.engineFrames};
                }""")
                # Serial trusted events are intentional. With no scheduled game
                # frame and a frozen simulation clock, both belong to the same
                # engine frame even if CDP round trips take multiple real ms.
                await key("keyDown", code, letter)
                await key("keyUp", code, letter)
                delivered = await frame.evaluate("""() => ({
                  clock:Date.now(),frame:fastTapProbe.engineFrames,
                  trusted:[...fastTapProbe.trusted],
                  normalized:[...fastTapProbe.normalized],
                  sounds:[...fastTapProbe.sounds]})""")
                zero = await manual_frames(milliseconds=0) if zero_render else None
                tick = await manual_frames(milliseconds=17)
                release = await manual_frames(milliseconds=17)
                settled = await manual_frames(42)
                actions = classify(settled["sounds"])
                down = [event for event in delivered["trusted"] if event["type"] == "keydown"]
                up = [event for event in delivered["trusted"] if event["type"] == "keyup"]
                delivery_ok = (len(down) == len(up) == 1 and
                    down[0]["frame"] == up[0]["frame"] == before["frame"] == delivered["frame"] and
                    before["clock"] == delivered["clock"])
                zero_ok = (zero is None or
                    (zero["clock"] == before["clock"] and not classify(zero["sounds"])))
                expected = 1 if action else 0
                count = sum(item["action"] == action for item in actions) if action else len(actions)
                once = count == expected and (not action or len(actions) == 1)
                tick_actions = classify(tick["sounds"])
                tick_ok = (len(tick_actions) == 1 and tick_actions[0]["action"] == action
                           if action else not tick_actions)
                timing_ok = expected_timing(action, actions, tick)
                return {"kind": "zero-render-then-17ms" if zero_render else "direct-17ms",
                    "before": before, "delivered": delivered,
                    "zero_render": zero, "first_tick": tick,
                    "release_tick": release, "settled": settled,
                    "actions": actions, "actual_action_count": count,
                    "trusted_same_frame_delivery": delivery_ok,
                    "zero_render_no_action": zero_ok,
                    "action_on_first_tick": tick_ok,
                    "action_at_expected_timing": timing_ok,
                    "exactly_once": once,
                    "qualified": delivery_ok and zero_ok,
                    "success": delivery_ok and zero_ok and once and timing_ok}

            for case in group["cases"]:
                case["deterministic_trials"] = []
                for zero_render in (False, True):
                    trials = []
                    for _ in range(ARGS.deterministic_repeats):
                        trial = await deterministic_trial(case["code"], case["key"],
                                                          case["expected_action"], zero_render)
                        trials.append(trial)
                        case["deterministic_trials"].append(trial)
                    suffix = "zero-render-then-tick" if zero_render else "direct-tick"
                    REPORT["checks"][f"{group_name}:{case['name']}:{suffix}:delivery"] = all(
                        trial["qualified"] for trial in trials)
                    REPORT["checks"][f"{group_name}:{case['name']}:{suffix}:actual-action-once"] = all(
                        trial["success"] for trial in trials)
            await snapshot(f"{group_name}-game.png")
            group["state"] = await game.locator("body").get_attribute("data-state")
            REPORT["checks"][f"{group_name}:game-ready"] = group["state"] == "ready"
            await ctx.close()
        await browser.close()


try:
    if ARGS.adjudicate_existing:
        raw_report = ARGS.adjudicate_existing.resolve()
        REPORT = json.loads(raw_report.read_text())
        REPORT["adjudication"] = {
            "raw_report": str(raw_report), "raw_report_sha256": sha(raw_report),
            "original_final_contract_pass": REPORT.get("final_contract_pass"),
            "original_failed_checks": [name for name, passed in REPORT["checks"].items() if not passed],
            "browser_rerun": False,
            "reason": "Original test incorrectly required slash audio on first tick, before its designed windup.",
        }
    else:
        asyncio.run(main())
    adjudicate_recorded(REPORT)
except BaseException as error:
    REPORT["exception"] = repr(error)
    raise
finally:
    REPORT["checks"]["no-runtime-errors"] = not REPORT["errors"]
    REPORT["checks"]["no-run-exception"] = not REPORT.get("exception")
    REPORT["final_contract_pass"] = bool(REPORT["groups"]) and all(REPORT["checks"].values())
    evidence = []
    for group in REPORT["groups"]:
        if group["name"] not in ("firmware", "software"):
            continue
        for case in group["cases"]:
            if case["name"] not in ("n-slash", "e-dash") or "held" not in case:
                continue
            qualified = [trial for trial in case.get("deterministic_trials", []) if trial["qualified"]]
            evidence.append({"group": group["name"], "case": case["name"],
                "held_works": case["held"]["success"], "qualified_deterministic_pairs": len(qualified),
                "missed_qualified_pairs": sum(not trial["success"] for trial in qualified),
                "missed_after_zero_tick_render": sum(not trial["success"] for trial in qualified
                    if trial["kind"] == "zero-render-then-17ms")})
    REPORT["baseline_hypothesis"] = evidence
    REPORT["baseline_reproduced"] = bool(evidence) and all(
        item["held_works"] and item["qualified_deterministic_pairs"] >= 2 * ARGS.deterministic_repeats
        and item["missed_after_zero_tick_render"] > 0 for item in evidence)
    (OUT / "report.json").write_text(json.dumps(REPORT, indent=2) + "\n")
    print(json.dumps({"out": str(OUT), "expect": ARGS.expect,
        "final_contract_pass": REPORT["final_contract_pass"],
        "baseline_reproduced": REPORT["baseline_reproduced"],
        "baseline_hypothesis": evidence,
        "failed_checks": [name for name, passed in REPORT["checks"].items() if not passed],
        "exception": REPORT.get("exception")}, indent=2))

assert (REPORT["baseline_reproduced"] if ARGS.expect == "baseline"
        else REPORT["final_contract_pass"]), "See report.json for measured action evidence"
