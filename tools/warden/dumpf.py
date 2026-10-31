"""dumpf.py mod func args... : dump a full canvas as chars. args are python literals."""
import importlib, sys
from wlib import CHARS
inv = {v: k for k, v in CHARS.items()}
mod = importlib.import_module(sys.argv[1])
cv = getattr(mod, sys.argv[2])(*[eval(a) for a in sys.argv[3:]])
print("    " + "".join(str(x % 10) for x in range(cv.w)))
for y in range(cv.h):
    print(f"{y:3d}|" + "".join(inv.get(cv.get(x, y), "?") if cv.get(x, y) else "." for x in range(cv.w)))
