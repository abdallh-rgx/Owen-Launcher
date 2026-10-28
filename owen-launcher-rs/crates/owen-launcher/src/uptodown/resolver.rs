#![forbid(unsafe_code)]
//! 1:1 port of `UptodownResolver.kt` (object `UptodownResolver`) — **URL
//! resolution only**.
//!
//! The hidden `WebView` of the original is replaced by
//! [`crate::http::HttpBackend`]: the download page is fetched directly and the
//! *same literal matching rules* of the original `CLICK_SCRIPT` /
//! `shouldOverrideUrlLoading` / download-listener trio are applied to the
//! HTML:
//!
//! 1. `#detail-download-button-xapk` → its `data-url` counts only when it
//!    contains `dw.uptodown.com` (`u.indexOf('dw.uptodown.com') >= 0`).
//! 2. `#detail-download-button` → the regular download button (`btn.click()`).
//! 3. Any URL starting with `https://dw.uptodown.com/dwn/` or
//!    `http://dw.uptodown.com/dwn/` is a direct download
//!    (`isDirectDownloadUrl`, used by `shouldOverrideUrlLoading` and the
//!    download listener).
//!
//! `UptodownResolver.downloadWithProgress` moved to
//! [`crate::uptodown::installer`] (as `download_with_progress`) together with
//! its constants, per the launcher-crate layout.
//!
//! The Cloudflare-turnstile retry loop of the original script (a JS runtime
//! concern with no HTTP equivalent) is covered by the raw-URL fallback scan
//! of step 3: if the page HTML already embeds the post-turnstile link, it is
//! found without executing JavaScript.

use super::api::UptodownApi;
use crate::http::HttpBackend;
use std::time::{Duration, Instant};

/// 1:1 port of the Kotlin `object UptodownResolver`.
pub struct UptodownResolver;

impl UptodownResolver {
    /// `private const val UA` (WebView `userAgentString`).
    pub const UA: &'static str = "Mozilla/5.0 (Linux; Android 13; Pixel 7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Mobile Safari/537.36";

    /// `private const val RESOLVE_TIMEOUT_MS` — the overall resolution
    /// deadline (the `mainHandler.postDelayed(timeout, …)` cap of the
    /// original). Enforced as a wall-clock deadline across the HTTP steps.
    pub const RESOLVE_TIMEOUT_MS: u64 = 45_000;

    /// Literal rule of `CLICK_SCRIPT`: only a `data-url` containing
    /// `dw.uptodown.com` is followed for the XAPK button.
    const DIRECT_DOWNLOAD_HOST: &str = "dw.uptodown.com";

    /// Literal rule of `CLICK_SCRIPT`: `document.getElementById('detail-download-button-xapk')`.
    const XAPK_BUTTON_ID: &str = "detail-download-button-xapk";

    /// Literal rule of `CLICK_SCRIPT`: `document.getElementById('detail-download-button')`.
    const DOWNLOAD_BUTTON_ID: &str = "detail-download-button";

    /// Native backend configured with [`UptodownResolver::RESOLVE_TIMEOUT_MS`].
    #[cfg(not(target_arch = "wasm32"))]
    pub fn default_backend() -> crate::http::ReqwestBackend {
        crate::http::ReqwestBackend::with_timeout_ms(Self::RESOLVE_TIMEOUT_MS)
    }

    /// `fun isDirectDownloadUrl(url: String): Boolean`.
    pub fn is_direct_download_url(url: &str) -> bool {
        url.starts_with("https://dw.uptodown.com/dwn/")
            || url.starts_with("http://dw.uptodown.com/dwn/")
    }

