import sys
from rig import frame
from zoom import zoom
import anims
name, d = sys.argv[1], sys.argv[2]
idx = [int(x) for x in sys.argv[4:]] if len(sys.argv) > 4 else None
specs, durs = getattr(anims, name)(d) if d != "-" else getattr(anims, name)()
fr = [frame(s)[0] for s in specs]
if idx: fr = [fr[i] for i in idx]
zoom(fr, sys.argv[3], k=10, ground="#1f8250", box=(4, 4, 44, 46))
