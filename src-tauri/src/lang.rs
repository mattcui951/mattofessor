//! The languages Mattofessor speaks, and each one's names in the outside world: Data Dragon's
//! locale, CommunityDragon's folder, Windows OCR's language tag and the League client locales that
//! pick it. To add a language: an entry here, the same entry in `src/lib/locales.ts`, and its
//! dictionary (`src/lib/locales/<id>.ts`).

pub struct Lang {
    /// Mattofessor's own id (settings, the page): "en", "zh" (Simplified), "zh-TW", "ja"...
    pub id: &'static str,
    /// Data Dragon locale.
    pub dd: &'static str,
    /// CommunityDragon's folder for the language ("default" is English).
    pub cdragon: &'static str,
    /// Windows OCR language tag (the card reader).
    pub ocr: &'static str,
    /// League client locales that mean this language (prefixes of e.g. "es_MX").
    pub client: &'static [&'static str],
}

pub const LANGS: [Lang; 8] = [
    Lang { id: "en", dd: "en_US", cdragon: "default", ocr: "en-US", client: &["en_"] },
    Lang { id: "zh", dd: "zh_CN", cdragon: "zh_cn", ocr: "zh-Hans-CN", client: &["zh_CN", "zh_MY", "zh_SG"] },
    Lang { id: "zh-TW", dd: "zh_TW", cdragon: "zh_tw", ocr: "zh-Hant-TW", client: &["zh_TW", "zh_HK"] },
    Lang { id: "ja", dd: "ja_JP", cdragon: "ja_jp", ocr: "ja-JP", client: &["ja_"] },
    Lang { id: "ko", dd: "ko_KR", cdragon: "ko_kr", ocr: "ko-KR", client: &["ko_"] },
    Lang { id: "es", dd: "es_ES", cdragon: "es_es", ocr: "es-ES", client: &["es_"] },
    Lang { id: "fr", dd: "fr_FR", cdragon: "fr_fr", ocr: "fr-FR", client: &["fr_"] },
    Lang { id: "ru", dd: "ru_RU", cdragon: "ru_ru", ocr: "ru-RU", client: &["ru_"] },
];

pub fn english() -> &'static Lang {
    &LANGS[0]
}

/// A language by Mattofessor id or Data Dragon locale (any case); English when unknown.
pub fn get(id: &str) -> &'static Lang {
    LANGS.iter().find(|l| l.id.eq_ignore_ascii_case(id) || l.dd.eq_ignore_ascii_case(id)).unwrap_or_else(english)
}

/// Whether Mattofessor has this language (an id from the settings or the page).
pub fn known(id: &str) -> bool {
    LANGS.iter().any(|l| l.id == id)
}

/// The language of a League client locale ("zh_TW", "es_MX"...); English for the others.
pub fn from_client(locale: &str) -> &'static Lang {
    LANGS.iter().find(|l| l.client.iter().any(|p| locale.starts_with(p))).unwrap_or_else(english)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_ids_locales_and_clients() {
        assert_eq!(get("zh").dd, "zh_CN");
        assert_eq!(get("zh-TW").cdragon, "zh_tw");
        assert_eq!(get("ja_JP").id, "ja");
        assert_eq!(get("klingon").id, "en");
        assert_eq!(from_client("zh_TW").id, "zh-TW");
        assert_eq!(from_client("zh_CN").id, "zh");
        assert_eq!(from_client("es_MX").id, "es");
        assert_eq!(from_client("ko_KR").id, "ko");
        assert_eq!(from_client("pt_BR").id, "en");
        assert_eq!(from_client("ru_RU").id, "ru");
        assert!(known("fr") && known("ru") && !known("de"));
    }
}
