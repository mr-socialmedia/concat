#!/usr/bin/env python3
"""Builds the CutZone horizontal lockups with the wordmark as outlined paths.

The supplied logos carried live <text>, so they drew in Arial wherever Space Grotesk was
not installed. Here the word is shaped by HarfBuzz (kerning included) from Space Grotesk
Bold and written out as vector paths: no font is needed to display the files.

    pip install fonttools uharfbuzz
    python3 scripts/build-wordmark.py [--font SpaceGrotesk[wght].ttf]

Space Grotesk is (c) 2020 The Space Grotesk Project Authors, SIL Open Font License 1.1,
https://github.com/floriankarsten/space-grotesk. Only outlines derived from it are
committed, not the font file.
"""
import argparse, io, re, urllib.request
from pathlib import Path

import uharfbuzz as hb
from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
from fontTools.ttLib import TTFont
from fontTools.varLib.instancer import instantiateVariableFont

FONT_URL = ("https://raw.githubusercontent.com/google/fonts/main/ofl/spacegrotesk/"
            "SpaceGrotesk%5Bwght%5D.ttf")
ROOT = Path(__file__).resolve().parent.parent
BRAND = ROOT / "brand"

SIZE = 67            # the supplied lockup's size, on the symbol's 120-unit grid
TRACKING = -0.025    # em; the guideline allows -0.02 to -0.03 (the supplied file used -0.045)
BASELINE = 85
TEXT_X = 140
RIGHT_MARGIN = 18

def load_font(path):
    if path is None:
        cache = Path("/tmp/SpaceGrotesk[wght].ttf")
        if not cache.exists():
            urllib.request.urlretrieve(FONT_URL, cache)
        path = cache
    font = TTFont(path)
    return instantiateVariableFont(font, {"wght": 700})

def shape(font, text):
    buf = io.BytesIO(); font.save(buf)
    face = hb.Face(buf.getvalue()); hbfont = hb.Font(face)
    b = hb.Buffer(); b.add_str(text); b.guess_segment_properties()
    hb.shape(hbfont, b, {"kern": True, "liga": True})
    order = font.getGlyphOrder()
    return [(order[i.codepoint], p.x_advance) for i, p in zip(b.glyph_infos, b.glyph_positions)]

def outline(font, runs, x, scale, tracking_units):
    glyphs = font.getGlyphSet(); parts = []
    for name, adv in runs:
        pen = SVGPathPen(glyphs, ntos=lambda v: f"{v:.2f}".rstrip("0").rstrip("."))
        glyphs[name].draw(TransformPen(pen, (scale, 0, 0, -scale, x, BASELINE)))
        parts.append(pen.getCommands())
        x += (adv + tracking_units) * scale
    return " ".join(p for p in parts if p), x

def inner(name):
    s = (BRAND / name).read_text()
    defs = re.search(r"<defs>(.*?)</defs>", s, re.S).group(1)
    body = s.split("</defs>", 1)[1].rsplit("</svg>", 1)[0]
    return defs, body

def lockup(font, cut_fill, zone_fill, mark, background=None, label="CutZone"):
    upem = font["head"].unitsPerEm; scale = SIZE / upem; tracking = TRACKING * upem
    cut, x = outline(font, shape(font, "Cut"), TEXT_X, scale, tracking)
    zone, x = outline(font, shape(font, "Zone"), x, scale, tracking)
    width = round(x - TRACKING * SIZE + RIGHT_MARGIN)
    defs, body = inner(mark)
    bg = f'<rect width="{width}" height="120" fill="{background}"/>' if background else ""
    return (f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="120" '
            f'viewBox="0 0 {width} 120" role="img" aria-label="{label}"><defs>{defs}</defs>{bg}{body}'
            f'<path d="{cut}" fill="{cut_fill}"/><path d="{zone}" fill="{zone_fill}"/></svg>\n')

if __name__ == "__main__":
    ap = argparse.ArgumentParser(); ap.add_argument("--font"); args = ap.parse_args()
    font = load_font(args.font)
    files = {
        # on dark surfaces: the supplied colours
        "logo-horizontal-dark.svg": lockup(font, "#F8F7FC", "#A78BFA", "symbol.svg"),
        "logo-horizontal-dark-bg.svg": lockup(font, "#F8F7FC", "#A78BFA", "symbol.svg", "#0B0A12"),
        # on light surfaces: Midnight and Deep Purple (10:1 on white), one-colour mark
        "logo-horizontal-light.svg": lockup(font, "#0B0A12", "#5B21B6", "symbol-black.svg"),
        "logo-horizontal-white.svg": lockup(font, "#FFFFFF", "#FFFFFF", "symbol-white.svg"),
        "logo-horizontal-black.svg": lockup(font, "#0B0A12", "#0B0A12", "symbol-black.svg"),
    }
    for name, svg in files.items():
        assert "<text" not in svg and "font-family" not in svg
        (BRAND / name).write_text(svg); print("wrote", name, len(svg), "bytes")
