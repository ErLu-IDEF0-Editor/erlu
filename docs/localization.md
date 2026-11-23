# Interface localization

The desktop and browser interfaces support English (`en`), Russian (`ru`), French
(`fr`), Spanish (`es`), Arabic (`ar`) and Simplified Chinese (`zh`). Select a language
in **View → Language**. The initial language follows the saved preference, then the
system locale (LC_ALL, LC_MESSAGES, LANG) or browser language. Unknown locales fall
back to English. Interface language does not change model content or enum/API values.
MCP and browser clients can also use `language_set`.

`assets/i18n.json` contains 320 keys per language, including menus, dialogs,
validation captions, supplied plugin action/form labels and browser recovery UI.
Plugin schemas and serialized enum values stay language-neutral; only their displayed
captions are translated. Plugin-generated data/reports are kept as returned by the
plugin, rather than rewriting user terminology or machine-readable exports.

`erlu-core::localization` owns the language list, locale normalization and shared
logical translations. Labels fall back to their source text for unknown external
plugin strings. Add new UI strings to every language when changing the interface.

## Fonts and Arabic display

Bundled fonts cover the UI without system font dependencies. Noto Sans SC is a
regular-weight static instance with full character coverage, available to both egui
and the SVG/PDF font database. See `assets/fonts/README.md` for font sources,
licenses and regeneration commands.

Egui 0.37 shapes glyphs but does not provide full bidirectional text layout. Its
read-only Arabic captions therefore use an explicit reshaping/bidi display adapter
and private-use aliases in the derived ErluDisplay font to avoid double shaping.
This adaptation is confined to rendered egui labels. Translation files, model data,
editable values, API output, clipboard/report exports and SVG/PDF stay in logical
Unicode order. Diagram coordinates and ICOM directions are not mirrored. Browser
DOM recovery messages use native RTL layout.

## Verification (23 November 2026)

- 320 keys in each language; tests verify equal key sets and `%1`/`%2` placeholders.
- Tests verify glyph coverage, localized plugin titles/fields/enums, locale parsing,
  language actions and unchanged document data/dirty state.
- The workspace suite (49 tests) and Clippy passed inside `nix-shell`.
- Desktop windows opened with isolated Spanish, Arabic and Chinese system locales;
  screenshots were inspected on Linux/Xvfb.
- Chromium scenarios exercise all six languages and translated recovery messages,
  while checking that model data stays unchanged. Browser screenshots cover all six.

Native Windows/macOS GUI acceptance remains separate. The browser suite now passes
on Chromium, Firefox and WebKit on Linux; see [browser tests](browser-tests.md).

## Currency choices

**Properties → ABC Units → Currency** lists the symbol, ISO code and localized name.
The first ten presets use the largest currencies by global FX turnover in the BIS
April 2025 survey (final June 2026 tables, table 25.2): USD, EUR, JPY, GBP, CNY,
CHF, AUD, CAD, HKD and SGD. Source:
https://www.bis.org/publications/triennial-survey/rpfx25-fx-annex-2606.pdf

Russian rubles, conventional units (`¤`, the generic currency sign, known in Russian
as «жучок») and person-hours remain available below the ranked list. The currency
field is also editable for custom symbols/units. No exchange-rate conversion occurs.

Only the selected symbol is stored in `meta.currency` and rendered with prices;
the descriptive caption and ISO code are menu-only. Thus USD, AUD, CAD, HKD and SGD
all display `$`, and JPY/CNY both display `¥`. This preserves the existing file
format: selecting a preset does not introduce a stored ISO currency identity.

Currency verification: the complete Rust suite (49 tests), Clippy and desktop/web
builds passed inside `nix-shell`. All 11 Chromium scenarios passed, including real
mouse selection of EUR, CAD and conventional units, checking both the stored symbol
and SVG price output without the descriptive currency name.
