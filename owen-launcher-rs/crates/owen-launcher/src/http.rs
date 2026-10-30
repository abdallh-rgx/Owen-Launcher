#![forbid(unsafe_code)]
//! HTTP abstraction shared by the Uptodown layer — the Rust replacement for
//! the `HttpURLConnection` / hidden-`WebView` calls of the Kotlin original.
//!
//! * native: [`ReqwestBackend`] (blocking `reqwest`, follows redirects like
//!   `HttpURLConnection` does by default).
//! * wasm:  [`EhttpBackend`] (browser `fetch` through `ehttp`; callback-based,
//!   so the synchronous trait methods report an error and real network access
//!   goes through [`fetch_async`]).
//!
//! [`fetch_async`] works on **both** targets and is the entry point for the
//! wasm half of the launcher.

#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;

/// One HTTP response, backend-agnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    /// Status code of the final response (after redirects, when the backend
    /// follows them).
    pub status: u16,
    /// Response headers as `(name, value)` pairs, order preserved.
    pub headers: Vec<(String, String)>,
    /// Response body. Empty for [`HttpBackend::get_streaming`] (the body is
    /// streamed out through the callback instead).
    pub body: Vec<u8>,
    /// The final URL the backend reached. Identical to the request URL when
    /// no redirect happened; differs when the backend followed redirects.
    /// `UptodownResolver` uses it to detect redirect landings on
    /// `dw.uptodown.com` (what `shouldOverrideUrlLoading` saw in the WebView
    /// port).
    pub url: String,
}

impl HttpResponse {
    /// First header matching `name` (case-insensitive).
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    /// Body as lossy UTF-8 text.
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}

/// The HTTP surface used by the Uptodown layer.
///
/// Every method is synchronous; on wasm use [`fetch_async`] instead (the
/// browser event loop cannot be blocked).
pub trait HttpBackend {
    /// Plain `GET` (the port of `UptodownApi.httpGet`'s `HttpURLConnection`).
    fn get(&self, url: &str, headers: &[(String, String)]) -> Result<HttpResponse, String>;

    /// Plain `POST` with a raw body.
    fn post(
        &self,
        url: &str,
        headers: &[(String, String)],
        body: &[u8],
    ) -> Result<HttpResponse, String>;

    /// Streamed `GET` for large downloads — the port of
    /// `UptodownResolver.downloadWithProgress`'s `HttpURLConnection` loop
    /// (`instanceFollowRedirects = true`).
    ///
    /// `on_chunk` is invoked once per body chunk with the final status code,
    /// the chunk itself and the `Content-Length` when the server sent one
    /// (in this order: `(status, chunk, total)`). Returning `false` from
    /// `on_chunk` aborts the transfer (the method then returns `Err`). The
    /// returned [`HttpResponse`] carries the final status/headers/url with an
    /// **empty** body.
    fn get_streaming(
        &self,
        url: &str,
        headers: &[(String, String)],
        chunk_size: usize,
        on_chunk: &mut dyn FnMut(u16, &[u8], Option<u64>) -> bool,
    ) -> Result<HttpResponse, String>;
}

/* ------------------------------ native ------------------------------ */

/// Blocking `reqwest` backend (native targets only).
///
/// Follows redirects (limit 10), mirroring the `HttpURLConnection` defaults of
/// the Kotlin original.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone)]
pub struct ReqwestBackend {
    client: reqwest::blocking::Client,
}

#[cfg(not(target_arch = "wasm32"))]
impl Default for ReqwestBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl ReqwestBackend {
    /// Client without an explicit timeout (like a browser fetch).
    pub fn new() -> Self {
        Self {
            client: reqwest::blocking::Client::new(),
        }
    }

    /// Client with a total request timeout (`UptodownApi.TIMEOUT_MS` /
    /// `UptodownResolver.RESOLVE_TIMEOUT_MS` shaped).
    pub fn with_timeout_ms(timeout_ms: u64) -> Self {
        Self::from_builder(
            reqwest::blocking::Client::builder().timeout(Duration::from_millis(timeout_ms)),
        )
    }

    /// Client with the connect timeout of the original `downloadWithProgress`
    /// (`CONNECT_TIMEOUT_MS`). The original's per-read timeout
    /// (`READ_TIMEOUT_MS`) has no equivalent on the blocking reqwest 0.12
    /// client (its `timeout` is a *total* cap that would kill long
    /// downloads), so it is not applied here.
    pub fn with_timeouts_ms(connect_ms: u64, _read_ms: u64) -> Self {
        Self::from_builder(
            reqwest::blocking::Client::builder()
                .connect_timeout(Duration::from_millis(connect_ms)),
        )
    }

