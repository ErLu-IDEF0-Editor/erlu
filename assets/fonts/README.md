# Bundled fonts

- `DejaVuSans.ttf`: DejaVu Sans, under `LICENSE-DejaVu.txt`.
- `NotoSansSC.ttf`: Noto Sans SC, instantiated at regular weight 400 from the official Google Fonts
  repository (`ofl/notosanssc/NotoSansSC[wght].ttf`), under `LICENSE-NotoSansSC.txt`.
  Full character coverage is retained so Chinese model labels and exports do not depend on system fonts.
  Regenerate with `scripts/build_cjk_font.py /path/to/NotoSansSC-variable.ttf`
  using fontTools; the unmodified variable source is available from
  https://github.com/google/fonts/tree/main/ofl/notosanssc.
- `ErluDisplay.ttf`: derivative of DejaVu Sans under the same license, renamed
  ErluDisplay. It adds private-use cmap aliases for Arabic presentation forms.
  `scripts/build_display_font.py` regenerates it with fontTools, from the repository root:
  `nix-shell -p python3Packages.fonttools --run 'python3 scripts/build_display_font.py'`.

The aliases are used only by egui's display adapter. They prevent HarfRust from
reversing already shaped/reordered Arabic labels again. Documents, translations,
API responses, copied/saved reports and SVG/PDF retain logical Unicode text.