    /// `suspend fun resolveDirectUrl(context, versionId): String?` — the
    /// hidden-WebView resolution, ported to plain HTTP:
    ///
    /// 1. Load `UptodownApi.downloadPageUrl(versionId)` (the `wv.loadUrl`).
    /// 2. `CLICK_SCRIPT` step 1: the XAPK button — its `data-url` is used
    ///    only when it contains `dw.uptodown.com`; `location.href = u` is the
    ///    navigation the WebView would intercept
    ///    (`isDirectDownloadUrl` → return; otherwise fetch it and look at the
    ///    landing URL / body).
    /// 3. `CLICK_SCRIPT` step 2: the regular button — `btn.click()` navigates
    ///    to its `data-url`; fetch it (Referer = the download page, like the
    ///    WebView) and check the landing URL / body.
    /// 4. Fallback: any direct `dw.uptodown.com/dwn/` URL embedded in the
    ///    page HTML — what the download listener would have caught.
    /// 5. Nothing found (or the 45 s deadline passed) → `None`
    ///    (`cont.resume(null)` of the timeout path).
    pub fn resolve_direct_url(http: &dyn HttpBackend, version_id: i64) -> Option<String> {
        let deadline = Instant::now() + Duration::from_millis(Self::RESOLVE_TIMEOUT_MS);

        // wv.loadUrl(pageUrl)
        let page_url = UptodownApi::download_page_url(version_id);
        let page = http
            .get(
                &page_url,
                &[("User-Agent".to_string(), Self::UA.to_string())],
            )
            .ok()?;
        if page.status >= 400 {
            return None;
        }
        let html = page.text();

        // CLICK_SCRIPT step 1: the XAPK button.
        if let Some(url) = find_button_data_url(&html, Self::XAPK_BUTTON_ID) {
            if url.contains(Self::DIRECT_DOWNLOAD_HOST) {
                // location.href = u → shouldOverrideUrlLoading intercepts a
                // direct URL immediately.
                if Self::is_direct_download_url(&url) {
                    return Some(url);
                }
                if let Some(found) = Self::follow_url(http, &url, &page_url, deadline) {
                    return Some(found);
                }
            }
        }

        // CLICK_SCRIPT step 2: the regular download button.
        if let Some(url) = find_button_data_url(&html, Self::DOWNLOAD_BUTTON_ID) {
            if Self::is_direct_download_url(&url) {
                return Some(url);
            }
            if let Some(found) = Self::follow_url(http, &url, &page_url, deadline) {
                return Some(found);
            }
        }

        // Fallback: the download listener catches any dw.uptodown.com/dwn/
        // URL the page itself carries.
        find_direct_url(&html)
    }

    /// The navigation after `location.href = u` / `btn.click()`: fetch the
    /// target (the WebView sends the download page as Referer) and apply
    /// `shouldOverrideUrlLoading` to the landing URL, then scan the body for
    /// an embedded direct link.
    fn follow_url(
        http: &dyn HttpBackend,
        url: &str,
        referer: &str,
        deadline: Instant,
    ) -> Option<String> {
        if Instant::now() >= deadline {
            return None;
        }
        let response = http
            .get(
                url,
                &[
                    ("User-Agent".to_string(), Self::UA.to_string()),
                    ("Referer".to_string(), referer.to_string()),
                ],
            )
            .ok()?;
        // shouldOverrideUrlLoading: the redirect chain landed on a direct URL.
        if Self::is_direct_download_url(&response.url) {
            return Some(response.url);
        }
        // The interstitial page usually embeds the real link.
        find_direct_url(&response.text())
    }
}

/// `document.getElementById(id)` + its `data-url` attribute — the element is
/// located by its literal `id="…"` (double or single quoted, so the plain
/// button id never matches the xapk variant) and the attribute is read from
/// the surrounding tag.
fn find_button_data_url(html: &str, button_id: &str) -> Option<String> {
    let id_double = format!("id=\"{button_id}\"");
    let id_single = format!("id='{button_id}'");
    let id_pos = html
        .find(&id_double)
        .or_else(|| html.find(&id_single))?;
    let tag_start = html[..id_pos].rfind('<')?;
    let tag_end = id_pos + html[id_pos..].find('>')?;
    let tag = &html[tag_start..tag_end];
    extract_attr(tag, "data-url")
}

/// Read `attr="value"` / `attr='value'` / `attr=value` from one tag.
fn extract_attr(tag: &str, attr: &str) -> Option<String> {
    for quote in ['"', '\''] {
        let literal = format!("{attr}={quote}");
        if let Some(pos) = tag.find(&literal) {
            let start = pos + literal.len();
            let end = tag[start..].find(quote)? + start;
            return Some(decode_html_entities(&tag[start..end]));
        }
    }
    let literal = format!("{attr}=");
    if let Some(pos) = tag.find(&literal) {
        let start = pos + literal.len();
        let end = tag[start..]
            .find(char::is_whitespace)
            .map(|e| start + e)
            .unwrap_or(tag.len());
        if end > start {
            return Some(decode_html_entities(&tag[start..end]));
        }
    }
    None
}

/// Minimal entity decoding for URLs (`&amp;` → `&`).
fn decode_html_entities(value: &str) -> String {
    value.replace("&amp;", "&")
}

