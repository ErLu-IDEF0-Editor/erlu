//! Display presets, not exchange rates or document currency identities.
use crate::localization;

#[derive(Clone, Copy, Debug)]
pub struct CurrencyPreset {
    pub code: &'static str,
    pub name: &'static str,
    pub symbol: &'static str,
}
impl CurrencyPreset {
    /// Logical Unicode; the UI applies its own display adapter.
    pub fn caption(&self, language: &str) -> String {
        let name = localization::translate(language, self.name);
        if self.code.is_empty() && self.symbol == self.name {
            name
        } else if self.code.is_empty() {
            format!("{} — {name}", self.symbol)
        } else {
            format!("{} — {} — {name}", self.symbol, self.code)
        }
    }
}

/// First ten: global FX turnover, BIS April 2025, final June 2026 tables, table 25.2.
/// https://www.bis.org/publications/triennial-survey/rpfx25-fx-annex-2606.pdf
/// RUB, generic units and person-hours remain available outside that ranking.
/// Only `symbol` is stored in Metadata.currency, preserving the existing format.
pub const PRESETS: [CurrencyPreset; 13] = [
    CurrencyPreset {
        code: "USD",
        name: "US dollar",
        symbol: "$",
    },
    CurrencyPreset {
        code: "EUR",
        name: "Euro",
        symbol: "€",
    },
    CurrencyPreset {
        code: "JPY",
        name: "Japanese yen",
        symbol: "¥",
    },
    CurrencyPreset {
        code: "GBP",
        name: "Pound sterling",
        symbol: "£",
    },
    CurrencyPreset {
        code: "CNY",
        name: "Chinese yuan",
        symbol: "¥",
    },
    CurrencyPreset {
        code: "CHF",
        name: "Swiss franc",
        symbol: "Fr",
    },
    CurrencyPreset {
        code: "AUD",
        name: "Australian dollar",
        symbol: "$",
    },
    CurrencyPreset {
        code: "CAD",
        name: "Canadian dollar",
        symbol: "$",
    },
    CurrencyPreset {
        code: "HKD",
        name: "Hong Kong dollar",
        symbol: "$",
    },
    CurrencyPreset {
        code: "SGD",
        name: "Singapore dollar",
        symbol: "$",
    },
    CurrencyPreset {
        code: "RUB",
        name: "Russian ruble",
        symbol: "₽",
    },
    CurrencyPreset {
        code: "",
        name: "Conventional units",
        symbol: "¤",
    },
    CurrencyPreset {
        code: "",
        name: "person-hours",
        symbol: "person-hours",
    },
];
