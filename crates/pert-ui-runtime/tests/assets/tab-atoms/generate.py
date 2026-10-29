"""Generate the synthetic tab-atom test font from scratch, with no input font.

Optional maintenance tool, never run by a build or test. Generated with
fontTools 4.60.1; see README.md for the committed font's hash.

Usage: python3 generate.py [DESTINATION]
The default destination is TabAtomFixture.ttf beside this script.
"""
# SPDX-License-Identifier: MIT OR Apache-2.0
import sys
from pathlib import Path
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.feaLib.builder import addOpenTypeFeaturesFromString
import hashlib

destination = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(__file__).with_name('TabAtomFixture.ttf')
order = ['.notdef', 'space', 'tab', 'X', 'A', 'B', 'mixed', 'backmixed', 'tabs']
builder = FontBuilder(1000, isTTF=True)
builder.setupGlyphOrder(order)
builder.setupCharacterMap({9:'tab',32:'space',65:'A',66:'B',88:'X'})
glyphs = {}
for index,name in enumerate(order):
    pen = TTGlyphPen(None)
    if name != 'space':
        # Deliberately visible tab and ligature shapes, distinct from .notdef.
        left = 40
        right = 250 if name == 'tab' else (1050 if name in ('mixed','backmixed','tabs') else 500)
        bottom = 40 + index * 10
        pen.moveTo((left,bottom))
        pen.lineTo((right,bottom))
        pen.lineTo((right,700))
        pen.lineTo((left,700))
        pen.closePath()
    glyphs[name] = pen.glyph()
builder.setupGlyf(glyphs)
builder.setupHorizontalMetrics({name: (1200 if name in ('mixed','backmixed','tabs') else 600,0) for name in order})
builder.setupHorizontalHeader(ascent=800, descent=-200)
builder.setupNameTable({'familyName':'Pert UI Tab Atom Fixture','styleName':'Regular','uniqueFontIdentifier':'Pert UI Tab Atom Fixture 1','fullName':'Pert UI Tab Atom Fixture Regular','psName':'PertUITabAtomFixture-Regular','version':'Version 1.0','copyright':'Copyright 2026 pert-ui contributors. Licensed under MIT OR Apache-2.0.'})
builder.setupOS2(sTypoAscender=800,sTypoDescender=-200,sTypoLineGap=0,usWinAscent=800,usWinDescent=200,sxHeight=500,sCapHeight=700)
builder.setupPost()
builder.setupMaxp()
addOpenTypeFeaturesFromString(builder.font, '''
languagesystem DFLT dflt;
languagesystem latn dflt;
feature rlig {
  sub tab X by mixed;
  sub X tab by backmixed;
  sub tab tab by tabs;
} rlig;
''')
builder.font['head'].created = 3865622400
builder.font['head'].modified = 3865622400
builder.font.recalcTimestamp = False
builder.save(destination)
print(destination.name, destination.stat().st_size, hashlib.sha256(destination.read_bytes()).hexdigest())