    fn from_builder(builder: reqwest::blocking::ClientBuilder) -> Self {
        match builder.build() {
            Ok(client) => Self { client },
            Err(err) => {
                // Practically unreachable (no TLS feature fiddling here);
                // fall back to a default client rather than panicking.
                eprintln!("[http] reqwest client build failed: {err}; using default");
                Self {
                    client: reqwest::blocking::Client::new(),
                }
            }
        }
    }

    fn collect_headers(
        resp: &reqwest::blocking::Response,
    ) -> Vec<(String, String)> {
        resp.headers()
            .iter()
            .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
            .collect()
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl HttpBackend for ReqwestBackend {
    fn get(&self, url: &str, headers: &[(String, String)]) -> Result<HttpResponse, String> {
        let mut request = self.client.get(url);
        for (name, value) in headers {
            request = request.header(name, value);
        }
        let resp = request.send().map_err(|e| e.to_string())?;
        let status = resp.status().as_u16();
        let url = resp.url().to_string();
        let headers = Self::collect_headers(&resp);
        let body = resp.bytes().map_err(|e| e.to_string())?.to_vec();
        Ok(HttpResponse {
            status,
            headers,
            body,
            url,
        })
    }

    fn post(
        &self,
        url: &str,
        headers: &[(String, String)],
        body: &[u8],
    ) -> Result<HttpResponse, String> {
        let mut request = self.client.post(url).body(body.to_vec());
        for (name, value) in headers {
            request = request.header(name, value);
        }
        let resp = request.send().map_err(|e| e.to_string())?;
        let status = resp.status().as_u16();
        let url = resp.url().to_string();
        let headers = Self::collect_headers(&resp);
        let body = resp.bytes().map_err(|e| e.to_string())?.to_vec();
        Ok(HttpResponse {
            status,
            headers,
            body,
            url,
        })
    }

    fn get_streaming(
        &self,
        url: &str,
        headers: &[(String, String)],
        chunk_size: usize,
        on_chunk: &mut dyn FnMut(u16, &[u8], Option<u64>) -> bool,
    ) -> Result<HttpResponse, String> {
        use std::io::Read;

        let mut request = self.client.get(url);
        for (name, value) in headers {
            request = request.header(name, value);
        }
        let mut resp = request.send().map_err(|e| e.to_string())?;
        let status = resp.status().as_u16();
        let url = resp.url().to_string();
        let headers = Self::collect_headers(&resp);
        let total: Option<u64> = resp
            .headers()
            .get(reqwest::header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse().ok());

        let mut buffer = vec![0u8; chunk_size.max(1)];
        loop {
            let read = resp.read(&mut buffer).map_err(|e| e.to_string())?;
            if read == 0 {
                break;
            }
            if !on_chunk(status, &buffer[..read], total) {
                return Err("download aborted by caller".to_string());
            }
        }
        Ok(HttpResponse {
            status,
            headers,
            body: Vec::new(),
            url,
        })
    }
}

/* -------------------------------- wasm -------------------------------- */

/// `ehttp` backend (wasm targets only).
///
/// The browser `fetch` behind `ehttp` is callback-based: the synchronous
/// [`HttpBackend`] methods cannot block the single-threaded wasm event loop
/// and therefore report an error. Use [`fetch_async`] on wasm — it drives
/// `ehttp::fetch` directly.
#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone, Default)]
pub struct EhttpBackend;

#[cfg(target_arch = "wasm32")]
impl EhttpBackend {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(target_arch = "wasm32")]
impl HttpBackend for EhttpBackend {
    fn get(&self, _url: &str, _headers: &[(String, String)]) -> Result<HttpResponse, String> {
        Err("EhttpBackend: the wasm event loop cannot be blocked; use owen_launcher::http::fetch_async".to_string())
    }

    fn post(
        &self,
        _url: &str,
        _headers: &[(String, String)],
        _body: &[u8],
    ) -> Result<HttpResponse, String> {
        Err("EhttpBackend: the wasm event loop cannot be blocked; use owen_launcher::http::fetch_async".to_string())
    }

    fn get_streaming(
        &self,
        _url: &str,
        _headers: &[(String, String)],
        _chunk_size: usize,
        _on_chunk: &mut dyn FnMut(u16, &[u8], Option<u64>) -> bool,
    ) -> Result<HttpResponse, String> {
        Err("EhttpBackend: the wasm event loop cannot be blocked; use owen_launcher::http::fetch_async".to_string())
    }
}

/* ------------------------------ fetch_async ------------------------------ */

/// Asynchronous one-shot `GET` that works on **both** native and wasm.
///
/// * native: the blocking [`ReqwestBackend`] runs on a background thread.
/// * wasm:   `ehttp::fetch` (browser `fetch`) drives the request.
pub fn fetch_async(
    url: String,
    headers: Vec<(String, String)>,
    on_done: impl FnOnce(Result<HttpResponse, String>) + Send + 'static,
) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::thread::spawn(move || {
            let backend = ReqwestBackend::new();
            on_done(backend.get(&url, &headers));
        });
    }

