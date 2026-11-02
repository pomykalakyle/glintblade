"""Remove only authored Cinder support-contour ink, retaining sprite geometry.

Native atlas coordinates are deliberately listed by hand. These are lower
ground-facing contours of existing stone volumes, not each column's lowest
opaque pixel. Upper/lateral outlines, face junctions, cracks, cavities and raised
tiers are excluded. No alpha, normals, metadata, new colours or blending.

The foundry's right face is already the same #101017 as its contour. It is NOT
brightened: there is no distinct adjacent face colour to continue there. Only
its identifiable left-face contour and central vertical rib are corrected.

Call apply_contact_ink(name, atlas) from the scoped native packer after assembling
the atlas. Unknown names return an unchanged copy. Source pixels are checked, and
the helper is idempotent so already corrected atlases are safe.
"""
from dataclasses import dataclass
from pathlib import Path
from PIL import Image


INK = (16, 16, 23, 255)
LEGACY_INK = (26, 25, 35, 255)
LEFT = (26, 25, 35, 255)
RIGHT = (63, 65, 79, 255)
RIB = (121, 112, 122, 255)
ASH = (41, 44, 60, 255)
WARM = (81, 72, 84, 255)
SOOT = (22, 21, 29, 255)
BROKEN = (57, 55, 70, 255)


@dataclass(frozen=True)
class ContactSpan:
    row: int
    y: int
    x0: int
    x1: int  # inclusive
    donor_x: int
    donor_y: int  # cell-local
    material: tuple


def S(row, y, x0, x1, dx, dy, colour):
    return ContactSpan(row, y, x0, x1, dx, dy, colour)


# The two native faces of the intact diamond meet at its lit vertical rib.
# The outline's short horizontal runs follow the original pixel staircase.
MARKERS = (
    S(0, 71, 21, 22, 23, 71, LEFT),
    S(0, 71, 41, 42, 40, 71, RIGHT),
    S(0, 72, 23, 24, 25, 72, LEFT),
    S(0, 72, 39, 40, 38, 72, RIGHT),
    S(0, 73, 25, 26, 27, 73, LEFT),
    S(0, 73, 37, 38, 36, 73, RIGHT),
    S(0, 74, 27, 28, 29, 74, LEFT),
    S(0, 74, 35, 36, 34, 74, WARM),
    S(0, 75, 29, 30, 29, 74, LEFT),
    S(0, 75, 31, 32, 31, 74, RIB),
    S(0, 75, 33, 34, 33, 74, WARM),
    # Broken-tall: inner split at x23/24 above the foot stays inked.
    S(1, 69, 17, 19, 20, 69, LEFT),
    S(1, 69, 45, 46, 44, 69, RIGHT),
    S(1, 70, 19, 21, 22, 70, LEFT),
    S(1, 70, 43, 44, 42, 70, RIGHT),
    S(1, 71, 22, 24, 25, 71, LEFT),
    S(1, 71, 41, 42, 40, 71, RIGHT),
    S(1, 72, 24, 25, 26, 72, LEFT),
    S(1, 72, 38, 40, 37, 72, RIGHT),
    S(1, 73, 26, 27, 28, 73, LEFT),
    S(1, 73, 35, 37, 34, 73, RIGHT),
    S(1, 74, 28, 29, 30, 74, LEFT),
    S(1, 74, 33, 34, 32, 74, RIGHT),
    S(1, 75, 30, 31, 30, 74, LEFT),
    S(1, 75, 32, 32, 32, 74, RIGHT),
    # Leaning marker: main diamond plus the separate low right support toe.
    # Preserve the long dark face junction at x36 and all upper break edges.
    S(2, 70, 19, 20, 21, 70, LEFT),
    S(2, 71, 21, 22, 23, 71, LEFT),
    S(2, 72, 23, 24, 25, 72, LEFT),
    S(2, 73, 25, 26, 27, 73, LEFT),
    S(2, 74, 27, 28, 29, 74, LEFT),
    S(2, 75, 29, 30, 29, 74, LEFT),
    S(2, 75, 31, 31, 31, 74, RIGHT),
    S(2, 74, 32, 33, 31, 74, RIGHT),
    S(2, 73, 34, 35, 33, 73, RIGHT),
    S(2, 71, 44, 45, 43, 71, RIGHT),
    S(2, 72, 41, 43, 40, 72, RIGHT),
    S(2, 72, 36, 38, 38, 71, LEFT),
    S(2, 73, 39, 39, 39, 72, LEFT),
    S(2, 73, 40, 40, 40, 72, RIGHT),
    # Slotted marker: three irregular ground-bearing lobes. The cavity roofs,
    # dividing verticals x25/34/39, shaft edges and raised ledges stay outlined.
    S(3, 70, 18, 18, 19, 70, LEFT),
    S(3, 71, 19, 20, 21, 71, LEFT),
    S(3, 72, 21, 22, 21, 71, LEFT),
    S(3, 72, 23, 23, 23, 71, SOOT),
    S(3, 73, 25, 26, 27, 73, LEFT),
    S(3, 74, 27, 28, 28, 73, LEFT),
    S(3, 75, 29, 31, 29, 74, RIGHT),
    S(3, 74, 32, 32, 31, 74, RIGHT),
    S(3, 73, 33, 33, 32, 73, RIGHT),
    S(3, 70, 45, 45, 44, 70, WARM),
    S(3, 71, 43, 43, 42, 71, RIGHT),
    S(3, 71, 44, 44, 44, 70, WARM),
    S(3, 72, 39, 40, 39, 71, LEFT),
    S(3, 72, 41, 42, 42, 71, RIGHT),
    S(3, 75, 35, 35, 35, 74, BROKEN),
    S(3, 75, 36, 38, 36, 74, WARM),
)


