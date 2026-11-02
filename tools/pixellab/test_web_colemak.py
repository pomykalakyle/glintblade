"""Verify firmware/software Colemak in the local browser build using trusted CDP events.

uv run --with playwright python tools/pixellab/test_web_colemak.py --out out/colemak-check-new
Requires web/glintblade.wasm and installed Playwright Chromium. No network publishing.
Review the saved action PNGs as well as the normalized key-event assertions.
"""
import asyncio, base64, json, mimetypes, pathlib, argparse
from playwright.async_api import async_playwright

ROOT=pathlib.Path(__file__).resolve().parents[2]
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--out',type=pathlib.Path,required=True)
OUT=parser.parse_args().out.resolve()
OUT.mkdir(parents=True,exist_ok=False)
BASE='https://pomykalakyle.github.io/glintblade/'
report={'checks':{},'errors':[]}

async def main():
    async with async_playwright() as pw:
        browser=await pw.chromium.launch(headless=True,args=['--no-sandbox','--use-angle=swiftshader','--enable-unsafe-swiftshader'])
        ctx=await browser.new_context(viewport={'width':768,'height':520})
        async def route(r):
            if r.request.url.startswith(BASE):
                name=r.request.url[len(BASE):].split('?')[0] or 'index.html';p=ROOT/'web'/name
                await r.fulfill(body=p.read_bytes(),content_type='application/wasm' if name.endswith('.wasm') else mimetypes.guess_type(str(p))[0])
            elif r.request.url=='https://x.com/glint-colemak-test':
                await r.fulfill(content_type='text/html',body='<iframe width="480" height="360" sandbox="allow-popups allow-same-origin allow-scripts allow-presentation" allow="autoplay; fullscreen" src="'+BASE+'"></iframe>')
            else:await r.abort()
        await ctx.route('**/*',route)
        page=await ctx.new_page();page.on('pageerror',lambda e:report['errors'].append(str(e)))
        await page.goto('https://x.com/glint-colemak-test')
        f=page.frame_locator('iframe');await f.locator('#play').click()
        await f.locator('body[data-state="ready"]').wait_for(timeout=120000)
        frame=page.frames[1];cdp=await ctx.new_cdp_session(page)
        await page.wait_for_timeout(2200)
        await frame.evaluate('''() => {const original=wasm_exports;window.keylog=[];window.trustedlog=[];canvas.addEventListener('keydown',e=>{if(e.isTrusted)trustedlog.push([e.code,e.key])},true);wasm_exports={...original,key_down:(...a)=>{keylog.push(['down',...a]);return original.key_down(...a)},key_up:(...a)=>{keylog.push(['up',...a]);return original.key_up(...a)}}}''')
        async def dispatch(kind,code,key,**extra):
            await cdp.send('Input.dispatchKeyEvent',{'type':kind,'code':code,'key':key,**extra})
        async def press(code,key,delay=100,**extra):
            await dispatch('keyDown',code,key,**extra);await page.wait_for_timeout(delay);await dispatch('keyUp',code,key,**extra)
        async def snap(name):
            data=await frame.evaluate("() => new Promise(r=>requestAnimationFrame(()=>{wasm_exports.frame();r(canvas.toDataURL('image/png'))}))")
            (OUT/name).write_bytes(base64.b64decode(data.split(',')[1]))
        # Fresh loads remain WASD. N/E must work without selecting Colemak first.
        await snap('fresh-wasd-title.png')
        await press('Enter','Enter');await page.wait_for_timeout(5400)
        for label,code,typed,want in [('fresh-software-slash','KeyJ','n',78),('fresh-firmware-slash','KeyN','n',78),('fresh-software-dash','KeyK','e',69),('fresh-firmware-dash','KeyE','e',69)]:
            await frame.evaluate('keylog.length=0')
            await dispatch('keyDown',code,typed);await page.wait_for_timeout(65)
            await snap(label+'.png')
            await dispatch('keyUp',code,typed)
            log=await frame.evaluate('keylog')
            report['checks'][label]=any(e[0]=='down' and e[1]==want for e in log) and any(e[0]=='up' and e[1]==want for e in log)
            await page.wait_for_timeout(700)
        # The existing movement selector still works on the pause screen.
        await press('Escape','Escape');await press('Tab','Tab')
        await page.wait_for_timeout(200);await snap('colemak-pause.png')
        await press('Escape','Escape');await page.wait_for_timeout(200)
        # Actual trusted software-layout events: physical J/K type N/E.
        for label,code,typed,want in [('software-slash','KeyJ','n',78),('firmware-slash','KeyN','n',78),('software-dash','KeyK','e',69),('firmware-dash','KeyE','e',69)]:
            await frame.evaluate('keylog.length=0')
            await dispatch('keyDown',code,typed);await page.wait_for_timeout(65)
            await snap(label+'.png')
            await dispatch('keyUp',code,typed)
            log=await frame.evaluate('keylog')
            report['checks'][label]=any(e[0]=='down' and e[1]==want for e in log) and any(e[0]=='up' and e[1]==want for e in log)
            await page.wait_for_timeout(700)
        for label,code,typed,want in [('software-up','KeyW','w',87),('software-left','KeyA','a',65),('software-down','KeyS','r',82),('software-right','KeyD','s',83),('firmware-down','KeyR','r',82),('firmware-right','KeyS','s',83),('semicolon-pause','KeyP',';',59),('colon-pause','KeyP',':',59)]:
            await frame.evaluate('keylog.length=0');await press(code,typed)
            report['checks'][label]=await frame.evaluate('(wanted)=>keylog.some(e=>e[0]==="down"&&e[1]===wanted)',want)
            await page.wait_for_timeout(200)
        # A different key/Shift value at release must release the original N.
        await frame.evaluate('keylog.length=0')
        await dispatch('keyDown','KeyJ','N',modifiers=8);await dispatch('keyUp','KeyJ','j')
        report['checks']['changed-keyup-releases-original']=await frame.evaluate("keylog.some(e=>e[0]==='up'&&e[1]===78)&&!keylog.some(e=>e[0]==='up'&&e[1]===74)")
        # Two physical keys can name the same logical letter. Release once.
        await frame.evaluate('keylog.length=0')
        await dispatch('keyDown','KeyJ','n');await dispatch('keyDown','KeyN','n');await dispatch('keyUp','KeyJ','n')
        report['checks']['shared-physical-hold']=await frame.evaluate("!keylog.some(e=>e[0]==='up'&&e[1]===78)")
        await dispatch('keyUp','KeyN','n')
        report['checks']['last-physical-release']=await frame.evaluate("keylog.filter(e=>e[0]==='up'&&e[1]===78).length===1")
        await frame.evaluate('keylog.length=0')
        await dispatch('keyDown','KeyJ','n');await frame.evaluate("window.dispatchEvent(new Event('blur'))")
        report['checks']['blur-clears-letter']=await frame.evaluate("keylog.some(e=>e[0]==='up'&&e[1]===78)")
        await dispatch('keyUp','KeyJ','n')
        # Escape/arrows remain untouched; no remapping to printed punctuation.
        await press('Escape','Escape');await press('ArrowRight','ArrowRight')
        report['checks']['arrow-preserved']=await frame.evaluate("keylog.some(e=>e[0]==='down'&&e[1]===262)")
        report['checks']['no-runtime-error']=not report['errors']
        report['checks']['game-ready']=await f.locator('body').get_attribute('data-state')=='ready'
        await ctx.close();await browser.close()
try:
    asyncio.run(main())
finally:
    (OUT/'report.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
assert all(report['checks'].values()),report