    #[cfg(target_arch = "wasm32")]
    {
        let mut request = ehttp::Request::get(url);
        let borrowed: Vec<(&str, &str)> = headers
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        request.headers = ehttp::Headers::new(&borrowed);
        ehttp::fetch(request, move |result: ehttp::Result<ehttp::Response>| {
            match result {
                Ok(resp) => on_done(Ok(HttpResponse {
                    status: resp.status,
                    headers: resp.headers.headers,
                    body: resp.bytes,
                    url: resp.url,
                })),
                Err(err) => on_done(Err(err)),
            }
        });
    }
}

/// Asynchronous one-shot `POST` that works on **both** native and wasm.
///
/// native: the blocking [`ReqwestBackend`] runs on a background thread.
/// wasm:   `ehttp::fetch` (browser `fetch`) drives the request.
pub fn fetch_async_post(
    url: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    on_done: impl FnOnce(Result<HttpResponse, String>) + Send + 'static,
) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::thread::spawn(move || {
            let backend = ReqwestBackend::new();
            on_done(backend.post(&url, &headers, &body));
        });
    }

    #[cfg(target_arch = "wasm32")]
    {
        let mut request = ehttp::Request::post(url);
        let borrowed: Vec<(&str, &str)> = headers
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        request.headers = ehttp::Headers::new(&borrowed);
        request.body = body;
        ehttp::fetch(request, move |result: ehttp::Result<ehttp::Response>| {
            match result {
                Ok(resp) => on_done(Ok(HttpResponse {
                    status: resp.status,
                    headers: resp.headers.headers,
                    body: resp.bytes,
                    url: resp.url,
                })),
                Err(err) => on_done(Err(err)),
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;

    /// Minimal local HTTP server good enough for one GET (native only).
    fn serve_once(body: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 1024];
                let _ = stream.read(&mut buf); // request (ignored)
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.flush();
            }
        });
        format!("http://{addr}/hello")
    }

    #[test]
    fn http_response_header_and_text() {
        let resp = HttpResponse {
            status: 200,
            headers: vec![
                ("Content-Type".to_string(), "text/html".to_string()),
                ("X-Foo".to_string(), "bar".to_string()),
            ],
            body: b"<html>hi</html>".to_vec(),
            url: "https://example.com/".to_string(),
        };
        assert_eq!(resp.header("content-type"), Some("text/html"));
        assert_eq!(resp.header("CONTENT-TYPE"), Some("text/html"));
        assert_eq!(resp.header("missing"), None);
        assert_eq!(resp.text(), "<html>hi</html>");
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn reqwest_backend_get_against_local_server() {
        let url = serve_once("uptodown-ok");
        let backend = ReqwestBackend::new();
        let resp = backend.get(&url, &[("User-Agent".to_string(), "test".to_string())])
            .expect("local get");
        assert_eq!(resp.status, 200);
        assert_eq!(resp.text(), "uptodown-ok");
        assert_eq!(resp.url, url);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn fetch_async_native_get() {
        let url = serve_once("async-ok");
        let (tx, rx) = mpsc::channel();
        fetch_async(
            url,
            vec![("User-Agent".to_string(), "test".to_string())],
            move |result| {
                tx.send(result).unwrap();
            },
        );
        let resp = rx.recv_timeout(Duration::from_secs(10)).expect("callback").expect("get");
        assert_eq!(resp.status, 200);
        assert_eq!(resp.text(), "async-ok");
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn reqwest_backend_timeouts_constructible() {
        let _ = ReqwestBackend::with_timeout_ms(15_000);
        let _ = ReqwestBackend::with_timeouts_ms(30_000, 60_000);
    }
}
