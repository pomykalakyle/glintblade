"""Geometry normals from the authored rig's native part volumes.

Normals follow hood, cloak, boots, arm and blade independently. Albedo shade
values are never interpreted as height. Alpha is taken only from the finished
sprite. The old approved normal rows are copied intact by export_eightway.py.
OpenGL tangent convention: red right, green up, blue towards the viewer.
"""
import math

import numpy as np
from PIL import Image

import blade
import parts
import rig


def encode(v):
    v = np.asarray(v, dtype=np.float64)
    v /= max(float(np.linalg.norm(v)), 1e-9)
    return tuple(int(x) for x in np.rint((v + 1) * 127.5).clip(0, 255))


def render(spec, finished):
    n = rig.CELL
    vectors = np.zeros((n, n, 3), np.uint8)
    assigned = np.zeros((n, n), bool)

    def volume(mask, shape):
        pts = [(x, y) for y in range(n) for x in range(n) if mask.px[y][x] != "."]
        if not pts:
            return
        top, bottom = min(y for _, y in pts), max(y for _, y in pts)
        for y in range(top, bottom + 1):
            xs = [x for x, yy in pts if yy == y]
            if not xs:
                continue
            cx = (min(xs) + max(xs)) / 2
            radius = max(1, (max(xs) - min(xs) + 1) / 2)
            for x in xs:
                tx = (x - cx) / radius
                if shape == "hood":
                    nx = tx * 0.64
                    ny = (1 - 2 * (y - top) / max(1, bottom - top)) * 0.32
                elif shape == "cloak":
                    nx = tx * 0.48
                    ny = max(0, 1 - (y - top) / 3) * 0.18
                else:
                    nx, ny = tx * 0.35, 0.18
                vectors[y, x] = encode((nx, ny, math.sqrt(max(.01, 1 - nx * nx - ny * ny))))
                assigned[y, x] = True

    d = spec.get("dir", "down")
    for x, y, grid in spec.get("under", []):
        mask = rig.new_canvas()
        mask.stamp(grid, x, y)
        volume(mask, "boot")
    for key, table, shape in (("body", parts.BODY, "cloak"), ("hood", parts.HOOD, "hood")):
        if spec.get(key) is not None:
            mask = rig.new_canvas()
            rig.stamp_part(mask, table[d], spec[key])
            volume(mask, shape)
    for x, y, grid in spec.get("extra", []):
        mask = rig.new_canvas()
        mask.stamp(grid, x, y)
        volume(mask, "boot")
    if spec.get("arm"):
        mask = rig.new_canvas()
        shoulder, hand = spec["arm"]
        rig.draw_arm(mask, shoulder, hand)
        volume(mask, "boot")
    if spec.get("flip"):
        mirrored, valid = vectors.copy(), assigned.copy()
        vectors[:], assigned[:] = 0, False
        for y in range(n):
            for x in range(n):
                xx = 2 * rig.ANCHOR[0] - x
                if valid[y, x] and 0 <= xx < n:
                    vectors[y, xx] = (255 - int(mirrored[y, x, 0]), int(mirrored[y, x, 1]), int(mirrored[y, x, 2]))
                    assigned[y, xx] = True

    # A real crystal face uses its own narrow prism, independent of the cloak's
    # width. The rear blade is masked by the foreground body in the actual rig.
    b = spec.get("blade")
    if b:
        hx, hy, k = b["hx"], b["hy"], b["k"]
        if spec.get("flip"):
            hx, k = 40 - hx, (8 - k) % 16
        blade_mask = rig.new_canvas()
        blade.draw(blade_mask, hx, hy, k, b.get("length"), b.get("glint"),
                   b.get("charged", 0), guard=b.get("guard", True),
                   hilt=b.get("hilt", True), dim=b.get("dim", False))
        lit, sh = blade.offsets(k)
        # The upper-left-facing facet and ridge are physical planes, not a
        # luminance gradient. Their geometry is fixed even during charge glints.
        pts = blade.points(hx, hy, k, (b.get("length") or blade.LENGTH[k]) + 1)
        pointset = {(x + rig.SHIFT[0], y + rig.SHIFT[1]) for x, y in pts[1:]}
        for y in range(n):
            for x in range(n):
                ch = blade_mask.px[y][x]
                if ch == "." or finished.px[y][x] != ch:
                    continue
                if b.get("behind") and assigned[y, x]:
                    continue
                if (x, y) in pointset:
                    normal = (0, 0, 1)
                elif (x - lit[0], y - lit[1]) in pointset:
                    normal = (lit[0] * .58, -lit[1] * .58, .81)
                else:
                    normal = (sh[0] * .58, -sh[1] * .58, .81)
                vectors[y, x] = encode(normal)
                assigned[y, x] = True

    # Outline and tiny pixel fixes inherit their closest native component.
    # This does not read albedo luminance and never changes the silhouette.
    known = np.argwhere(assigned)
    out = np.zeros((n, n, 4), np.uint8)
    for y in range(n):
        for x in range(n):
            if finished.px[y][x] == ".":
                continue
            if not assigned[y, x] and len(known):
                yy, xx = known[np.argmin((known[:, 0] - y) ** 2 + (known[:, 1] - x) ** 2)]
                vectors[y, x] = vectors[yy, xx]
            elif not assigned[y, x]:
                vectors[y, x] = (128, 128, 255)
            out[y, x, :3] = vectors[y, x]
            out[y, x, 3] = 255
    return Image.fromarray(out)
