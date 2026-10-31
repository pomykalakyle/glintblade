"""Previews: scaled sheets over Bloomwild ground, animated GIFs, and scale lineups next to Glint."""

import json

from PIL import Image, ImageDraw

from pix import PREVIEWS, SPRITES, hex_rgb

GRASS = hex_rgb("#1f8250")
GRASS_DARK = hex_rgb("#145846")
GRASS_LIGHT = hex_rgb("#55ad4c")
STONE = hex_rgb("#b9aab0")
STONE_DARK = hex_rgb("#857a98")


def ground(w, h, kind="grass"):
    """A calm native-res patch of Bloomwild ground: grass with a few tufts, or a stone path."""
    im = Image.new("RGB", (w, h), GRASS if kind == "grass" else STONE)
    p = im.load()
    if kind == "grass":
        # sparse tufts at hashed positions (deterministic, no visible pattern)
        n = w * h // 70
        for i in range(n):
            hsh = (i * 2654435761 + 12345) & 0xFFFFFFFF
            x = hsh % w
            y = (hsh >> 12) % h
            if 1 <= x < w - 1 and 1 <= y < h:
                p[x, y] = GRASS_DARK
                p[x + 1, y] = GRASS_DARK
                p[x, y - 1] = GRASS_LIGHT if (hsh >> 7) % 4 == 0 else GRASS_DARK
    else:
        for y in range(h):
            if y % 9 == 8:
                for x in range(w):
                    p[x, y] = STONE_DARK
            off = 0 if (y // 9) % 2 == 0 else 8
            for x in range(w):
                if (x + off) % 16 == 15 and y % 9 != 8:
                    p[x, y] = STONE_DARK
    return im


def shadow(im, cx, cy, rx, ry, k=0.6):
    p = im.load()
    for y in range(cy - ry, cy + ry + 1):
        for x in range(cx - rx, cx + rx + 1):
            if 0 <= x < im.width and 0 <= y < im.height:
                if ((x - cx) / (rx + 0.5)) ** 2 + ((y - cy) / (ry + 0.5)) ** 2 <= 1.0:
                    r, g, b = p[x, y][:3]
                    p[x, y] = (int(r * k * 0.9), int(g * k * 0.85), int(b * k * 1.05))


def load_sheet(name):
    im = Image.open(SPRITES / f"{name}.png").convert("RGBA")
    man = json.load(open(SPRITES / f"{name}.json"))
    return im, man


def cell(im, man, anim, i, flip=False):
    cw, ch = man["cell"]
    a = man["anims"][anim]
    c = im.crop((i * cw, a["row"] * ch, (i + 1) * cw, (a["row"] + 1) * ch))
    if flip:
        c = c.transpose(Image.FLIP_LEFT_RIGHT)
    return c


def anchor(man, anim):
    return tuple(man["anims"][anim].get("anchor", man["anchor"]))


def durations(man, anim):
    a = man["anims"][anim]
    if "durations" in a:
        return a["durations"]
    return [round(60 / a["fps"])] * a["frames"]


def glint():
    """Glint's first idle frame if the hero sheet exists, else a rough 18 px stand-in."""
    try:
        im, man = load_sheet("hero")
        name = next(n for n in ("idle_down", "idle_right", "idle") if n in man["anims"])
        return cell(im, man, name, 0), anchor(man, name)
    except Exception:
        g = Image.new("RGBA", (12, 20), (0, 0, 0, 0))
        d = ImageDraw.Draw(g)
        o, c1, c2, sc, ey = (hex_rgb(h) + (255,) for h in ("#0a0a24", "#1d2266", "#2c3a9a", "#62c4f6", "#8ff8ff"))
        d.rectangle((2, 1, 9, 18), fill=o)
        d.rectangle((3, 2, 8, 17), fill=c2)
        d.rectangle((6, 9, 8, 17), fill=c1)
        d.rectangle((3, 8, 8, 9), fill=sc)
        d.rectangle((4, 4, 7, 6), fill=hex_rgb("#05051a") + (255,))
        g.putpixel((4, 5), ey)
        g.putpixel((6, 5), ey)
        return g, (6, 18)


def place(bg, spr, anc, x, y, shadow_rx=None):
    if shadow_rx:
        shadow(bg, x, y, shadow_rx, 2)
    bg.paste(spr, (x - anc[0], y - anc[1]), spr)


def up(im, s):
    return im.resize((im.width * s, im.height * s), Image.NEAREST)


def sheet_preview(name, scale=8, bg_kind="grass"):
    im, man = load_sheet(name)
    bg = ground(im.width, im.height, bg_kind).convert("RGBA")
    bg.alpha_composite(im)
    big = up(bg, scale)
    # label rows in a side gutter
    gut = 150
    out = Image.new("RGB", (big.width + gut, big.height), (16, 14, 30))
    out.paste(big.convert("RGB"), (gut, 0))
    d = ImageDraw.Draw(out)
    ch = man["cell"][1] * scale
    for an, a in man["anims"].items():
        d.text((6, a["row"] * ch + ch // 2 - 6), an, fill=(230, 226, 240))
        d.line((gut, a["row"] * ch, out.width, a["row"] * ch), fill=(16, 14, 30))
    out.save(PREVIEWS / f"{name}_sheet_{scale}x_{bg_kind}.png")
    return out


def anim_gif(name, anims, scale=6, pad=(28, 22), shadow_rx=6, flips=None, out=None, loops=2):
    """One GIF showing several anims side by side (each plays its own timing)."""
    im, man = load_sheet(name)
    cw, chh = man["cell"]
    flips = flips or [False] * len(anims)
    tw = cw + pad[0]
    W, H = tw * len(anims), chh + pad[1]
    # timeline at 60 Hz ticks
    seqs = []
    for an in anims:
        seq = []
        for i, d in enumerate(durations(man, an)):
            seq += [i] * d
        seqs.append(seq)
    total = max(len(s) for s in seqs) * loops
    base = ground(W, H)
    frames, ds = [], []
    last, run = None, 0
    for t in range(total):
        key = tuple(s[t % len(s)] for s in seqs)
        if key == last:
            ds[-1] += 1
            continue
        fr = base.copy().convert("RGBA")
        for k, an in enumerate(anims):
            c = cell(im, man, an, key[k], flips[k])
            a = anchor(man, an)
            ax = cw - 1 - a[0] if flips[k] else a[0]
            gx, gy = k * tw + tw // 2, H - 8
            shadow(fr, gx, gy, shadow_rx, 2)
            fr.alpha_composite(c, (gx - ax, gy - a[1]))
        frames.append(up(fr.convert("RGB"), scale))
        ds.append(1)
        last = key
    ms = [max(20, round(d * 1000 / 60 / 10) * 10) for d in ds]
    path = PREVIEWS / (out or f"{name}_{'_'.join(anims)}.gif")
    frames[0].save(path, save_all=True, append_images=frames[1:], duration=ms, loop=0, disposal=1)
    return path
