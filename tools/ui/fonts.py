"""Fonts: font_small (HUD + dialog), font_tiny (key caps, labels), font_display (region titles)."""

import glyphs_small as GS
from fontlib import Font


def font_small():
    return Font("font_small", GS.G, GS.CELL_H, GS.BASELINE, GS.CAP_H, GS.X_H, GS.TRACKING, GS.LINE_H,
                kerning=GS.KERN if hasattr(GS, "KERN") else None,
                notes="HUD and dialog font. Cap height 7, x-height 5, descenders 2. Draw with a 1px drop shadow "
                      "(ui ink #252d3c) over the game; text colors ui.text #dce0ce (body) and #9ba79e (secondary).")


def font_tiny():
    import glyphs_tiny as GT
    return Font("font_tiny", GT.G, GT.CELL_H, GT.BASELINE, GT.CAP_H, GT.X_H, GT.TRACKING, GT.LINE_H,
                notes="Tiny caps-only font (cap height 5) for key caps, button labels and small HUD numbers. "
                      "Lowercase letters map to the same caps.")


def font_display():
    import glyphs_display as GD
    return Font("font_display", GD.G, GD.CELL_H, GD.BASELINE, GD.CAP_H, GD.X_H, GD.TRACKING, GD.LINE_H,
                kerning=GD.KERN,
                notes="Title font, style v2: narrow cut-stone capitals (uniform 2 px strokes, 45-degree clipped "
                      "corners), the same construction as the logo. Cap height 13. Caps only: lowercase letters "
                      "map to the same capitals. Tracking 3 for a widely spaced inscription. Draw in bone "
                      "#dce0ce with a 1 px ink #252d3c drop shadow or outline.")
