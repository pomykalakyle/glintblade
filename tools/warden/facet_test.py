import sys
from PIL import Image
from wlib import Canvas, on_bg

def shoulder(cv, m=False):
    f = (lambda pts: [(120 - x, y) for x, y in pts]) if m else (lambda p: p)
    lit = not m
    # silhouette base (front facet)
    cv.poly(f([(14, 22), (44, 22), (49, 30), (48, 40), (42, 46), (22, 48), (9, 44), (5, 34), (6, 24)]), "S3" if lit else "S2")
    # top facet
    cv.poly(f([(16, 8), (36, 8), (45, 14), (48, 22), (40, 25), (14, 24), (9, 20), (9, 14)]), "S4" if lit else "S3")
    # bright ridge facet (upper-left bevel)
    cv.poly(f([(16, 8), (30, 8), (24, 12), (14, 15), (9, 14)]), "S5" if lit else "S4")
    # outer facet
    cv.poly(f([(9, 14), (14, 15), (14, 24), (9, 20)]), "S4" if lit else "S2")
    cv.poly(f([(6, 24), (14, 24), (12, 34), (5, 34)]), "S4" if lit else "S1")
    # lower outer facet
    cv.poly(f([(5, 34), (12, 34), (16, 42), (9, 44)]), "S2" if lit else "S1")
    # underside
    cv.poly(f([(9, 44), (16, 42), (42, 42), (42, 46), (22, 48)]), "S1" if lit else "S0")
    # inner facet toward the neck
    cv.poly(f([(44, 22), (48, 22), (49, 30), (48, 40), (42, 42), (42, 26)]), "S2" if lit else "S3")

cv = Canvas(120, 60)
shoulder(cv)
shoulder(cv, True)
cv.outline("OL")
on_bg(cv.image(), "#323a58", 6, pad=2).save(sys.argv[1])
