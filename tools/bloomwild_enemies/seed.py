"""Style v2 matte seed projectile with a dusty husk and dried-rose split.

Hand-authored spin and impact grids preserve the reviewed 7x7 silhouette and timing.
The held seed shares this husk, ivory seam and rose split without emissive pixels.
Run from the repository root with uv run --no-project --with pillow python3 tools/bloomwild_enemies/seed.py.
"""

from pix import Canvas, grid, build_sheet

PAL = {
    "o": "#303b4d",  # outline
    "s": "#4d505f", "S": "#747678", "k": "#a3a38f", "K": "#bdbba1",  # husk (palettes_v2.json seed and earth ramps)
    "Y": "#bf9da9", "Z": "#dfdac1",  # matte dried-rose split and ivory seam
}
EMISSIVE = []

CW, CH = 12, 12
AX, AY = 6, 6  # anchor: the seed's center

# flight: the split turns a quarter at a time (| / - \)
SPIN = [
    grid("""
.kkS.
kKYSS
kSZSs
SSYSs
.sss.
"""),
    grid("""
.kkS.
kKSYS
kSZSs
SYSSs
.sss.
"""),
    grid("""
.kkS.
kKkSS
kYZYs
SSSSs
.sss.
"""),
    grid("""
.kkS.
kYkSS
kSZSs
SSSYs
.sss.
"""),
]
# impact: squash, crack open, halves fall
HIT = [
    (grid("""
.kkkSS.
kKYZYSs
.SSsss.
"""), (3, 1)),
    (grid("""
.kk...SS.
kKS.Z.SSs
.SS...ss.
"""), (4, 1)),
    (grid("""
kk.......S.
kS...Y...Ss
"""), (5, 0)),
]


def cell_of(g, center):
    """Center and outline an authored projectile grid in its existing cell."""
    c = Canvas(CW, CH)
    cx, cy = center
    c.paste(g, AX - cx, AY - cy)
    c.outline("o")
    return c


def build():
    """Write the authored sheet and its compatible animation manifest."""
    anims = [
        ("fly", [cell_of(g, (2, 2)) for g in SPIN], {"fps": 15, "durations": [4, 4, 4, 4],
            "notes": "loop while in flight; the engine draws the ground shadow"}),
        ("hit", [cell_of(g, c) for g, c in HIT], {"fps": 15, "durations": [3, 4, 6],
            "notes": "play once on impact or when it lands, then remove"}),
    ]
    return build_sheet(
        "seed", (CW, CH), (AX, AY), anims, PAL, EMISSIVE,
        extra={"notes": "Thorn imp seed projectile, 7x7 px with its outline. Same husk and matte split as the seed the imp "
                        "holds in windup_* and attack_*#0; spawn it on the imp's release_frame at release_offset. "
                        "No emissive colors. Husk uses the palettes_v2.json thorn_imp seed and earth ramps; the seam is dried rose and ivory."},
    )


if __name__ == "__main__":
    sheet, manifest = build()
    print(f"{__file__}: {sheet.size}, {len(manifest['anims'])} animation rows")
