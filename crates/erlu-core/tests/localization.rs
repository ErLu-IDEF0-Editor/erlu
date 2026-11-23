use erlu_core::{localization::*, *};
use serde_json::json;
use std::collections::BTreeSet;

#[test]
fn catalogs_are_complete_and_preserve_placeholders() {
    let all = catalog();
    let keys: BTreeSet<_> = all["en"].as_object().unwrap().keys().collect();
    for (lang, _) in LANGUAGES {
        assert_eq!(
            all[lang]
                .as_object()
                .unwrap()
                .keys()
                .collect::<BTreeSet<_>>(),
            keys,
            "{lang}"
        );
        for key in &keys {
            let value = all[lang][*key].as_str().unwrap();
            assert!(!value.trim().is_empty(), "{lang}: {key}");
            for marker in ["%1", "%2"] {
                assert_eq!(
                    key.matches(marker).count(),
                    value.matches(marker).count(),
                    "{lang}: {key}"
                );
            }
            assert!(
                !value
                    .chars()
                    .any(|c| ('\u{e000}'..='\u{f8ff}').contains(&c)),
                "Display aliases must never enter translations"
            );
        }
    }
}
#[test]
fn all_languages_work_through_actions_without_changing_documents() {
    let mut engine = Engine::default();
    let original = engine.document.clone();
    for (lang, _) in LANGUAGES {
        engine
            .call("language_set", json!({"language":lang}), None)
            .unwrap();
        assert_eq!(engine.view.language, lang);
        assert_eq!(engine.document, original);
        assert!(!engine.dirty());
    }
    assert!(
        engine
            .call("language_set", json!({"language":"unknown"}), None)
            .is_err()
    );
    for (locale, lang) in [
        ("es_ES.UTF-8", "es"),
        ("ar-EG", "ar"),
        ("zh-Hans-CN", "zh"),
        ("FR_ca", "fr"),
        ("ru_RU", "ru"),
        ("C", "en"),
        ("de-DE", "en"),
    ] {
        assert_eq!(locale_language(locale), lang);
    }
}
#[test]
fn bundled_fonts_cover_every_ui_translation_and_arabic_visual_glyph() {
    let primary = ttf_parser::Face::parse(render::DISPLAY_FONT, 0).unwrap();
    let cjk = ttf_parser::Face::parse(render::CJK_FONT, 0).unwrap();
    for (lang, name) in LANGUAGES {
        for text in catalog()[lang]
            .as_object()
            .unwrap()
            .values()
            .map(|v| v.as_str().unwrap())
            .chain(std::iter::once(name))
        {
            for c in display_text(text).chars().filter(|c| !c.is_whitespace()) {
                assert!(
                    primary.glyph_index(c).is_some() || cjk.glyph_index(c).is_some(),
                    "Missing glyph {c:?} ({lang}: {text})"
                );
            }
        }
    }
    let logical = "حفظ PDF 123";
    let displayed = display_text(logical);
    assert!(displayed.contains("PDF 123"));
    assert!(
        displayed
            .chars()
            .any(|c| ('\u{e000}'..='\u{f8ff}').contains(&c))
    );
    assert_eq!(translate("ar", "Save"), "حفظ");
    assert_eq!(display_text("Save PDF 123"), "Save PDF 123");
}

#[test]
fn plugin_titles_fields_and_enums_are_localized_without_changing_wire_values() {
    fn check(value: &serde_json::Value) {
        match value {
            serde_json::Value::Object(object) => {
                if let Some(title) = object.get("title").and_then(|v| v.as_str()) {
                    for (language, _) in LANGUAGES {
                        assert!(
                            catalog()[language][title].is_string(),
                            "Missing {language}: {title}"
                        );
                    }
                }
                if let Some(values) = object.get("enum").and_then(|v| v.as_array()) {
                    for option in values.iter().filter_map(|v| v.as_str()) {
                        for (language, _) in LANGUAGES {
                            assert!(
                                catalog()[language][option].is_string(),
                                "Missing {language}: {option}"
                            );
                        }
                    }
                }
                for value in object.values() {
                    check(value);
                }
            }
            serde_json::Value::Array(values) => {
                for value in values {
                    check(value);
                }
            }
            _ => {}
        }
    }
    for manifest in [
        include_str!("../../../plugins/colors/plugin.json"),
        include_str!("../../../plugins/responsibility_map/plugin.json"),
        include_str!("../../../plugins/semantic_dictionary/plugin.json"),
    ] {
        check(&serde_json::from_str(manifest).unwrap());
    }
}
