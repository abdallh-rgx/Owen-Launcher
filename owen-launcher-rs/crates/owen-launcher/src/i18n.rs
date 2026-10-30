#![forbid(unsafe_code)]
//! The 15-language i18n of the launcher — the same set Fortnite ships with
//! (ar is the default, RTL). Dictionaries are embedded JSON
//! (`assets/i18n/*.json`); every missing key falls back to English, then to
//! the key itself.

use std::collections::HashMap;

/// One language of the picker.
pub struct LangMeta {
    pub code: &'static str,
    /// Name in the language itself (shown in the picker).
    pub native: &'static str,
    /// English name (secondary line + font fallback hint).
    pub english: &'static str,
    pub rtl: bool,
}

/// The 15 Fortnite languages.
pub const LANGS: &[LangMeta] = &[
    LangMeta { code: "ar", native: "العربية", english: "Arabic", rtl: true },
    LangMeta { code: "de", native: "Deutsch", english: "German", rtl: false },
    LangMeta { code: "en", native: "English", english: "English", rtl: false },
    LangMeta { code: "es", native: "Español", english: "Spanish (Spain)", rtl: false },
    LangMeta { code: "es419", native: "Español (Latinoamérica)", english: "Spanish (LatAm)", rtl: false },
    LangMeta { code: "fr", native: "Français", english: "French", rtl: false },
    LangMeta { code: "it", native: "Italiano", english: "Italian", rtl: false },
    LangMeta { code: "ja", native: "日本語", english: "Japanese", rtl: false },
    LangMeta { code: "ko", native: "한국어", english: "Korean", rtl: false },
    LangMeta { code: "pl", native: "Polski", english: "Polish", rtl: false },
    LangMeta { code: "ptbr", native: "Português (Brasil)", english: "Portuguese (Brazil)", rtl: false },
    LangMeta { code: "ru", native: "Русский", english: "Russian", rtl: false },
    LangMeta { code: "tr", native: "Türkçe", english: "Turkish", rtl: false },
    LangMeta { code: "zhcn", native: "简体中文", english: "Chinese (Simplified)", rtl: false },
    LangMeta { code: "zhtw", native: "繁體中文", english: "Chinese (Traditional)", rtl: false },
];

pub fn lang_meta(code: &str) -> &'static LangMeta {
    LANGS.iter().find(|l| l.code == code).unwrap_or(&LANGS[0])
}

const RAW: &[(&str, &str)] = &[
    ("ar", include_str!("../assets/i18n/ar.json")),
    ("de", include_str!("../assets/i18n/de.json")),
    ("en", include_str!("../assets/i18n/en.json")),
    ("es", include_str!("../assets/i18n/es.json")),
    ("es419", include_str!("../assets/i18n/es419.json")),
    ("fr", include_str!("../assets/i18n/fr.json")),
    ("it", include_str!("../assets/i18n/it.json")),
    ("ja", include_str!("../assets/i18n/ja.json")),
    ("ko", include_str!("../assets/i18n/ko.json")),
    ("pl", include_str!("../assets/i18n/pl.json")),
    ("ptbr", include_str!("../assets/i18n/ptbr.json")),
    ("ru", include_str!("../assets/i18n/ru.json")),
    ("tr", include_str!("../assets/i18n/tr.json")),
    ("zhcn", include_str!("../assets/i18n/zhcn.json")),
    ("zhtw", include_str!("../assets/i18n/zhtw.json")),
];

fn parse(code: &str) -> HashMap<String, String> {
    let raw = RAW
        .iter()
        .find(|(c, _)| *c == code)
        .map(|(_, r)| *r)
        .or_else(|| RAW.iter().find(|(c, _)| *c == "en").map(|(_, r)| *r))
        .expect("embedded i18n");
    let value: serde_json::Value = serde_json::from_str(raw).expect("valid i18n JSON");
    let mut map = HashMap::new();
    if let Some(obj) = value.as_object() {
        for (k, v) in obj {
            if let Some(s) = v.as_str() {
                map.insert(k.clone(), s.to_string());
            }
        }
    }
    map
}

/// Loaded dictionaries for the active language (+ English fallback).
pub struct I18n {
    pub lang: String,
    pub rtl: bool,
    dict: HashMap<String, String>,
    fallback: HashMap<String, String>,
}

impl I18n {
    pub fn new(lang: &str) -> Self {
        I18n {
            lang: lang.to_string(),
            rtl: lang_meta(lang).rtl,
            dict: parse(lang),
            fallback: parse("en"),
        }
    }

    /// Translate a key; English fallback, then the key itself.
    pub fn t<'a>(&'a self, key: &'a str) -> &'a str {
        if let Some(v) = self.dict.get(key) {
            return v.as_str();
        }
        if let Some(v) = self.fallback.get(key) {
            return v.as_str();
        }
        key
    }

    /// Translate with `{v}`-style placeholder replacement.
    pub fn tf(&self, key: &str, var: &str, value: &str) -> String {
        self.t(key).replace(&format!("{{{var}}}"), value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fifteen_languages_embedded() {
        assert_eq!(LANGS.len(), 15);
        for (_, raw) in RAW {
            let v: serde_json::Value = serde_json::from_str(raw).expect("valid JSON");
            assert!(v.as_object().map(|o| !o.is_empty()).unwrap_or(false));
        }
    }

    #[test]
    fn translation_and_fallback() {
        let en = I18n::new("en");
        assert_eq!(en.t("brand"), "OWEN");
        assert_eq!(en.t("missing_key_xyz"), "missing_key_xyz");
        let ar = I18n::new("ar");
        assert!(ar.rtl);
        assert_eq!(ar.t("brand"), "أوين");
        // {v} placeholder
        let title = ar.tf("uninstallTitle", "v", "12.50");
        assert!(title.contains("12.50"));
        assert!(!title.contains("{v}"));
    }
}
