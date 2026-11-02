"""Exercise the local browser build inside an X-style cross-origin sandbox.

Requires a web build assembled with prepare_web_build.py and Playwright Chromium.
uv run --with playwright python tools/pixellab/test_web_player.py --out out/web-check-new
The isolated test intercepts local files at the final HTTPS URLs; it publishes nothing.
PNG readbacks need direct visual review in addition to the input/state assertions.
Each fresh output directory also contains geometry.json and geometry-matrix.png.
Use --web-dir for a preserved baseline web bundle, then compare the two matrices.
"""
import asyncio, json, pathlib, mimetypes, time, base64, argparse, html
from playwright.async_api import async_playwright

ROOT = pathlib.Path(__file__).resolve().parents[2]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--out', type=pathlib.Path, required=True)
parser.add_argument('--web-dir', type=pathlib.Path, default=ROOT / 'web')
args = parser.parse_args()
OUT = args.out.resolve()
WEB = args.web_dir.resolve()
OUT.mkdir(parents=True, exist_ok=False)
BASE = 'https://pomykalakyle.github.io/glintblade/'
report = {'checks': {}, 'console': [], 'errors': [], 'geometry': []}
geometry_cards = []

async def read_canvas(game, name):
    data=await game.evaluate("() => new Promise(resolve=>requestAnimationFrame(()=>{wasm_exports.frame();resolve(canvas.toDataURL('image/png'))}))")
    (OUT/name).write_bytes(base64.b64decode(data.split(',')[1]))

async def resize_fixture(page, game, width, height):
    """Resize the same live game, including its real ResizeObserver/resize path."""
    await page.set_viewport_size({'width': max(width + 20, 430), 'height': max(height + 20, 700)})
    await page.locator('#game').evaluate('(el, size) => {el.width=size[0];el.height=size[1]}', [width, height])
    await page.wait_for_timeout(350)
    await game.locator('#glcanvas').focus()

