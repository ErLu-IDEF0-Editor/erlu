//! Logical translations and display-only adaptation for egui's LTR text layout.
use serde_json::Value;
use std::sync::OnceLock;

pub const LANGUAGES: [(&str, &str); 6] = [
    ("en", "English"),
    ("ru", "Русский"),
    ("fr", "Français"),
    ("es", "Español"),
    ("ar", "العربية"),
    ("zh", "中文"),
];

pub fn catalog() -> &'static Value {
    static CATALOG: OnceLock<Value> = OnceLock::new();
    CATALOG.get_or_init(|| {
        serde_json::from_str(include_str!("../../../assets/i18n.json"))
            .expect("Translation catalog")
    })
}
pub fn supported(language: &str) -> bool {
    LANGUAGES.iter().any(|(code, _)| *code == language)
}
pub fn locale_language(locale: &str) -> &'static str {
    let base = locale.split(['-', '_', '.', '@']).next().unwrap_or("");
    LANGUAGES
        .iter()
        .find(|(code, _)| code.eq_ignore_ascii_case(base))
        .map_or("en", |(code, _)| code)
}
pub fn translate(language: &str, source: &str) -> String {
    catalog()[language][source]
        .as_str()
        .unwrap_or(source)
        .to_owned()
}
/// Keep persisted text, API values and SVG/PDF in logical Unicode order.
/// egui 0.37 does not implement bidi. ErluDisplay maps shaped visual forms
/// to private-use glyphs so egui does not shape/reverse them a second time.
pub fn display_text(source: &str) -> String {
    if !source
        .chars()
        .any(|c| ('\u{0600}'..='\u{06ff}').contains(&c))
    {
        return source.into();
    }
    let result = source
        .split('\n')
        .map(|line| {
            let shaped = arabic_reshaper::arabic_reshape(line);
            let bidi = unicode_bidi::BidiInfo::new(&shaped, None);
            bidi.paragraphs
                .iter()
                .map(|p| {
                    bidi.reorder_line(p, p.range.clone())
                        .chars()
                        .map(|c| match c as u32 {
                            n @ 0xfb50..=0xfeff => char::from_u32(0xe000 + n - 0xfb50).unwrap(),
                            n @ 0x0600..=0x06ff => char::from_u32(0xf000 + n - 0x0600).unwrap(),
                            _ => c,
                        })
                        .collect::<String>()
                })
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");
    // split/join preserves blank lines, including the final newline.
    result
}
