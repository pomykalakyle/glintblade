#!/usr/bin/env python3
"""Read-only native alpha/contact profiles; never modifies game assets."""
from pathlib import Path
import hashlib
import json
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "out/pixellab/round7/foot-profiles"
OUT.mkdir(parents=True, exist_ok=True)
CASES = [
    ("hero", "idle_down", (17,32), (32,35), 4, True),
    ("sproutling", "idle_down", (6,19), (19,22), 3, True),
    ("thorn_imp", "idle_down", (10,23), (27,30), 3, True),
    ("crystal_beetle", "idle_down", (3,29), (14,22), 8, True),
    ("wisp", "idle", (0,16), (17,20), 2, False),
    ("ember_hound", "idle_down", (0,40), (19,22), 4, True),
    ("spitting_totem", "idle_down", (0,28), (31,35), 5, True),
    ("bloomwild_ruins", "pillar", (0,48), (42,52), 10, True),
    ("hero", "idle_right", (16,29), (32,35), 4, True),
    ("hero", "idle_up", (17,32), (32,35), 4, True),
    ("hero", "idle_left", (20,33), (32,35), 4, True),
    ("ember_hound", "idle_right", (0,40), (19,22), 4, True),
    ("ember_hound", "idle_up", (0,40), (18,21), 4, True),
    ("bloomwild_ruins", "pillar_broken_tall", (0,48), (44,54), 10, True),
    ("bloomwild_ruins", "pillar_broken_short", (0,48), (44,54), 10, True),
]


def runs(xs):
    result = []
    for x in xs:
        if result and result[-1][1] == x:
            result[-1][1] = x+1
        else:
            result.append([x,x+1])
    return result


reports = []
cells = []
for name,anim,xwindow,band,depth,contact in CASES:
    path = ROOT / "assets/sprites" / (name+".png")
    meta = json.loads(path.with_suffix(".json").read_text())
    cw,ch = meta["cell"]
    row = meta["anims"][anim]["row"]
    anchor = meta["anims"][anim].get("anchor",meta["anchor"])
    cell = Image.open(path).convert("RGBA").crop((0,row*ch,cw,(row+1)*ch))
    opacity = cell.getchannel("A")
    source_rows = []
    for y in range(max(0,band[0]),min(ch,band[1])):
        xs = [x for x in range(max(0,xwindow[0]),min(cw,xwindow[1])) if opacity.getpixel((x,y))>=128]
        source_rows.append({"y":y,"x_runs_half_open":runs(xs),"opaque_pixels":len(xs)})
    all_x = [x for r in source_rows for lo,hi in r["x_runs_half_open"] for x in range(lo,hi)]
    reports.append({"sheet":name,"animation":anim,"frame":0,"cell":[cw,ch],"anchor":anchor,
                    "source_sha256":hashlib.sha256(path.read_bytes()).hexdigest(),
                    "contact_window_x_half_open":xwindow,"contact_band_y_half_open":band,
                    "native_contact_x_span_half_open":[min(all_x),max(all_x)+1] if all_x else None,
                    "finite_volume_depth_hint":depth,"ground_contact":contact,"rows":source_rows,
                    "authored_collision_footprint":meta["anims"][anim].get("footprint",meta.get("footprint"))})
    cells.append(cell)

# Native source pixels at exactly 5x. Guides stay outside the source art.
def board(indices,dest,cols):
    tile_w,tile_h = 320,360
    result = Image.new("RGB",(tile_w*cols,tile_h*((len(indices)+cols-1)//cols)),(29,31,39))
    draw = ImageDraw.Draw(result)
    for index,i in enumerate(indices):
        r,cell = reports[i],cells[i]
        xx,yy = (index%cols)*tile_w,(index//cols)*tile_h
        draw.text((xx+5,yy+5),r["sheet"]+" / "+r["animation"],fill="white")
        draw.text((xx+5,yy+22),"anchor "+str(r["anchor"])+"; depth "+str(r["finite_volume_depth_hint"]),fill=(172,183,204))
        native = cell.resize((cell.width*5,cell.height*5),Image.Resampling.NEAREST)
        ox,oy = xx+(tile_w-native.width)//2,yy+48
        result.paste(native,(ox,oy),native)
        # External y ticks identify the source contact band without painting it.
        lo,hi = r["contact_band_y_half_open"]
        draw.line((ox-5,oy+lo*5,ox-5,oy+hi*5-1),fill=(115,218,206),width=2)
        draw.text((xx+5,yy+333),"band y"+str(r["contact_band_y_half_open"])+" x"+str(r["contact_window_x_half_open"]),fill=(172,183,204))
    result.save(dest)

board(list(range(8)),OUT/"native-contact-5x.png",4)
board(list(range(8,15)),OUT/"direction-contact-5x.png",4)
target = ROOT/"art/pixellab/2026-11-01-round7/foot-profiles.json"
target.write_text(json.dumps({"coordinate_convention":"All x/y ranges are source-cell coordinates, half open. Volume depth is a world-ground hint, not a row count. This is read-only analysis.","cases":reports},indent=2)+"\n")
print(target)