async def check_geometry(page, game, name, *, touch_expected=False):
    """Measure GPU output, independently of the renderer's sizing calculation.

    The matte is the frontend's clear color. A well-fit, uncropped scene retains
    its aspect, is centered, and reaches at least one opposite pair of edges.
    Stable authored HUD colors independently prove that both corners survived.
    """
    result = await game.evaluate(r'''() => new Promise(resolve => requestAnimationFrame(() => {
      wasm_exports.frame();
      const image=document.createElement('canvas');image.width=canvas.width;image.height=canvas.height;
      const ctx=image.getContext('2d',{willReadFrequently:true});ctx.drawImage(canvas,0,0);
      const width=image.width,height=image.height,p=ctx.getImageData(0,0,width,height).data;
      const columns=new Uint32Array(width),rows=new Uint32Array(height);
      for(let y=0;y<height;y++)for(let x=0;x<width;x++){
        const k=(y*width+x)*4;
        if(p[k+3]>250 && (Math.abs(p[k]-4)>2 || Math.abs(p[k+1]-3)>2 || Math.abs(p[k+2]-10)>2)){
          columns[x]++;rows[y]++;
        }
      }
      const xs=[],ys=[];
      for(let x=0;x<width;x++)if(columns[x]>=Math.max(2,height*.01))xs.push(x);
      for(let y=0;y<height;y++)if(rows[y]>=Math.max(2,width*.01))ys.push(y);
      const bbox=xs.length&&ys.length?[xs[0],ys[0],xs[xs.length-1]+1,ys[ys.length-1]+1]:null;
      const rect=el=>{const r=el.getBoundingClientRect();return{x:r.x,y:r.y,width:r.width,height:r.height}};
      const visible=el=>{
        if(document.fullscreenElement&&!document.fullscreenElement.contains(el))return false;
        const style=getComputedStyle(el),r=el.getBoundingClientRect();
        return !el.hidden&&style.display!=='none'&&style.visibility!=='hidden'&&r.width>0&&r.height>0;
      };
      const canvasRect=rect(canvas),footer=document.querySelector('footer');
      const controls=[...document.querySelectorAll('#touch button')].filter(visible).map(el=>({code:el.dataset.code,...rect(el)}));
      let scene=null,hearts=[],shard={purple:0,bone:0};
      if(bbox){
        const [x0,y0,x1,y1]=bbox,w=x1-x0,h=y1-y0;
        scene={x:canvasRect.x+x0*canvasRect.width/width,y:canvasRect.y+y0*canvasRect.height/height,
          width:w*canvasRect.width/width,height:h*canvasRect.height/height};
        // These native-grid coordinates identify actual authored HUD art, not
        // the scale helper: six rose health marks and the far-right prism/0.
        const count=(left,top,right,bottom,predicate)=>{
          let found=0;
          for(let y=Math.max(0,Math.floor(y0+top*h/216));y<Math.min(height,Math.ceil(y0+bottom*h/216));y++)
            for(let x=Math.max(0,Math.floor(x0+left*w/384));x<Math.min(width,Math.ceil(x0+right*w/384));x++){
              const k=(y*width+x)*4;if(predicate(p[k],p[k+1],p[k+2]))found++;
            }
          return found;
        };
        const rose=(r,g,b)=>r>110&&r>g+12&&Math.abs(g-b)<30;
        for(let i=0;i<6;i++)hearts.push(count(7+8*i,6,14+8*i,14,rose));
        shard.purple=count(352,4,380,19,(r,g,b)=>b>100&&b>r+5&&b>g+10);
        shard.bone=count(352,4,380,19,(r,g,b)=>r>190&&g>190&&b>160&&Math.abs(r-g)<18&&Math.abs(g-b)<35);
      }
      resolve({buffer:[width,height],bbox,canvas:canvasRect,scene,hearts,shard,
        footer:visible(footer)?rect(footer):null,controls,
        viewport:[innerWidth,innerHeight],dpr:devicePixelRatio,
        touchPadding:getComputedStyle(document.querySelector('#player')).paddingBottom,
        fullscreen:document.fullscreenElement===canvas,png:image.toDataURL('image/png')});
    }))''')
    folder = OUT / 'geometry'
    folder.mkdir(exist_ok=True)
    canvas_png = result.pop('png')
    (folder / f'{name}-canvas.png').write_bytes(base64.b64decode(canvas_png.split(',')[1]))
    if result['fullscreen']:
        shot = await page.screenshot(path=str(folder / f'{name}-page.png'))
    else:
        shot = await (await game.frame_element()).screenshot(path=str(folder / f'{name}-page.png'))

    def overlap(a, b):
        return max(0, min(a['x'] + a['width'], b['x'] + b['width']) - max(a['x'], b['x'])) * max(0, min(a['y'] + a['height'], b['y'] + b['height']) - max(a['y'], b['y']))

    checks = {'scene_present': result['bbox'] is not None}
    if result['bbox']:
        left, top, right, bottom = result['bbox']
        width, height = result['buffer']
        scene = result['scene']
        checks.update({
            'aspect_16_9': abs(scene['width'] * 9 - scene['height'] * 16) <= 40,
            'centered': abs(left - (width - right)) <= 2 and abs(top - (height - bottom)) <= 2,
            'opposite_edges_filled': min(max(left, width - right), max(top, height - bottom)) <= 2,
            'inside_viewport': scene['x'] >= -1 and scene['y'] >= -1 and scene['x'] + scene['width'] <= result['viewport'][0] + 1 and scene['y'] + scene['height'] <= result['viewport'][1] + 1,
            'footer_separate': result['footer'] is None or overlap(scene, result['footer']) < .25,
            'touch_separate': all(overlap(scene, control) < .25 for control in result['controls']),
            'touch_visible_as_expected': bool(result['controls']) == touch_expected,
            'six_health_marks_visible': len(result['hearts']) == 6 and all(n > 0 for n in result['hearts']),
            'right_shard_and_counter_visible': result['shard']['purple'] > 0 and result['shard']['bone'] > 0,
        })
    result.update({'name': name, 'checks': checks})
    report['geometry'].append(result)
    for check, passed in checks.items():
        report['checks'][f'geometry_{name}_{check}'] = passed
    geometry_cards.append({'name': name, 'image': 'data:image/png;base64,' + base64.b64encode(shot).decode(), 'result': result})
    (OUT / 'geometry.json').write_text(json.dumps(report['geometry'], indent=2) + '\n')

