"""Dump a canvas region as a char block (for hand editing). usage: dump.py mod:func x0 y0 x1 y1"""
import importlib
import sys

from wlib import CHARS

inv = {v: k for k, v in CHARS.items()}
mod, fn = sys.argv[1].split(":")
cv = getattr(importlib.import_module(mod), fn)()
x0, y0, x1, y1 = map(int, sys.argv[2:6])
print("    " + "".join(str((x // 10) % 10) for x in range(x0, x1 + 1)))
print("    " + "".join(str(x % 10) for x in range(x0, x1 + 1)))
for y in range(y0, y1 + 1):
    row = "".join(inv.get(cv.get(x, y), "?") if cv.get(x, y) else "." for x in range(x0, x1 + 1))
    print(f"{y:3d}|{row}")
