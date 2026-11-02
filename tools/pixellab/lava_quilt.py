"""Pack native molten source crops with minimum-error cut seams; never interpolate pixels.

Uses only Pillow and the Python standard library. Every output pixel records an exact
(candidate, x, y) origin in unchanged PixelLab source PNGs. The periodic closure uses the
same cut method, so opposite edges share continuous source rows/columns at the wrap.
"""
from array import array
from pathlib import Path
from PIL import Image

PALETTE = [(0x24,0x23,0x2d,255),(0x61,0x30,0x39,255),(0xa6,0x4c,0x3b,255),
           (0xd8,0x78,0x4d,255),(0xf2,0xae,0x66,255),(0xff,0xdd,0xa0,255)]
ERROR = [[sum((a-b)**2 for a,b in zip(x[:3],y[:3])) for y in PALETTE] for x in PALETTE]


def cut(cost):
    """One connected minimum-error seam from top to bottom; adjacent steps <=1px."""
    height,width=len(cost),len(cost[0]);previous=cost[0][:];parents=[]
    for row in cost[1:]:
        p=[];current=[]
        for x in range(width):
            at=min(range(max(0,x-1),min(width,x+2)),key=previous.__getitem__)
            p.append(at);current.append(row[x]+previous[at])
        parents.append(p);previous=current
    x=min(range(width),key=previous.__getitem__);path=[x]
    for p in reversed(parents):x=p[x];path.append(x)
    return path[::-1]


def build(source_dir, origin_out=None):
    source_dir=Path(source_dir);sources={};raw={}
    for number in (2,3):
        im=Image.open(source_dir/f'result-{number}.png').convert('RGBA')
        assert im.size==(128,128)
        # The Pro batch used a onepixel uniform staging border; no subject pixel is scaled.
        for box in [(0,0,1,128),(127,0,128,128),(0,0,128,1),(0,127,128,128)]:
            assert set(im.crop(box).getdata())=={PALETTE[0]}
        raw[number]=list(im.getdata());sources[number]=[PALETTE.index(p) for p in raw[number]]
    patch,overlap,columns,rows=112,24,12,3;step=patch-overlap
    width,height=patch+(columns-1)*step,patch+(rows-1)*step
    pixels=bytearray([255])*(width*height);origin=array('I',[0])*(width*height)
    placements=[]
    for gy in range(rows):
        for gx in range(columns):
            x,y=gx*step,gy*step
            # The intricate source dominates; occasional larger rafts interrupt its recurrence.
            number=2 if (gx+gy*3)%5==3 else 3
            src=sources[number]
            options=[(1+(gx*7+gy*3+i*5)%15,1+(gx*3+gy*11+i*7)%15) for i in range(8)]
            def score(offset):
                ox,oy=offset;total=0
                if gx:
                    for py in range(patch):
                        for px in range(overlap):
                            a=pixels[(y+py)*width+x+px]
                            if a!=255:total+=ERROR[a][src[(oy+py)*128+ox+px]]
                if gy:
                    for py in range(overlap):
                        for px in range(overlap if gx else 0,patch):
                            a=pixels[(y+py)*width+x+px]
                            if a!=255:total+=ERROR[a][src[(oy+py)*128+ox+px]]
                return total
            ox,oy=min(options,key=score)
            left=cut([[ERROR[pixels[(y+py)*width+x+px]][src[(oy+py)*128+ox+px]]
                       for px in range(overlap)] for py in range(patch)]) if gx else [-1]*patch
            top=cut([[ERROR[pixels[(y+py)*width+x+px]][src[(oy+py)*128+ox+px]]
                      for py in range(overlap)] for px in range(patch)]) if gy else [-1]*patch
            for py in range(patch):
                for px in range(patch):
                    i=(y+py)*width+x+px
                    if pixels[i]==255 or ((not gx or px>=left[py]) and (not gy or py>=top[px])):
                        sx,sy=ox+px,oy+py;pixels[i]=src[sy*128+sx]
                        origin[i]=number*16384+sy*128+sx
            placements.append(dict(candidate=number,source_crop=[ox,oy,ox+patch,oy+patch],target=[x,y],left_cut=left,top_cut=top))
    # Close the torus by cutting through duplicated edge bands, retaining continuous
    # native source rows at the actual wrap and cutting only where palette error is low.
    sx=cut([[ERROR[pixels[y*width+1+x]][pixels[y*width+width-overlap+1+x]]
             for x in range(overlap-2)] for y in range(height)])
    new_width=width-overlap
    for y,seam in enumerate(sx):
        for x in range(seam+1):
            dst=y*width+x;src=y*width+new_width+x
            pixels[dst]=pixels[src];origin[dst]=origin[src]
    reduced=bytearray();reduced_origin=array('I')
    for y in range(height):
        reduced.extend(pixels[y*width:y*width+new_width]);reduced_origin.extend(origin[y*width:y*width+new_width])
    pixels,origin,width=reduced,reduced_origin,new_width
    sy=cut([[ERROR[pixels[(1+y)*width+x]][pixels[(height-overlap+1+y)*width+x]]
             for y in range(overlap-2)] for x in range(width)])
    new_height=height-overlap
    for x,seam in enumerate(sy):
        for y in range(seam+1):
            dst=y*width+x;src=(new_height+y)*width+x
            pixels[dst]=pixels[src];origin[dst]=origin[src]
    pixels=pixels[:width*new_height];origin=origin[:width*new_height];height=new_height
    im=Image.new('RGBA',(width,height));im.putdata([PALETTE[p] for p in pixels])
    for p,o in zip(pixels,origin):
        number,at=divmod(o,16384);assert PALETTE[p]==raw[number][at], 'non-native pixel introduced'
    import hashlib
    encoded=b''.join(o.to_bytes(4,'little') for o in origin)
    if origin_out is not None:
        origins=Image.new('RGB',(width,height))
        origins.putdata([(o//16384,(o%16384)%128,(o%16384)//128) for o in origin])
        origins.save(origin_out)
    audit={'method':'Native source quilting with connected minimum-error integer cut seams; no interpolation, recolouring, flipping or scaling',
           'size':[width,height],'pixels':len(pixels),'exact_native_origins_verified':len(pixels),
           'origin_map_encoding':'native-origins.png RGB=(candidate, original128px source x, original128px source y)',
           'origin_map_sha256':hashlib.sha256(encoded).hexdigest(),
           'palette':len(set(pixels)),'source_candidates':[2,3],'patch':patch,'overlap':overlap,'placements':placements,
           'periodic_cut_x':sx,'periodic_cut_y':sy}
    return im,audit


if __name__=='__main__':
    import argparse,json,hashlib
    p=argparse.ArgumentParser();p.add_argument('--source',type=Path,required=True);p.add_argument('--out',type=Path,required=True);a=p.parse_args()
    a.out.parent.mkdir(parents=True,exist_ok=True)
    image,audit=build(a.source,a.out.with_name('native-origins.png'));image.save(a.out)
    audit['sha256']=hashlib.sha256(a.out.read_bytes()).hexdigest()
    a.out.with_suffix('.json').write_text(json.dumps(audit,indent=2)+'\n')
    print(a.out,image.size,'nativepixels',audit['exact_native_origins_verified'])
