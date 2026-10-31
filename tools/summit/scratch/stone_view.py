import sys
sys.path.insert(0, "..")
import numpy as np
from common import *
import stones as S
img, base = S.from_planes(S.MONO0_PLAN, lit_rows=range(3, 30))
S.overlay(img, S.MONO0_RUNES, 5, 9)
S.overlay(img, S.MONO0_CRACK, 6, 31)
S.overlay(img, S.MONO0_FROST, 2, 1)
S.finish(img, base, (2, 16))
S.overlay(img, S.MONO0_MOSS, 1, base - 5)
bg = np.zeros((img.shape[0] + 8, img.shape[1] + 30, 3), np.uint8); bg[:] = RGB["2"]
over(bg, img, 2, 2)
g, (ax, ay) = glint_standin()
over(bg, g, img.shape[1] + 8, 2 + base - ay + 1)
save_rgb("../../../assets/previews/_wip_summit_stone.png", bg, 8)
save_rgb("../../../assets/previews/_wip_summit_stone1x.png", bg, 2)
