from fontTools.ttLib import TTFont
font=TTFont('assets/fonts/DejaVuSans.ttf')
for table in font['cmap'].tables:
 if not table.isUnicode():continue
 for code,glyph in list(table.cmap.items()):
  if 0xFB50<=code<=0xFEFF:table.cmap[0xE000+code-0xFB50]=glyph
  elif 0x0600<=code<=0x06ff:table.cmap[0xF000+code-0x0600]=glyph
for record in font['name'].names:
 if record.nameID in (1,2,3,4,6,16,17):
  value='Regular' if record.nameID in (2,17) else 'ErluDisplay'
  record.string=value.encode(record.getEncoding())
font.save('assets/fonts/ErluDisplay.ttf')
