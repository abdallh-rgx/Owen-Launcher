#![forbid(unsafe_code)]
//! 1:1 port of `UptodownApi.kt` (object `UptodownApi`).
//!
//! The `HttpURLConnection` calls run through [`crate::http::HttpBackend`]
//! (blocking reqwest on native). The `@Volatile cachedVersions` singleton
//! field becomes the per-instance cache of [`UptodownApi`].

use super::version::UptodownVersion;
use crate::http::HttpBackend;

/// Uptodown "Fortnite" app endpoints (`UptodownApi.kt`).
pub struct UptodownApi {
    /// `@Volatile private var cachedVersions: List<UptodownVersion>?`
    cached_versions: Option<Vec<UptodownVersion>>,
}

impl Default for UptodownApi {
    fn default() -> Self {
        Self::new()
    }
}

impl UptodownApi {
    pub fn new() -> Self {
        Self {
            cached_versions: None,
        }
    }

    /// `const val HOST` (public in the Kotlin original).
    pub const HOST: &'static str = "https://fortnite.en.uptodown.com";

    /// `private const val APP_CODE`.
    const APP_CODE: &str = "725251";

    /// `private const val UA`.
    const UA: &str = "Mozilla/5.0 (Linux; Android 13; Pixel 7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Mobile Safari/537.36";

    /// `private const val REFERER`.
    const REFERER: &str = "https://fortnite.en.uptodown.com/android/versions";

    /// `private const val TIMEOUT_MS` — the per-request timeout of
    /// `httpGet`. Native callers get a matching backend via
    /// [`UptodownApi::default_backend`].
    pub const TIMEOUT_MS: u64 = 15_000;

    /// `private const val MAX_PAGES`.
    const MAX_PAGES: u32 = 120;

    /// Native backend configured with [`UptodownApi::TIMEOUT_MS`]
    /// (`connectTimeout`/`readTimeout` of the original `httpGet`).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn default_backend() -> crate::http::ReqwestBackend {
        crate::http::ReqwestBackend::with_timeout_ms(Self::TIMEOUT_MS)
    }

    /// `fun clearCache()`.
    pub fn clear_cache(&mut self) {
        self.cached_versions = None;
    }

    /// `fun downloadPageUrl(versionId: Long): String`.
    pub fn download_page_url(version_id: i64) -> String {
        format!("{}/android/download/{version_id}", Self::HOST)
    }

    /// `suspend fun httpGet(url: String): String?` — same headers, same
    /// 200-only acceptance, `None` on any failure.
    pub fn http_get(http: &dyn HttpBackend, url: &str) -> Option<String> {
        let headers = Self::api_headers();
        let response = http.get(url, &headers).ok()?;
        if response.status != 200 {
            return None;
        }
        Some(response.text())
    }

    fn api_headers() -> Vec<(String, String)> {
        vec![
            ("User-Agent".to_string(), Self::UA.to_string()),
            ("X-Requested-With".to_string(), "XMLHttpRequest".to_string()),
            ("Referer".to_string(), Self::REFERER.to_string()),
            (
                "Accept".to_string(),
                "application/json, text/plain, */*".to_string(),
            ),
        ]
    }

    /// `suspend fun fetchAllVersions(onPage: (pageIndex, totalPages) -> Unit)` —
    /// same pagination (1..=MAX_PAGES), same `break` conditions, same item
    /// filtering.
    ///
    /// Note: like the Kotlin original, the second callback argument is fed
    /// `all.size` (the accumulated count), not a real page total.
    pub fn fetch_all_versions(
        &mut self,
        http: &dyn HttpBackend,
        mut on_page: impl FnMut(usize, usize),
    ) -> Vec<UptodownVersion> {
        if let Some(cached) = &self.cached_versions {
            return cached.clone();
        }
        let mut all: Vec<UptodownVersion> = Vec::new();
        for page in 1..=Self::MAX_PAGES {
            let url = format!(
                "{}/android/apps/{}/versions/{page}",
                Self::HOST,
                Self::APP_CODE
            );
            let body = match Self::http_get(http, &url) {
                Some(body) => body,
                None => break,
            };
            let data = match parse_versions_page(&body) {
                Some(data) => data,
                None => break,
            };
            if data.is_empty() {
                break;
            }
            for item in data {
                all.push(item);
            }
            on_page(page as usize, all.len());
        }
        self.cached_versions = Some(all.clone());
        all
    }

    /// `suspend fun findByCl(cl: Long): UptodownVersion?`.
    pub fn find_by_cl(&mut self, http: &dyn HttpBackend, cl: i64) -> Option<UptodownVersion> {
        let all = self.fetch_all_versions(http, |_, _| {});
        all.into_iter()
            .find(|version| {
                version
                    .version
                    .to_ascii_lowercase()
                    .contains(&format!("-cl-{cl}").to_ascii_lowercase())
                    || cl_suffix(&version.version) == Some(cl)
            })
    }

    /// `suspend fun findVersions(query: String): List<UptodownVersion>`.
    pub fn find_versions(&mut self, http: &dyn HttpBackend, query: &str) -> Vec<UptodownVersion> {
        let all = self.fetch_all_versions(http, |_, _| {});
        if query.trim().is_empty() {
            return all;
        }
        let needle = query.to_ascii_lowercase();
        all.into_iter()
            .filter(|version| version.version.to_ascii_lowercase().contains(&needle))
            .collect()
    }
}