async def save_geometry_matrix(browser):
    """An inspectable contact sheet for each baseline/fixed run, not fake art."""
    if not geometry_cards:
        return
    cells = []
    for item in geometry_cards:
        data = item['result']
        failures = [name for name, passed in data['checks'].items() if not passed]
        summary = 'PASS' if not failures else 'FAIL: ' + ', '.join(failures)
        cells.append('<article><h2>' + html.escape(item['name']) + '</h2><p class="' + ('bad' if failures else 'good') + '">' + html.escape(summary) + '</p><p>' + html.escape(f"buffer {data['buffer']} · bbox {data['bbox']} · DPR {data['dpr']} · controls padding {data['touchPadding']}") + '</p><img src="' + item['image'] + '"></article>')
    markup = '<!doctype html><meta charset="utf-8"><style>body{margin:20px;background:#10151c;color:#d8dccf;font:13px monospace}main{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:16px}article{background:#19222c;padding:12px;min-width:0}h2{font-size:16px;margin:0 0 6px}p{overflow-wrap:anywhere;margin:5px 0}.good{color:#9de6b5}.bad{color:#ffaaa0}img{display:block;width:100%;max-height:650px;object-fit:contain;object-position:top;background:#080c11;image-rendering:pixelated}</style><h1>Actual browser geometry</h1><main>' + ''.join(cells) + '</main>'
    (OUT / 'geometry-matrix.html').write_text(markup)
    ctx = await browser.new_context(viewport={'width': 1080, 'height': 800}, device_scale_factor=1)
    page = await ctx.new_page()
    await page.set_content(markup)
    await page.locator('img').evaluate_all('(images)=>Promise.all(images.map(image=>image.decode()))')
    await page.screenshot(path=str(OUT / 'geometry-matrix.png'), full_page=True)
    await ctx.close()

