"""Exercise the local browser build inside an X-style cross-origin sandbox.

Requires web/glintblade.wasm and a Playwright Chromium installation.
uv run --with playwright python tools/pixellab/test_web_player.py --out out/web-check-new
The isolated test intercepts local files at the final HTTPS URLs; it publishes nothing.
PNG readbacks need direct visual review in addition to the input/state assertions.
"""
import asyncio, json, pathlib, mimetypes, time, base64, argparse
from playwright.async_api import async_playwright

ROOT = pathlib.Path(__file__).resolve().parents[2]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--out', type=pathlib.Path, required=True)
OUT = parser.parse_args().out.resolve()
OUT.mkdir(parents=True, exist_ok=False)
BASE = 'https://pomykalakyle.github.io/glintblade/'
report = {'checks': {}, 'console': [], 'errors': []}

async def read_canvas(game, name):
    data=await game.evaluate("() => new Promise(resolve=>requestAnimationFrame(()=>{wasm_exports.frame();resolve(canvas.toDataURL('image/png'))}))")
    (OUT/name).write_bytes(base64.b64decode(data.split(',')[1]))

async def main():
    async with async_playwright() as pw:
        browser = await pw.chromium.launch(headless=True, args=['--no-sandbox', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'])

        async def context(mobile=False, failure=False):
            ctx = await browser.new_context(viewport={'width': 390 if mobile else 1000, 'height': 700 if mobile else 760}, has_touch=mobile, is_mobile=mobile, device_scale_factor=1)
            async def route(req):
                url = req.request.url
                if url.startswith(BASE):
                    name = url[len(BASE):].split('?')[0] or 'index.html'
                    path = ROOT / 'web' / name
                    if failure and name == 'glintblade.wasm':
                        await req.fulfill(status=503, body='Test failure')
                    elif not path.is_file():
                        await req.fulfill(status=404)
                    else:
                        await req.fulfill(status=200, body=path.read_bytes(), content_type='application/wasm' if name.endswith('.wasm') else mimetypes.guess_type(str(path))[0] or 'application/octet-stream')
                elif url.startswith('https://x.com/glintblade-embed-test'):
                    await req.fulfill(content_type='text/html', body='<meta name="viewport" content="width=device-width, initial-scale=1"><style>body{margin:0;background:#10131a}iframe{border:0;display:block;margin:10px auto}</style><iframe id="game" width="' + str(390 if mobile else 480) + '" height="360" sandbox="allow-popups allow-same-origin allow-scripts allow-presentation" allow="autoplay; fullscreen; web-share" src="' + BASE + '?autoplay=1&auto_play=true"></iframe>')
                else:
                    await req.abort()
            await ctx.route('**/*', route)
            page = await ctx.new_page()
            page.on('console', lambda msg: report['console'].append({'type': msg.type, 'text': msg.text}))
            page.on('pageerror', lambda e: report['errors'].append(str(e)))
            await page.goto('https://x.com/glintblade-embed-test')
            frame = page.frame_locator('#game')
            await frame.locator('#play').wait_for()
            return ctx, page, frame

        ctx, page, frame = await context()
        await page.wait_for_timeout(500)
        report['checks']['autoplay_ignored'] = await frame.locator('body').get_attribute('data-state') == 'idle'
        started = time.monotonic()
        await frame.locator('#play').click()
        await frame.locator('body[data-state="ready"]').wait_for(timeout=120000)
        report['startup_seconds'] = round(time.monotonic() - started, 3)
        game = page.frames[1]
        report['runtime'] = await game.evaluate('({version:wasm_exports.crate_version(),canvas:[canvas.width,canvas.height],body:[innerWidth,innerHeight],focus:document.activeElement.id})')
        await game.evaluate('''() => {
            const original=wasm_exports;window.inputLog=[];window.frameTimes=[];
            wasm_exports={...original,key_down:(...a)=>{inputLog.push(['down',...a]);return original.key_down(...a)},key_up:(...a)=>{inputLog.push(['up',...a]);return original.key_up(...a)},frame:(...a)=>{const t=performance.now();const r=original.frame(...a);frameTimes.push(performance.now()-t);return r}};
        }''')
        await page.wait_for_timeout(2200)
        await read_canvas(game, 'desktop-title.png')
        await page.keyboard.press('Enter', delay=100)
        await page.wait_for_timeout(5000)
        await read_canvas(game, 'desktop-bloom-start.png')
        await page.keyboard.down('KeyD')
        await page.wait_for_timeout(500)
        await page.keyboard.up('KeyD')
        await page.keyboard.press('KeyJ', delay=100)
        await page.keyboard.press('KeyK', delay=100)
        await page.wait_for_timeout(700)
        await read_canvas(game, 'desktop-bloom-move.png')
        await page.keyboard.press('KeyF', delay=100)
        await page.wait_for_timeout(300)
        report['checks']['fullscreen_request_not_fatal'] = await frame.locator('body').get_attribute('data-state') == 'ready'
        await game.evaluate("() => {canvas.requestFullscreen=()=>Promise.reject(new DOMException('Denied for test','NotAllowedError'));importObject.env.sapp_set_fullscreen(true)}")
        await page.wait_for_timeout(150)
        report['checks']['denied_fullscreen_not_fatal'] = await frame.locator('body').get_attribute('data-state') == 'ready'
        report['keyboard_events'] = await game.evaluate('inputLog')
        report['checks']['wasd_j_k_delivered'] = await game.evaluate("[68,74,75].every(k=>inputLog.some(e=>e[0]==='down'&&e[1]===k))")
        report['frame_cpu_ms'] = await game.evaluate('({count:frameTimes.length,mean:frameTimes.reduce((a,b)=>a+b,0)/frameTimes.length,max:Math.max(...frameTimes)})')
        await ctx.close()

        ctx, page, frame = await context(mobile=True)
        await frame.locator('#play').tap()
        await frame.locator('body[data-state="ready"]').wait_for(timeout=120000)
        game = page.frames[1]
        await page.wait_for_timeout(2200)
        report['checks']['touch_controls_visible'] = await frame.locator('#touch').is_visible()
        await page.screenshot(path=str(OUT / 'mobile-title.png'))
        await frame.locator('[data-code="Enter"]').tap()
        await page.wait_for_timeout(5000)
        await game.evaluate('''() => {const original=wasm_exports;window.inputLog=[];wasm_exports={...original,key_down:(...a)=>{inputLog.push(['down',...a]);return original.key_down(...a)},key_up:(...a)=>{inputLog.push(['up',...a]);return original.key_up(...a)}}}''')
        button = await frame.locator('[data-code="ArrowRight"]').bounding_box()
        await page.mouse.move(button['x'] + 15, button['y'] + 15)
        await page.mouse.down()
        await page.wait_for_timeout(300)
        await page.keyboard.down('ArrowRight')
        await page.keyboard.up('ArrowRight')
        held = await game.evaluate('inputLog')
        report['checks']['physical_up_preserves_touch_hold'] = not any(e[0] == 'up' and e[1] == 262 for e in held)
        await page.mouse.up()
        await page.wait_for_timeout(100)
        report['checks']['touch_releases_key'] = await game.evaluate("inputLog.some(e=>e[0]==='up'&&e[1]===262)")
        await frame.locator('[data-code="KeyX"]').tap()
        await frame.locator('[data-code="KeyC"]').tap()
        report['checks']['touch_attack_dash_delivered'] = await game.evaluate("[88,67].every(k=>inputLog.some(e=>e[0]==='down'&&e[1]===k))")
        await read_canvas(game, 'mobile-bloom-canvas.png')
        await page.screenshot(path=str(OUT / 'mobile-bloom.png'))
        await page.mouse.move(button['x'] + 15, button['y'] + 15)
        await page.mouse.down()
        await game.evaluate("window.dispatchEvent(new Event('blur'))")
        await page.mouse.up()
        report['checks']['blur_releases_touch'] = await game.evaluate("inputLog.filter(e=>e[0]==='up'&&e[1]===262).length>=2")
        await ctx.close()

        ctx, page, frame = await context(failure=True)
        await frame.locator('#play').click()
        await frame.locator('body[data-state="error"]').wait_for(timeout=15000)
        report['checks']['download_error_has_retry'] = await frame.locator('#play').inner_text() == 'Try again'
        report['checks']['download_error_has_fullpage'] = await frame.get_by_text('Open full page ↗').is_visible()
        await ctx.close()
        await browser.close()

try:
    asyncio.run(main())
except Exception as e:
    report['test_error'] = repr(e)
    raise
finally:
    (OUT / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({k: v for k, v in report.items() if k != 'console'}, indent=2))
assert all(report['checks'].values()), report['checks']