/// Find the first `https://dw.uptodown.com/dwn/…` / `http://dw.uptodown.com/dwn/…`
/// URL in a text, terminated by whitespace, a quote or an angle bracket —
/// the URL shape `shouldOverrideUrlLoading` / the download listener accept.
fn find_direct_url(text: &str) -> Option<String> {
    for prefix in ["https://dw.uptodown.com/dwn/", "http://dw.uptodown.com/dwn/"] {
        if let Some(pos) = text.find(prefix) {
            let rest = &text[pos..];
            let end = rest
                .find(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == '<' || c == '>')
                .unwrap_or(rest.len());
            return Some(decode_html_entities(&rest[..end]));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    struct MockBackend {
        pages: Vec<(String, crate::http::HttpResponse)>,
        hits: RefCell<Vec<String>>,
    }

    impl MockBackend {
        fn with(url: &str, status: u16, body: &str, final_url: &str) -> Self {
            Self {
                pages: vec![(
                    url.to_string(),
                    crate::http::HttpResponse {
                        status,
                        headers: vec![],
                        body: body.as_bytes().to_vec(),
                        url: final_url.to_string(),
                    },
                )],
                hits: RefCell::new(Vec::new()),
            }
        }

        fn get_hit(&self, index: usize) -> String {
            self.hits.borrow().get(index).cloned().unwrap_or_default()
        }
    }

    impl HttpBackend for MockBackend {
        fn get(
            &self,
            url: &str,
            _headers: &[(String, String)],
        ) -> Result<crate::http::HttpResponse, String> {
            self.hits.borrow_mut().push(url.to_string());
            for (pattern, response) in &self.pages {
                if url == pattern {
                    return Ok(response.clone());
                }
            }
            Err("404".to_string())
        }

        fn post(
            &self,
            _url: &str,
            _headers: &[(String, String)],
            _body: &[u8],
        ) -> Result<crate::http::HttpResponse, String> {
            unimplemented!()
        }

        fn get_streaming(
            &self,
            _url: &str,
            _headers: &[(String, String)],
            _chunk_size: usize,
            _on_chunk: &mut dyn FnMut(u16, &[u8], Option<u64>) -> bool,
        ) -> Result<crate::http::HttpResponse, String> {
            unimplemented!()
        }
    }

    #[test]
    fn is_direct_download_url_literal_rule() {
        assert!(UptodownResolver::is_direct_download_url(
            "https://dw.uptodown.com/dwn/abc"
        ));
        assert!(UptodownResolver::is_direct_download_url(
            "http://dw.uptodown.com/dwn/abc"
        ));
        assert!(!UptodownResolver::is_direct_download_url(
            "https://dw.uptodown.com/other/abc"
        ));
        assert!(!UptodownResolver::is_direct_download_url(
            "https://example.com/dwn/abc"
        ));
        assert!(!UptodownResolver::is_direct_download_url(""));
    }

    #[test]
    fn finds_button_data_url_in_tag() {
        let html = r#"<div><button id="detail-download-button-xapk" class="btn" data-url="https://dw.uptodown.com/dwn/XYZ">XAPK</button></div>"#;
        assert_eq!(
            find_button_data_url(html, "detail-download-button-xapk").as_deref(),
            Some("https://dw.uptodown.com/dwn/XYZ")
        );
        // The plain button id must NOT match the xapk button (quote-terminated
        // literal, like getElementById).
        assert_eq!(find_button_data_url(html, "detail-download-button"), None);
    }

    #[test]
    fn finds_button_data_url_with_attr_before_id_and_single_quotes() {
        let html = r#"<a data-url="https://fortnite.en.uptodown.com/post-download" id='detail-download-button'>Download</a>"#;
        assert_eq!(
            find_button_data_url(html, "detail-download-button").as_deref(),
            Some("https://fortnite.en.uptodown.com/post-download")
        );
    }

    #[test]
    fn decodes_amp_entities_in_data_url() {
        let html = r#"<button id="detail-download-button" data-url="https://dw.uptodown.com/dwn/a?x=1&amp;y=2">"#;
        assert_eq!(
            find_button_data_url(html, "detail-download-button").as_deref(),
            Some("https://dw.uptodown.com/dwn/a?x=1&y=2")
        );
    }

    #[test]
    fn find_direct_url_scans_text() {
        assert_eq!(
            find_direct_url(r#"var link = "https://dw.uptodown.com/dwn/ABC123";"#).as_deref(),
            Some("https://dw.uptodown.com/dwn/ABC123")
        );
        assert_eq!(
            find_direct_url("start http://dw.uptodown.com/dwn/Q'> end").as_deref(),
            Some("http://dw.uptodown.com/dwn/Q")
        );
        assert_eq!(find_direct_url("nothing here"), None);
    }

    #[test]
    fn resolve_via_xapk_button_data_url() {
        let page_url = UptodownApi::download_page_url(42);
        let html = r#"
            <button id="detail-download-button-xapk" data-url="https://dw.uptodown.com/dwn/XAPK1">xapk</button>
            <button id="detail-download-button" data-url="https://fortnite.en.uptodown.com/ignored">apk</button>
        "#;
        let backend = MockBackend::with(&page_url, 200, html, &page_url);
        assert_eq!(
            UptodownResolver::resolve_direct_url(&backend, 42).as_deref(),
            Some("https://dw.uptodown.com/dwn/XAPK1")
        );
        assert_eq!(backend.hits.borrow().len(), 1, "no follow-up fetch needed");
    }

    #[test]
    fn resolve_via_regular_button_landing_url() {
        let page_url = UptodownApi::download_page_url(7);
        let button_url = "https://fortnite.en.uptodown.com/android/post-download/7";
        let html = r#"<button id="detail-download-button" data-url="https://fortnite.en.uptodown.com/android/post-download/7">Download</button>"#;
        let mut backend = MockBackend::with(&page_url, 200, html, &page_url);
        backend.pages.push((
            button_url.to_string(),
            crate::http::HttpResponse {
                status: 200,
                headers: vec![],
                body: b"binary-ish".to_vec(),
                // The redirect chain landed on the direct URL — what
                // shouldOverrideUrlLoading would have intercepted.
                url: "https://dw.uptodown.com/dwn/LANDED".to_string(),
            },
        ));
        assert_eq!(
            UptodownResolver::resolve_direct_url(&backend, 7).as_deref(),
            Some("https://dw.uptodown.com/dwn/LANDED")
        );
        // The follow-up request went to the button's data-url.
        assert_eq!(backend.get_hit(1), button_url);
    }

    #[test]
    fn resolve_via_interstitial_body_link() {
        let page_url = UptodownApi::download_page_url(9);
        let button_url = "https://fortnite.en.uptodown.com/interstitial";
        let html = r#"<button id="detail-download-button" data-url="https://fortnite.en.uptodown.com/interstitial">Download</button>"#;
        let mut backend = MockBackend::with(&page_url, 200, html, &page_url);
        backend.pages.push((
            button_url.to_string(),
            crate::http::HttpResponse {
                status: 200,
                headers: vec![],
                body: br#"<html>Your download <a href="https://dw.uptodown.com/dwn/FROM_BODY">starts</a></html>"#.to_vec(),
                url: button_url.to_string(),
            },
        ));
        assert_eq!(
            UptodownResolver::resolve_direct_url(&backend, 9).as_deref(),
            Some("https://dw.uptodown.com/dwn/FROM_BODY")
        );
    }

    #[test]
    fn resolve_falls_back_to_raw_page_scan() {
        let page_url = UptodownApi::download_page_url(5);
        let html = r#"<script>window.__cfg = {url: "https://dw.uptodown.com/dwn/EMBEDDED"};</script>"#;
        let backend = MockBackend::with(&page_url, 200, html, &page_url);
        assert_eq!(
            UptodownResolver::resolve_direct_url(&backend, 5).as_deref(),
            Some("https://dw.uptodown.com/dwn/EMBEDDED")
        );
    }

    #[test]
    fn resolve_returns_none_on_missing_page_or_no_match() {
        // Page fetch fails.
        let backend = MockBackend::with("https://nowhere", 200, "", "");
        assert_eq!(UptodownResolver::resolve_direct_url(&backend, 1), None);
        // Page loads but has no buttons / direct URLs.
        let page_url = UptodownApi::download_page_url(2);
        let backend = MockBackend::with(&page_url, 200, "<html>plain</html>", &page_url);
        assert_eq!(UptodownResolver::resolve_direct_url(&backend, 2), None);
        // Non-200 page load (WebView error page has no buttons).
        let backend = MockBackend::with(&page_url, 503, "", &page_url);
        assert_eq!(UptodownResolver::resolve_direct_url(&backend, 2), None);
    }

    #[test]
    fn xapk_button_without_dw_marker_is_skipped() {
        // CLICK_SCRIPT: the xapk button counts only when its data-url contains
        // dw.uptodown.com — otherwise the regular button is used.
        let page_url = UptodownApi::download_page_url(11);
        let html = r#"
            <button id="detail-download-button-xapk" data-url="https://elsewhere.example/xapk">xapk</button>
            <button id="detail-download-button" data-url="https://dw.uptodown.com/dwn/REGULAR">apk</button>
        "#;
        let backend = MockBackend::with(&page_url, 200, html, &page_url);
        assert_eq!(
            UptodownResolver::resolve_direct_url(&backend, 11).as_deref(),
            Some("https://dw.uptodown.com/dwn/REGULAR")
        );
        assert_eq!(backend.hits.borrow().len(), 1);
    }
}
