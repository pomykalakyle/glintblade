"""Pack native PixelLab sources for Cinder. No generation, scaling or palette changes.

uv run --with pillow python tools/pixellab/build_cinder.py
"""
import argparse
import json
from pathlib import Path
from PIL import Image
from native_collision import add_collision
from lava_quilt import build as build_lava_quilt

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / 'art/pixellab/2026-11-01-round3/cinder'


def load(name):
    im = Image.open(SOURCE / name).convert('RGBA')
    assert set(im.getchannel('A').tobytes()) <= {0, 255}, name
    return im


def save(out, name, images, cell, anchor, *, anims=None, extra=None, rows=False):
    cw, ch = cell
    atlas = Image.new('RGBA', (cw, ch * len(images)) if rows else (cw * len(images), ch))
    for i, im in enumerate(images):
        assert im.size == (cw, ch), (name, im.size, cell)
        atlas.paste(im, (0, i*ch) if rows else (i*cw, 0))
    atlas.save(out / f'{name}.png')
    data = dict(cell=cell, anchor=anchor, anims=anims or {'body':dict(row=0, frames=len(images), fps=1)},
                notes='Native PixelLab source pixels. No scaling, interpolation, palette substitution or mirroring. '
                      'Generation receipts and selections: art/pixellab/2026-11-01-round3/cinder/.')
    if name == 'cinder_foundry':
        for anim in data['anims'].values():
            add_collision(anim, images[anim['row']], 10)
    if extra: data.update(extra)
    (out / f'{name}.json').write_text(json.dumps(data,indent=2)+'\n')


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--out', type=Path, default=ROOT/'assets/sprites')
    a = ap.parse_args()
    out = a.out.resolve(); out.mkdir(parents=True,exist_ok=True)
    selected = json.loads((SOURCE/'selection.json').read_text())
    for name, spec in selected.items():
        if name == 'cinder_markers':
            images=[]; anims={}
            for i,(anim,source) in enumerate(spec['sources'].items()):
                im=load(source['file']); target=Image.new('RGBA',tuple(spec['cell']))
                x,y=source['anchor']; ax,ay=spec['anchor']
                target.alpha_composite(im,(ax-x,ay-y))
                # A native shift must not crop the original subject.
                assert sum(a > 0 for a in target.getchannel('A').tobytes()) == sum(a > 0 for a in im.getchannel('A').tobytes())
                images.append(target)
                anims[anim]=dict(row=i,frames=1,fps=1)
                add_collision(anims[anim], target, 8)
            atlas=Image.new('RGBA',(spec['cell'][0],spec['cell'][1]*len(images)))
            for i,im in enumerate(images):atlas.paste(im,(0,i*spec['cell'][1]))
            atlas.save(out/f'{name}.png')
            (out/f'{name}.json').write_text(json.dumps(dict(cell=spec['cell'],anchor=spec['anchor'],anims=anims,
                 notes='PixelLab native foundry markers; collision follows the exact opaque structural base on the unchanged source grid.'),indent=2)+'\n')
        else:
            save(out,name,[load(x) for x in spec['sources']],spec['cell'],spec['anchor'],
                 anims=spec.get('anims'),extra=spec.get('extra'),rows=spec.get('rows',False))
    # The retained short roadside marker uses the old art, whose wider visible plinth
    # still needs the same physical contract. Preserve the original PNG byte for byte.
    legacy = ROOT / 'art/pixellab/2026-11-01-round4/cinder/legacy'
    for name, depths in [('cinder_pillars', None), ('cinder_props', {
            'urn_tall':4, 'urn_squat':4, 'lava_rock':4, 'shard_small':3})]:
        data = json.loads((legacy/f'{name}.json').read_text())
        atlas = Image.open(legacy/f'{name}.png').convert('RGBA')
        cw, ch = data['cell']
        for anim_name, anim in data['anims'].items():
            if depths is not None and anim_name not in depths: continue
            row = anim['row']
            add_collision(anim, atlas.crop((0, row*ch, cw, (row+1)*ch)),
                          8 if depths is None else depths[anim_name])
        data['notes'] += ' Structural base collision follows the exact native opaque rows.'
        (out/f'{name}.json').write_text(json.dumps(data,indent=2)+'\n')
        (out/f'{name}.png').write_bytes((legacy/f'{name}.png').read_bytes())
    lava = ROOT / 'art/pixellab/2026-11-01-round5/cinder/lava-study'
    selection = json.loads((lava/'selection.json').read_text())
    molten, audit = build_lava_quilt(lava)
    assert molten.size == tuple(selection['native_cell'])
    assert audit['exact_native_origins_verified'] == molten.width * molten.height
    assert set(molten.getchannel('A').tobytes()) == {255}
    save(out,'cinder_lava_surface',[molten],molten.size,(0,0),extra={
        'native_sources':selection['native_sources'],
        'native_origin_audit':audit['exact_native_origins_verified'],
        'notes':'Native PixelLab source quilt with connected minimum-error integer cut seams. '
                'All278784 output pixels exactly match recorded source coordinates. '
                'No resizing, interpolation, recolouring or flips. Prompt, receipt, unchanged '
                'sources and complete cut/origin audit: art/pixellab/2026-11-01-round5/cinder/lava-study/.'})
    print('Packed Cinder:', ', '.join([*selected, 'cinder_pillars', 'cinder_props', 'cinder_lava_surface']))

if __name__=='__main__': main()