# Explicit lower-left foundation contours. The near-black right faces are
# material, not separable ink, and remain fully unchanged (see module note).
FOUNDRY = (
    S(0, 61, 8, 8, 9, 61, ASH),
    S(0, 62, 9, 10, 11, 62, ASH),
    S(0, 63, 11, 12, 13, 63, ASH),
    S(0, 64, 13, 14, 15, 64, ASH),
    S(0, 65, 15, 16, 17, 65, ASH),
    S(0, 66, 17, 18, 19, 66, ASH),
    S(0, 67, 19, 20, 21, 67, ASH),
    S(0, 68, 21, 23, 24, 68, ASH),
    S(0, 69, 24, 24, 25, 69, ASH),
    S(0, 70, 25, 26, 27, 70, ASH),
    S(0, 71, 27, 28, 29, 71, ASH),
    S(0, 72, 29, 30, 31, 72, ASH),
    S(0, 73, 31, 33, 31, 72, ASH),
    S(0, 73, 34, 34, 34, 72, RIGHT),
    S(1, 60, 7, 7, 8, 60, ASH),
    S(1, 61, 8, 9, 10, 61, ASH),
    S(1, 62, 10, 11, 12, 62, ASH),
    S(1, 63, 12, 13, 14, 63, ASH),
    S(1, 64, 14, 15, 16, 64, ASH),
    S(1, 65, 16, 17, 18, 64, ASH),
    S(1, 66, 18, 19, 20, 66, ASH),
    S(1, 67, 20, 21, 22, 67, ASH),
    S(1, 68, 22, 23, 24, 68, ASH),
    S(1, 69, 24, 25, 26, 69, ASH),
    S(1, 70, 26, 27, 28, 70, ASH),
    S(1, 71, 28, 29, 30, 71, ASH),
    S(1, 72, 30, 31, 33, 72, ASH),
    S(1, 73, 32, 32, 32, 72, (54, 51, 64, 255)),
    S(1, 73, 33, 33, 33, 72, ASH),
    S(1, 73, 34, 34, 34, 72, RIGHT),
)


# Only the LIVE broken-short cell in the retained legacy atlas. Its existing
# straight front face already has a clear material donor immediately above.
PILLARS = (S(3, 45, 6, 23, 6, 44, RIGHT),)

SPECS = {
    "cinder_markers": ((64, 320), 80, INK, MARKERS),
    "cinder_foundry": ((64, 160), 80, INK, FOUNDRY),
    "cinder_pillars": ((32, 192), 48, LEGACY_INK, PILLARS),
}


def apply_contact_ink(name, image):
    """Return corrected RGBA copy; raise if authored source contracts drift."""
    key = Path(name).stem
    out = image.convert("RGBA").copy()
    if key not in SPECS:
        return out
    size, height, ink, spans = SPECS[key]
    if out.size != size:
        raise ValueError(f"{key}: expected native atlas {size}, got {out.size}")
    source = out.copy()
    for span in spans:
        dy = span.row * height + span.donor_y
        donor = source.getpixel((span.donor_x, dy))
        if donor != span.material:
            raise ValueError(f"{key}: adjacent face donor drift at "
                             f"{(span.donor_x, dy)}: {donor}")
        for x in range(span.x0, span.x1 + 1):
            xy = (x, span.row * height + span.y)
            before = source.getpixel(xy)
            if before not in (ink, donor):
                raise ValueError(f"{key}: authored contact drift at {xy}: {before}")
            out.putpixel(xy, donor[:3] + (before[3],))
    return out