/// `Regex("-(\\d+)-[Aa]ndroid$")` — the CL suffix of a version string
/// (`-<digits>-Android` / `-<digits>-android` at the very end). Returns the
/// captured digits, like `groupValues[1].toLongOrNull()`.
fn cl_suffix(version: &str) -> Option<i64> {
    let stem = version
        .strip_suffix("-Android")
        .or_else(|| version.strip_suffix("-android"))?;
    // The regex needs `-` + digits directly before the suffix.
    let idx = stem.rfind('-')?;
    let digits = &stem[idx + 1..];
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse::<i64>().ok()
}

/// The per-page JSON parsing of `fetchAllVersions`
/// (`success == 1` → `data` array → items), preserving the `optString` /
/// `optLong` defaults of the Kotlin original.
fn parse_versions_page(body: &str) -> Option<Vec<UptodownVersion>> {
    let root: serde_json::Value = serde_json::from_str(body).ok()?;
    let root = root.as_object()?;
    if opt_int(root, "success", 0) != 1 {
        return None;
    }
    let data = root.get("data")?.as_array()?.clone();
    let mut out = Vec::with_capacity(data.len());
    for item in &data {
        let Some(item) = item.as_object() else {
            continue;
        };
        // val versionUrl = item.optJSONObject("versionURL") ?: continue
        let Some(version_url) = item.get("versionURL").and_then(|v| v.as_object()) else {
            continue;
        };
        // val versionId = versionUrl.optLong("versionID", -1L); if (versionId <= 0L) continue
        let version_id = opt_i64(version_url, "versionID", -1);
        if version_id <= 0 {
            continue;
        }
        out.push(UptodownVersion {
            version: opt_string(item, "version"),
            version_id,
            // item.optString("kindFile", "apk")
            kind_file: opt_string_or(item, "kindFile", "apk"),
            sdk_version: opt_string(item, "sdkVersion"),
            last_update: opt_string(item, "lastUpdate"),
        });
    }
    Some(out)
}

/// `JSONObject.optInt(key, default)` for integer-ish values.
fn opt_int(map: &serde_json::Map<String, serde_json::Value>, key: &str, default: i64) -> i64 {
    match map.get(key) {
        Some(serde_json::Value::Number(n)) => n.as_i64().unwrap_or(default),
        Some(serde_json::Value::String(s)) => s.parse().unwrap_or(default),
        _ => default,
    }
}

/// `JSONObject.optLong(key, default)`.
fn opt_i64(map: &serde_json::Map<String, serde_json::Value>, key: &str, default: i64) -> i64 {
    opt_int(map, key, default)
}

/// `JSONObject.optString(key)` — `""` when missing.
fn opt_string(map: &serde_json::Map<String, serde_json::Value>, key: &str) -> String {
    opt_string_or(map, key, "")
}

