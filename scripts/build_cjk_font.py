"""Make the bundled regular CJK font from Google Fonts' NotoSansSC[wght].ttf.

Usage from the repository root: python3 scripts/build_cjk_font.py /path/to/variable.ttf
Requires fontTools. License: assets/fonts/LICENSE-NotoSansSC.txt.
"""
import sys
from fontTools.ttLib import TTFont
from fontTools.varLib.instancer import instantiateVariableFont

font = TTFont(sys.argv[1])
instantiateVariableFont(font, {"wght": 400}, inplace=True, updateFontNames=True)
font.save("assets/fonts/NotoSansSC.ttf")