async def main():
    async with async_playwright() as pw:
        browser = await pw.chromium.launch(headless=True, args=['--no-sandbox', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'])

        async def context(mobile=False, failure=False, dpr=1):
            ctx = await browser.new_context(viewport={'width': 390 if mobile else 1000, 'height': 700 if mobile else 760}, has_touch=mobile, is_mobile=mobile, device_scale_factor=dpr)
            async def route(req):
                url = req.request.url
                if url.startswith(BASE):
                    name = url[len(BASE):].split('?')[0] or 'index.html'
                    path = WEB / name
                    if failure and name.endswith('.wasm'):
                        await req.fulfill(status=503, body='Test failure')
                    elif not path.is_file():
                        await req.fulfill(status=404)
                    else:
                        await req.fulfill(status=200, body=path.read_bytes(), content_type='application/wasm' if name.endswith('.wasm') else mimetypes.guess_type(str(path))[0] or 'application/octet-stream')
                elif url.startswith('https://x.com/glintblade-embed-test'):
                    await req.fulfill(content_type='text/html', body='<meta name="viewport" content="width=device-width, initial-scale=1"><style>body{margin:0;background:#10131a}iframe{border:0;display:block;margin:10px auto}</style><iframe id="game" width="' + str(390 if mobile else 480) + '" height="360" sandbox="allow-popups allow-same-origin allow-scripts allow-presentation" allow="autoplay; fullscreen; web-share" allowfullscreen src="' + BASE + '?autoplay=1&auto_play=true"></iframe>')
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
        report['metadata'] = await game.evaluate("Object.fromEntries([...document.querySelectorAll('meta[name^=\"twitter:\"]')].map(element=>[element.name,element.content]))")
        report['checks']['fresh_card_480x298'] = report['metadata'].get('twitter:player:width') == '480' and report['metadata'].get('twitter:player:height') == '298'
        await game.evaluate('''() => {
            const original=wasm_exports;window.inputLog=[];window.frameTimes=[];
            wasm_exports={...original,key_down:(...a)=>{inputLog.push(['down',...a]);return original.key_down(...a)},key_up:(...a)=>{inputLog.push(['up',...a]);return original.key_up(...a)},frame:(...a)=>{const t=performance.now();const r=original.frame(...a);frameTimes.push(performance.now()-t);return r}};
        }''')
        await page.wait_for_timeout(2200)
        await read_canvas(game, 'desktop-title.png')
        await page.keyboard.press('Enter', delay=100)
        await page.wait_for_timeout(5000)
        await read_canvas(game, 'desktop-bloom-start.png')
        await check_geometry(page, game, 'desktop-cached-480x360')
        for name, width, height in [('desktop-fresh-480x298', 480, 298), ('desktop-cached-480x294', 480, 294), ('desktop-wide-1280x744', 1280, 744), ('desktop-tall-500x900', 500, 900)]:
            await resize_fixture(page, game, width, height)
            await check_geometry(page, game, name)
        await resize_fixture(page, game, 480, 360)
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
        report['checks']['fullscreen_actual_canvas'] = await game.evaluate('document.fullscreenElement===canvas')
        if report['checks']['fullscreen_actual_canvas']:
            await check_geometry(page, game, 'desktop-fullscreen')
            await game.evaluate('document.exitFullscreen()')
            await page.wait_for_timeout(350)
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
        for name, width, height in [('mobile-cached-proportional-390x239', 390, 239), ('mobile-fresh-390x298', 390, 298), ('mobile-cached-390x294', 390, 294), ('mobile-cached-390x360', 390, 360), ('mobile-portrait-390x700', 390, 700)]:
            await resize_fixture(page, game, width, height)
            await check_geometry(page, game, name, touch_expected=True)
        await resize_fixture(page, game, 390, 360)
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

        ctx, page, frame = await context(dpr=2)
        await frame.locator('#play').click()
        await frame.locator('body[data-state="ready"]').wait_for(timeout=120000)
        game = page.frames[1]
        await page.wait_for_timeout(2200)
        await page.keyboard.press('Enter', delay=100)
        await page.wait_for_timeout(5000)
        for name, width, height in [('hidpi-cached-480x360', 480, 360), ('hidpi-fresh-480x298', 480, 298), ('hidpi-cached-480x294', 480, 294)]:
            await resize_fixture(page, game, width, height)
            await check_geometry(page, game, name)
        report['checks']['hidpi_buffer_matches_device_pixels'] = await game.evaluate('Math.abs(canvas.width-canvas.clientWidth*devicePixelRatio)<=1 && Math.abs(canvas.height-canvas.clientHeight*devicePixelRatio)<=1')
        await ctx.close()

        ctx, page, frame = await context(failure=True)
        await frame.locator('#play').click()
        await frame.locator('body[data-state="error"]').wait_for(timeout=15000)
        report['checks']['download_error_has_retry'] = await frame.locator('#play').inner_text() == 'Try again'
        report['checks']['download_error_has_fullpage'] = await frame.get_by_text('Open full page ↗').is_visible()
        await ctx.close()
        await save_geometry_matrix(browser)
        await browser.close()

try:
    asyncio.run(main())
except Exception as e:
    report['test_error'] = repr(e)
    raise
finally:
    (OUT / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'checks_passed': sum(bool(v) for v in report['checks'].values()), 'checks_failed': [k for k,v in report['checks'].items() if not v], 'geometry_cases': len(report['geometry']), 'errors': report['errors'], 'output': str(OUT)}, indent=2))
assert all(report['checks'].values()), report['checks']
