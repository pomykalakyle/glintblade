"""Verify selected browser layouts with trusted CDP keys and actual action audio.

uv run --with playwright python tools/pixellab/test_web_colemak.py --out out/colemak-check-new
Requires a web build assembled with prepare_web_build.py and Playwright Chromium.
Use --public --expected-build COMMIT to verify the deployed Pages build instead.
No publishing.
Normalization is checked separately: forwarding N/E is not proof of an action.
WASD uses J/K, Colemak uses N/E; each layout must reject the other's letters.
Use --public --expected-build COMMIT to check the deployed Pages resources;
only the outer X-style test wrapper is local in that mode.
"""
import argparse
import asyncio
import base64
import json
import mimetypes
import pathlib
import urllib.parse
import wave

from playwright.async_api import async_playwright

ROOT = pathlib.Path(__file__).resolve().parents[2]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--out', type=pathlib.Path, required=True)
parser.add_argument('--web-dir', type=pathlib.Path, default=ROOT / 'web')
parser.add_argument('--public', action='store_true', help='Load actual GitHub Pages resources, with no local asset interception')
parser.add_argument('--expected-build', help='Require this exact published build label/commit')
args = parser.parse_args()
OUT = args.out.resolve()
WEB = args.web_dir.resolve()
OUT.mkdir(parents=True, exist_ok=False)
BASE = 'https://pomykalakyle.github.io/glintblade/'
GAME_URL = BASE + ('?v=controls-r13' if args.public else '')
STORAGE = 'glintblade.keyLayout'

# slash_3 has the same duration as beetle_charge. Do not accept that ambiguous
# sample as proof. The gap between probes resets the combo to slash_1 instead.
ACTION_NAMES = {'slash': ['slash_1', 'slash_2'], 'dash': ['dash', 'dash_2']}
WAVS = {}
for path in (ROOT / 'assets/sfx').glob('*.wav'):
    with wave.open(str(path), 'rb') as wav:
        WAVS[path.stem] = wav.getnframes() / wav.getframerate()
report = {
    'checks': {}, 'errors': [], 'actions': [], 'layouts': [], 'resources': [],
    'public': args.public, 'url': GAME_URL, 'expected_build': args.expected_build,
    'action_wavs': {name: WAVS[name] for names in ACTION_NAMES.values() for name in names},
    'action_duration_tolerance_seconds': .0001,
    'scope': ('Actual public GitHub Pages resources' if args.public else 'Actual local WASM')
             + ' in the X sandbox; trusted CDP keyboard events. '
             'Audio source starts are action evidence. Normalized events alone '
             'are not action evidence. No physical keyboard or native X app tested.',
}


def classify(sounds):
    return sorted({action for sound in sounds if not sound.get('loop')
                   and sound.get('seconds') is not None
                   for action, names in ACTION_NAMES.items()
                   if any(abs(sound['seconds'] - WAVS[name]) < .0001 for name in names)})