/// `JSONObject.optString(key, default)`.
fn opt_string_or(
    map: &serde_json::Map<String, serde_json::Value>,
    key: &str,
    default: &str,
) -> String {
    match map.get(key) {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(serde_json::Value::Number(n)) => n.to_string(),
        Some(serde_json::Value::Bool(b)) => b.to_string(),
        _ => default.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    /* --------------------------- mock backend --------------------------- */

    struct MockBackend {
        hits: Cell<usize>,
    }

    impl HttpBackend for MockBackend {
        fn get(
            &self,
            url: &str,
            _headers: &[(String, String)],
        ) -> Result<crate::http::HttpResponse, String> {
            self.hits.set(self.hits.get() + 1);
            if url.contains("/versions/1") {
                Ok(crate::http::HttpResponse {
                    status: 200,
                    headers: vec![],
                    body: br#"{"success":1,"data":[
                        {"version":"13.40-CL-14009477-Android","versionURL":{"versionID":14009477},"kindFile":"apk","sdkVersion":"29","lastUpdate":"Oct 21, 2020"},
                        {"version":"14.60-CL-15534296-Android","versionURL":{"versionID":15534296},"kindFile":"xapk","sdkVersion":"30","lastUpdate":"Jun 8, 2021"},
                        {"version":"broken-no-versionURL","sdkVersion":"30"},
                        {"version":"zero-id","versionURL":{"versionID":0}},
                        {"version":"default-kind","versionURL":{"versionID":77}}
                    ]}"#
                    .to_vec(),
                    url: url.to_string(),
                })
            } else {
                // Page 2+: empty data array → stop, like the original.
                Ok(crate::http::HttpResponse {
                    status: 200,
                    headers: vec![],
                    body: br#"{"success":1,"data":[]}"#.to_vec(),
                    url: url.to_string(),
                })
            }
        }

        fn post(
            &self,
            _url: &str,
            _headers: &[(String, String)],
            _body: &[u8],
        ) -> Result<crate::http::HttpResponse, String> {
            unimplemented!("not used by UptodownApi")
        }

        fn get_streaming(
            &self,
            _url: &str,
            _headers: &[(String, String)],
            _chunk_size: usize,
            _on_chunk: &mut dyn FnMut(u16, &[u8], Option<u64>) -> bool,
        ) -> Result<crate::http::HttpResponse, String> {
            unimplemented!("not used by UptodownApi")
        }
    }

    /* ------------------------------ tests ------------------------------ */

    #[test]
    fn download_page_url_format() {
        assert_eq!(
            UptodownApi::download_page_url(1768085),
            "https://fortnite.en.uptodown.com/android/download/1768085"
        );
    }

    #[test]
    fn cl_suffix_regex_port() {
        assert_eq!(cl_suffix("13.40-CL-14009477-Android"), Some(14009477));
        assert_eq!(cl_suffix("13.40-CL-14009477-android"), Some(14009477));
        assert_eq!(cl_suffix("1-22-333-Android"), Some(333));
        assert_eq!(cl_suffix("13.40-CL-14009477"), None, "no suffix");
        assert_eq!(cl_suffix("13.40-CL-14009477-ANDROID"), None, "[Aa] only");
        assert_eq!(cl_suffix("13.40-CL--Android"), None, "empty digits");
        assert_eq!(cl_suffix("13.40-CL-x-Android"), None, "non-digit");
        assert_eq!(cl_suffix("99999999999999999999-Android"), None, "overflow");
    }

    #[test]
    fn parse_versions_page_keeps_defaults_and_filters() {
        let body = r#"{"success":1,"data":[
            {"version":"13.40-CL-1-Android","versionURL":{"versionID":42},"kindFile":"xapk"},
            {"version":"skip-me","versionURL":{"versionID":0}},
            {"version":"no-versionURL"}
        ]}"#;
        let parsed = parse_versions_page(body).expect("parses");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].version_id, 42);
        assert!(parsed[0].is_xapk());
        assert_eq!(parsed[0].sdk_version, "");
        assert_eq!(parsed[0].last_update, "");

        let default_kind = r#"{"success":1,"data":[{"version":"v","versionURL":{"versionID":7}}]}"#;
        let parsed = parse_versions_page(default_kind).expect("parses");
        assert_eq!(parsed[0].kind_file, "apk", "optString default");
    }

    #[test]
    fn parse_versions_page_rejects_bad_payloads() {
        assert!(parse_versions_page("not json").is_none());
        assert!(parse_versions_page(r#"{"success":0,"data":[]}"#).is_none());
        assert!(parse_versions_page(r#"{"success":1}"#).is_none());
    }

    #[test]
    fn fetch_all_versions_pagination_and_cache() {
        let http = MockBackend { hits: Cell::new(0) };
        let mut api = UptodownApi::new();
        let mut pages: Vec<(usize, usize)> = Vec::new();
        let all = api.fetch_all_versions(&http, |page, count| pages.push((page, count)));

        assert_eq!(all.len(), 3, "two broken entries filtered");
        assert_eq!(all[0].version, "13.40-CL-14009477-Android");
        assert_eq!(all[1].kind_file, "xapk");
        assert_eq!(all[2].kind_file, "apk", "kindFile default");
        // onPage(page, all.size) — the Kotlin quirk is preserved.
        assert_eq!(pages, vec![(1, 3)]);
        // Two HTTP hits: page 1 + page 2 (empty data → break).
        assert_eq!(http.hits.get(), 2);

        // Second call served from cache — no extra HTTP hits.
        let again = api.fetch_all_versions(&http, |_, _| {});
        assert_eq!(again.len(), 3);
        assert_eq!(http.hits.get(), 2);

        api.clear_cache();
        let _ = api.fetch_all_versions(&http, |_, _| {});
        assert_eq!(http.hits.get(), 4, "cache cleared → refetch");
    }

    #[test]
    fn find_by_cl_matches_contains_and_suffix() {
        let http = MockBackend { hits: Cell::new(0) };
        let mut api = UptodownApi::new();
        // "-CL-15534296" contains match (case-insensitive).
        let found = api
            .find_by_cl(&http, 15534296)
            .expect("contains -CL- match");
        assert_eq!(found.version_id, 15534296);
        // Regex suffix match: version "13.40-CL-14009477-Android" ends with
        // -14009477-Android.
        let found = api.find_by_cl(&http, 14009477).expect("suffix match");
        assert_eq!(found.version_id, 14009477);
        // No match.
        assert!(api.find_by_cl(&http, 999999).is_none());
    }

    #[test]
    fn find_versions_blank_and_filter() {
        let http = MockBackend { hits: Cell::new(0) };
        let mut api = UptodownApi::new();
        assert_eq!(api.find_versions(&http, "   ").len(), 3, "blank → all");
        assert_eq!(api.find_versions(&http, "13.40").len(), 1);
        assert_eq!(api.find_versions(&http, "cl-15534296").len(), 1, "ignoreCase");
        assert_eq!(api.find_versions(&http, "zzz").len(), 0);
    }
}
