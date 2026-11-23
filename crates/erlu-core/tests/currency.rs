use erlu_core::{Document, currency::PRESETS, localization, render};

#[test]
fn currency_choices_are_named_in_all_languages_and_keep_symbols_separate() {
    assert_eq!(
        PRESETS[..10].iter().map(|p| p.code).collect::<Vec<_>>(),
        [
            "USD", "EUR", "JPY", "GBP", "CNY", "CHF", "AUD", "CAD", "HKD", "SGD"
        ]
    );
    for preset in PRESETS {
        for (language, _) in localization::LANGUAGES {
            assert!(localization::catalog()[language][preset.name].is_string());
            let caption = preset.caption(language);
            if preset.symbol != preset.name {
                assert!(caption.starts_with(preset.symbol));
            }
            if !preset.code.is_empty() {
                assert!(caption.contains(preset.code));
            }
            assert!(caption.contains(&localization::translate(language, preset.name)));
        }
    }
    assert_eq!(
        PRESETS
            .iter()
            .find(|p| p.name == "Conventional units")
            .unwrap()
            .symbol,
        "¤"
    );
    assert!(PRESETS.iter().any(|p| p.code == "RUB" && p.symbol == "₽"));
    for code in ["USD", "AUD", "CAD", "HKD", "SGD"] {
        assert_eq!(PRESETS.iter().find(|p| p.code == code).unwrap().symbol, "$");
    }
}

#[test]
fn prices_and_saved_metadata_contain_symbols_not_menu_descriptions() {
    for preset in PRESETS {
        let mut doc = Document::default();
        doc.meta.currency = preset.symbol.into();
        doc.meta.currency_placement = "1 ?".into();
        doc.diagrams
            .get_mut(&doc.root)
            .unwrap()
            .blocks
            .get_mut(&2)
            .unwrap()
            .price = Some(12.5);
        let expected = format!("12.50 {}", preset.symbol);
        let scene = render::scene(&doc, doc.root, false, None);
        assert!(
            scene
                .primitives
                .iter()
                .any(|p| matches!(p,render::Primitive::Text{text,..} if *text==expected)),
            "{}",
            preset.name
        );
        let json = serde_json::to_value(&doc).unwrap();
        assert_eq!(json["meta"]["currency"], preset.symbol);
        let loaded: Document = serde_json::from_value(json).unwrap();
        assert_eq!(loaded.meta.currency, preset.symbol);
    }
}