async def main():
    async with async_playwright() as pw:
        browser = await pw.chromium.launch(headless=True, args=[
            '--no-sandbox', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'])
        ctx = await browser.new_context(viewport={'width': 768, 'height': 520})
        if args.public:
            # A WASM response can exceed Chromium's inspector response cache.
            # Hash a clone of the response the game actually consumes instead.
            await ctx.add_init_script(r'''
              window.publicWasmProofs = {};
              const originalFetch = window.fetch;
              window.fetch = async function(...args) {
                const response = await originalFetch.apply(this,args);
                if(response.url.startsWith('https://pomykalakyle.github.io/glintblade/') && /\.wasm(?:\?|$)/.test(response.url)) {
                  const clone = response.clone();
                  publicWasmProofs[response.url] = clone.arrayBuffer().then(async bytes => ({
                    bytes:bytes.byteLength,
                    sha256:[...new Uint8Array(await crypto.subtle.digest('SHA-256',bytes))].map(x=>x.toString(16).padStart(2,'0')).join('')
                  }));
                }
                return response;
              };
            ''')
        await ctx.add_init_script('''
          window.actionSounds=[];window.trustedlog=[];
          const originalStart=AudioBufferSourceNode.prototype.start;
          AudioBufferSourceNode.prototype.start=function(...args){
            actionSounds.push({seconds:this.buffer?.duration,loop:this.loop,
              sampleRate:this.buffer?.sampleRate,at:performance.now()});
            return originalStart.apply(this,args);
          };
          window.addEventListener('keydown',event=>{
            if(event.isTrusted&&event.target?.id==='glcanvas')
              trustedlog.push([event.code,event.key]);
          },true);
        ''')

        async def route(request):
            url = request.request.url
            if url.startswith(BASE):
                if args.public:
                    await request.continue_()
                    return
                name = url[len(BASE):].split('?')[0] or 'index.html'
                path = (WEB / name).resolve()
                if not path.is_relative_to(WEB) or not path.is_file():
                    await request.fulfill(status=404)
                    return
                await request.fulfill(body=path.read_bytes(), content_type=(
                    'application/wasm' if name.endswith('.wasm')
                    else mimetypes.guess_type(str(path))[0] or 'application/octet-stream'))
            elif url == 'https://x.com/glint-colemak-test':
                await request.fulfill(content_type='text/html', body=(
                    '<iframe width="480" height="298" '
                    'sandbox="allow-popups allow-same-origin allow-scripts allow-presentation" '
                    'allow="autoplay; fullscreen" src="' + GAME_URL + '"></iframe>'))
            else:
                await request.abort()

        await ctx.route('**/*', route)
        page = await ctx.new_page()
        responses = []
        page.on('pageerror', lambda error: report['errors'].append(str(error)))
        page.on('response', lambda response: responses.append(response) if response.url.startswith(BASE) else None)
        await page.goto('https://x.com/glint-colemak-test')
        locator = page.frame_locator('iframe')
        await locator.locator('#play').wait_for()
        frame = page.frames[1]
        report['checks']['new_visitor_selector_wasd'] = await locator.locator('#layout').input_value() == '0'
        report['checks']['new_visitor_no_preference'] = await frame.evaluate(
            '(key)=>localStorage.getItem(key)', STORAGE) is None
        cdp = await ctx.new_cdp_session(page)

        async def dispatch(kind, code, key, **extra):
            await cdp.send('Input.dispatchKeyEvent', {'type': kind, 'code': code, 'key': key, **extra})

        async def press(code, key, delay=100, **extra):
            await dispatch('keyDown', code, key, **extra)
            await page.wait_for_timeout(delay)
            await dispatch('keyUp', code, key, **extra)

        async def snap(name):
            data = await frame.evaluate("() => new Promise(resolve=>requestAnimationFrame(()=>{wasm_exports.frame();resolve(canvas.toDataURL('image/png'))}))")
            (OUT / name).write_bytes(base64.b64decode(data.split(',')[1]))

        async def check_public_resources(label):
            if not args.public:
                return
            manifests = [response for response in responses
                         if urllib.parse.urlsplit(response.url).path.endswith('/build-info.json')]
            report['checks'][label + '-public-manifest-requested'] = bool(manifests)
            if not manifests:
                return
            manifest_response = manifests[-1]
            manifest_bytes = await manifest_response.body()
            manifest = json.loads(manifest_bytes)
            digest = manifest.get('sha256', '')
            hashed_name = 'glintblade.' + digest[:16] + '.wasm'
            report['checks'][label + '-public-manifest-valid'] = (
                manifest_response.status == 200 and manifest.get('schema') == 1
                and manifest.get('controls_revision') == 13
                and len(digest) == 64 and all(char in '0123456789abcdef' for char in digest)
                and manifest.get('wasm') == hashed_name)
            actual_build = await frame.evaluate('document.body.dataset.build')
            report['checks'][label + '-public-build-agrees'] = manifest.get('build') == actual_build
            if args.expected_build:
                report['checks'][label + '-expected-public-build'] = manifest.get('build') == args.expected_build
            wasm_url = BASE + hashed_name
            wasm_responses = [response for response in responses if response.url == wasm_url]
            report['checks'][label + '-public-immutable-wasm-requested'] = bool(wasm_responses)
            evidence = {'label': label, 'manifest_url': manifest_response.url,
                        'manifest_status': manifest_response.status, 'manifest': manifest}
            if wasm_responses:
                wasm_response = wasm_responses[-1]
                proof = await frame.evaluate('(url)=>window.publicWasmProofs[url]', wasm_response.url)
                actual_hash = proof['sha256']
                evidence.update({'wasm_url': wasm_response.url, 'wasm_status': wasm_response.status,
                                 'wasm_bytes': proof['bytes'], 'wasm_sha256': actual_hash,
                                 'wasm_hash_source': 'SHA256 of clone of actual browser fetch Response',
                                 'wasm_content_type': wasm_response.headers.get('content-type')})
                report['checks'][label + '-public-wasm-hash-matches'] = wasm_response.status == 200 and actual_hash == digest
                report['checks'][label + '-public-wasm-mime'] = 'application/wasm' in wasm_response.headers.get('content-type', '')
            report['resources'].append(evidence)

        async def start_game():
            nonlocal frame
            await locator.locator('#play').click()
            await locator.locator('body[data-state="ready"]').wait_for(timeout=120000)
            frame = page.frames[1]
            await page.wait_for_timeout(2200)
            await frame.evaluate('''() => {
              const original=wasm_exports;window.keylog=[];
              wasm_exports={...original,
                key_down:(...args)=>{keylog.push(['down',...args]);return original.key_down(...args)},
                key_up:(...args)=>{keylog.push(['up',...args]);return original.key_up(...args)}};
            }''')
            await check_public_resources('initial' if not report['resources'] else 'reloaded')

        async def check_layout(label, expected, preference):
            await frame.wait_for_function('(wanted)=>wasm_exports.glint_key_layout()===wanted&&document.querySelector("#layout").value===String(wanted)', arg=expected)
            actual = await frame.evaluate('''key=>({engine:wasm_exports.glint_key_layout(),
              selector:document.querySelector('#layout').value,
              stored:localStorage.getItem(key),focus:document.activeElement.id,
              instructions:document.querySelector('#instructions').textContent})''', STORAGE)
            report['layouts'].append({'label': label, **actual})
            report['checks'][label + '-engine-and-selector'] = actual['engine'] == expected and actual['selector'] == str(expected)
            report['checks'][label + '-saved-preference'] = actual['stored'] == preference
            report['checks'][label + '-canvas-focus'] = actual['focus'] == 'glcanvas'
            slash, dash, movement = ('N', 'E', 'WARS') if expected else ('J', 'K', 'WASD')
            report['checks'][label + '-instructions-match'] = (
                movement + ' / arrows to move' in actual['instructions']
                and slash + ' to slash' in actual['instructions']
                and dash + ' / Space to dash' in actual['instructions'])

        async def action(label, code, typed, expected):
            # Reset combo and dash cooldown before collecting the actual sound.
            await page.wait_for_timeout(1000)
            await locator.locator('#glcanvas').focus()
            await frame.evaluate('actionSounds.length=0;keylog.length=0;trustedlog.length=0')
            await dispatch('keyDown', code, typed)
            await page.wait_for_timeout(65)
            await snap(label + '.png')
            await dispatch('keyUp', code, typed)
            await page.wait_for_timeout(180)
            evidence = await frame.evaluate('({sounds:actionSounds,normalized:keylog,trusted:trustedlog})')
            actions = classify(evidence['sounds'])
            report['actions'].append({'label': label, 'code': code, 'typed': typed,
                                      'expected': expected, 'actual': actions, **evidence})
            report['checks'][label + '-actual-action'] = actions == ([expected] if expected else [])
            wanted = ord(typed.upper())
            report['checks'][label + '-normalized-letter'] = (
                any(event[0] == 'down' and event[1] == wanted for event in evidence['normalized'])
                and any(event[0] == 'up' and event[1] == wanted for event in evidence['normalized']))
            report['checks'][label + '-trusted-key'] = [code, typed] in evidence['trusted']

        await start_game()
        await check_layout('fresh-wasd', 0, None)
        report['revision'] = await frame.evaluate('({engine:wasm_exports.glint_controls_revision(),page:document.body.dataset.controlsRevision,build:document.body.dataset.build})')
        report['checks']['controls-revision-13'] = report['revision']['engine'] == 13 and report['revision']['page'] == '13'
        if args.expected_build:
            report['checks']['expected-page-build'] = report['revision']['build'] == args.expected_build
        await snap('fresh-wasd-title.png')
        await press('Enter', 'Enter')
        await page.wait_for_timeout(5400)
        for label, code, typed, expected in [
            ('wasd-j-slash', 'KeyJ', 'j', 'slash'), ('wasd-k-dash', 'KeyK', 'k', 'dash'),
            ('wasd-n-disabled', 'KeyN', 'n', None), ('wasd-e-disabled', 'KeyE', 'e', None),
            ('wasd-software-n-disabled', 'KeyJ', 'n', None),
            ('wasd-software-e-disabled', 'KeyK', 'e', None),
        ]:
            await action(label, code, typed, expected)

        await locator.locator('#layout').select_option('1')
        await check_layout('selected-colemak', 1, '1')
        await snap('selected-colemak.png')
        for label, code, typed, expected in [
            ('colemak-firmware-n-slash', 'KeyN', 'n', 'slash'),
            ('colemak-firmware-e-dash', 'KeyE', 'e', 'dash'),
            ('colemak-software-n-slash', 'KeyJ', 'n', 'slash'),
            ('colemak-software-e-dash', 'KeyK', 'e', 'dash'),
            ('colemak-j-disabled', 'KeyJ', 'j', None), ('colemak-k-disabled', 'KeyK', 'k', None),
        ]:
            await action(label, code, typed, expected)

        # Reload the same origin: an explicit selection must persist, and the
        # actual engine state must agree with the visible selector after startup.
        await page.reload()
        await locator.locator('#play').wait_for()
        report['checks']['persisted-selector-before-start'] = await locator.locator('#layout').input_value() == '1'
        await start_game()
        await check_layout('reloaded-colemak', 1, '1')
        await press('Tab', 'Tab')
        await check_layout('tab-to-wasd', 0, '0')
        await press('Tab', 'Tab')
        await check_layout('tab-to-colemak', 1, '1')
        await snap('colemak-title.png')
        await press('Enter', 'Enter')
        await page.wait_for_timeout(5400)
        await action('reloaded-colemak-n-slash', 'KeyN', 'n', 'slash')
        await action('reloaded-colemak-e-dash', 'KeyE', 'e', 'dash')

        # Preserve the independent normalizer, overlapping-key and blur checks.
        # These prove event handling, separately from the action evidence above.
        for label, code, typed, wanted in [
            ('software-up', 'KeyW', 'w', 87), ('software-left', 'KeyA', 'a', 65),
            ('software-down', 'KeyS', 'r', 82), ('software-right', 'KeyD', 's', 83),
            ('firmware-down', 'KeyR', 'r', 82), ('firmware-right', 'KeyS', 's', 83),
            ('semicolon-pause', 'KeyP', ';', 59), ('colon-pause', 'KeyP', ':', 59),
        ]:
            await frame.evaluate('keylog.length=0')
            await press(code, typed)
            report['checks'][label] = await frame.evaluate('(wanted)=>keylog.some(event=>event[0]==="down"&&event[1]===wanted)', wanted)
            await page.wait_for_timeout(200)
        await frame.evaluate('keylog.length=0')
        await dispatch('keyDown', 'KeyJ', 'N', modifiers=8)
        await dispatch('keyUp', 'KeyJ', 'j')
        report['checks']['changed-keyup-releases-original'] = await frame.evaluate("keylog.some(event=>event[0]==='up'&&event[1]===78)&&!keylog.some(event=>event[0]==='up'&&event[1]===74)")
        await frame.evaluate('keylog.length=0')
        await dispatch('keyDown', 'KeyJ', 'n')
        await dispatch('keyDown', 'KeyN', 'n')
        await dispatch('keyUp', 'KeyJ', 'n')
        report['checks']['shared-physical-hold'] = await frame.evaluate("!keylog.some(event=>event[0]==='up'&&event[1]===78)")
        await dispatch('keyUp', 'KeyN', 'n')
        report['checks']['last-physical-release'] = await frame.evaluate("keylog.filter(event=>event[0]==='up'&&event[1]===78).length===1")
        await frame.evaluate('keylog.length=0')
        await dispatch('keyDown', 'KeyJ', 'n')
        await frame.evaluate("window.dispatchEvent(new Event('blur'))")
        report['checks']['blur-clears-letter'] = await frame.evaluate("keylog.some(event=>event[0]==='up'&&event[1]===78)")
        await dispatch('keyUp', 'KeyJ', 'n')
        await press('Escape', 'Escape')
        await press('ArrowRight', 'ArrowRight')
        report['checks']['arrow-preserved'] = await frame.evaluate("keylog.some(event=>event[0]==='down'&&event[1]===262)")
        report['checks']['no-runtime-error'] = not report['errors']
        report['checks']['game-ready'] = await locator.locator('body').get_attribute('data-state') == 'ready'
        await ctx.close()
        await browser.close()


try:
    asyncio.run(main())
finally:
    (OUT / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))
assert all(report['checks'].values()), report
