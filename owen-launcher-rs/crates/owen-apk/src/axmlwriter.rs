//! Binary `AndroidManifest.xml` writer (AOSP binary XML) for the pure-Rust
//! Owen Launcher APK.
//!
//! Emits the standard AOSP layout — a `RES_XML_TYPE` file header wrapping a
//! UTF-8 `RES_STRING_POOL_TYPE` chunk, one `android` namespace pair and the
//! element chunks — for a manifest that byte-matches what the original app
//! needs so the Rust build installs as an *update* over the Kotlin one:
//!
//! * package `com.abdallhrgx.owenlauncher`, `versionCode 18` (`1.8.0`)
//! * the 9 original `uses-permission`s, `minSdk 24` / `targetSdk 34`
//! * an `android.app.NativeActivity` application with
//!   `android.app.lib_name` (the `libowen_app.so` eframe crate)
//! * the `InstallResultReceiver` dex receiver + its `INSTALL_RESULT`
//!   intent-filter.
//!
//! The string pool is deliberately parsed back with the EXISTING reader
//! ([`crate::axml::StringPool::parse_pool`]) in the roundtrip self-test —
//! if this writer and that reader ever disagree, the build breaks loudly.

use crate::axml::encode_string;
use std::collections::HashMap;

/* ------------------------------ constants ------------------------------- */

/// `RES_XML_TYPE` — the AXML file header chunk.
pub const RES_XML_TYPE: u16 = 0x0003;
/// `RES_STRING_POOL_TYPE` — the string pool chunk.
pub const RES_STRING_POOL_TYPE: u16 = 0x0001;
/// `RES_XML_START_NAMESPACE_TYPE`.
pub const RES_XML_START_NAMESPACE_TYPE: u16 = 0x0100;
/// `RES_XML_END_NAMESPACE_TYPE`.
pub const RES_XML_END_NAMESPACE_TYPE: u16 = 0x0101;
/// `RES_XML_START_ELEMENT_TYPE`.
pub const RES_XML_START_ELEMENT_TYPE: u16 = 0x0102;
/// `RES_XML_END_ELEMENT_TYPE`.
pub const RES_XML_END_ELEMENT_TYPE: u16 = 0x0103;

/// UTF-8 string pool flag (`ResStringPool::UTF8_FLAG`).
pub const UTF8_FLAG: u32 = 1 << 8;

/// `Res_value::TYPE_STRING`.
pub const TYPE_STRING: u8 = 0x03;
/// `Res_value::TYPE_INT_DEC`.
pub const TYPE_INT_DEC: u8 = 0x10;
/// `Res_value::TYPE_INT_HEX` (enum/flag values, like aapt emits them).
pub const TYPE_INT_HEX: u8 = 0x11;
/// `Res_value::TYPE_INT_BOOLEAN`.
pub const TYPE_INT_BOOLEAN: u8 = 0x12;

/// The original application id — kept so the Rust build updates it.
pub const PACKAGE: &str = "com.abdallhrgx.owenlauncher";
/// One past the Kotlin app's `versionCode 17` (`1.7.4`).
pub const VERSION_CODE: u32 = 18;
pub const VERSION_NAME: &str = "1.8.0";
pub const MIN_SDK: u32 = 24;
pub const TARGET_SDK: u32 = 34;
/// The original app's app label.
pub const APP_LABEL: &str = "Owen Launcher";
/// The `NativeActivity` meta-data pointing at `libowen_app.so`.
pub const META_LIB_NAME: &str = "android.app.lib_name";
/// The install-result receiver (the only dex class).
pub const INSTALL_RESULT_RECEIVER: &str =
    "com.abdallhrgx.owenlauncher.core.patch.InstallResultReceiver";
/// Its intent-filter action.
pub const ACTION_INSTALL_RESULT: &str = "com.abdallhrgx.owenlauncher.INSTALL_RESULT";

/// The 9 original uses-permissions (KILL_BACKGROUND_PROCESSES dropped with
/// the Kotlin kill-switch going away).
pub const PERMISSIONS: [&str; 9] = [
    "android.permission.INTERNET",
    "android.permission.ACCESS_NETWORK_STATE",
    "android.permission.QUERY_ALL_PACKAGES",
    "android.permission.REQUEST_INSTALL_PACKAGES",
    "android.permission.FOREGROUND_SERVICE",
    "android.permission.FOREGROUND_SERVICE_DATA_SYNC",
    "android.permission.POST_NOTIFICATIONS",
    "android.permission.REQUEST_IGNORE_BATTERY_OPTIMIZATIONS",
    "android.permission.WAKE_LOCK",
];

const ANDROID_PREFIX: &str = "android";
const ANDROID_URI: &str = "http://schemas.android.com/apk/res/android";

/* --------------------------- the element model --------------------------- */

/// One attribute value — typed exactly the way aapt2 compiles it from text.
#[derive(Debug, Clone, PartialEq)]
pub enum AttrValue {
    /// `TYPE_STRING` (data = string index, rawValue = same index).
    Str(String),
    /// `TYPE_INT_BOOLEAN` (`0xFFFFFFFF` = true).
    Bool(bool),
    /// `TYPE_INT_DEC` with the original raw text kept (aapt-style:
    /// `(type 0x10)0x12 (Raw: "18")`).
    IntDec(u32, &'static str),
    /// `TYPE_INT_HEX` for enum/flag attributes (launchMode, configChanges,
    /// windowSoftInputMode) with the raw flag string kept.
    IntHex(u32, &'static str),
}

/// One attribute: name + (android namespace?) + typed value.
#[derive(Debug, Clone)]
pub struct Attr {
    /// In the `android` namespace (`ns` = the URI index) or none (`-1`).
    pub android: bool,
    pub name: &'static str,
    pub value: AttrValue,
}

/// One XML element (name + attrs + children — no text nodes needed).
#[derive(Debug, Clone)]
pub struct Element {
    pub name: &'static str,
    pub attrs: Vec<Attr>,
    pub children: Vec<Element>,
}

impl Element {
    pub fn new(name: &'static str, attrs: Vec<Attr>) -> Self {
        Element { name, attrs, children: Vec::new() }
    }

    pub fn child(mut self, el: Element) -> Self {
        self.children.push(el);
        self
    }
}

fn a_str(name: &'static str, value: &str) -> Attr {
    Attr { android: true, name, value: AttrValue::Str(value.to_string()) }
}

fn a_bool(name: &'static str, v: bool) -> Attr {
    Attr { android: true, name, value: AttrValue::Bool(v) }
}

fn a_int(name: &'static str, v: u32, raw: &'static str) -> Attr {
    Attr { android: true, name, value: AttrValue::IntDec(v, raw) }
}

fn a_hex(name: &'static str, v: u32, raw: &'static str) -> Attr {
    Attr { android: true, name, value: AttrValue::IntHex(v, raw) }
}

/* ------------------------------ the writer ------------------------------- */

struct Pool {
    strings: Vec<String>,
    index: HashMap<String, u32>,
}

impl Pool {
    fn new() -> Self {
        Pool { strings: Vec::new(), index: HashMap::new() }
    }

    fn intern(&mut self, s: &str) -> u32 {
        if let Some(i) = self.index.get(s) {
            return *i;
        }
        let i = self.strings.len() as u32;
        self.strings.push(s.to_string());
        self.index.insert(s.to_string(), i);
        i
    }

    fn idx(&self, s: &str) -> i32 {
        self.index[s] as i32
    }
}

/// Collect every string the tree references (names, attr names, values and
/// raw texts) into the pool, insertion-ordered and deduplicated. The SORTED
/// flag is intentionally NOT set and there are no styles.
fn collect(el: &Element, pool: &mut Pool) {
    pool.intern(el.name);
    for attr in &el.attrs {
        pool.intern(attr.name);
        match &attr.value {
            AttrValue::Str(s) => {
                pool.intern(s);
            }
            AttrValue::Bool(_) => {}
            AttrValue::IntDec(_, raw) | AttrValue::IntHex(_, raw) => {
                pool.intern(raw);
            }
        }
    }
    for child in &el.children {
        collect(child, pool);
    }
}

/// Serialize the pool chunk: 28-byte header, offsets array, then the UTF-8
/// strings (each `var8 charLen var8 byteLen bytes 0x00`), 4-byte aligned.
fn build_pool(pool: &Pool) -> Vec<u8> {
    let mut data: Vec<u8> = Vec::new();
    let mut offsets = Vec::with_capacity(pool.strings.len());
    for s in &pool.strings {
        offsets.push(data.len() as u32);
        data.extend_from_slice(&encode_string(s, true));
    }
    while data.len() % 4 != 0 {
        data.push(0);
    }
    let strings_start = 28 + 4 * pool.strings.len();
    let size = strings_start + data.len();
    let mut out = Vec::with_capacity(size);
    out.extend_from_slice(&RES_STRING_POOL_TYPE.to_le_bytes());
    out.extend_from_slice(&28u16.to_le_bytes()); // headerSize
    out.extend_from_slice(&(size as u32).to_le_bytes());
    out.extend_from_slice(&(pool.strings.len() as u32).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // styleCount
    out.extend_from_slice(&UTF8_FLAG.to_le_bytes()); // flags: UTF-8, not sorted
    out.extend_from_slice(&(strings_start as u32).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // stylesStart
    for off in offsets {
        out.extend_from_slice(&off.to_le_bytes());
    }
    out.extend_from_slice(&data);
    out
}

/// 16-byte chunk header shared by the namespace/element chunks.
fn push_chunk_header(out: &mut Vec<u8>, ctype: u16, size: usize) {
    out.extend_from_slice(&ctype.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes()); // headerSize
    out.extend_from_slice(&(size as u32).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // lineNumber
    out.extend_from_slice(&0xFFFF_FFFFu32.to_le_bytes()); // comment = none
}

fn emit_element(out: &mut Vec<u8>, el: &Element, pool: &Pool, android_ns: i32) {
    let size = 16 + 20 + 20 * el.attrs.len();
    push_chunk_header(out, RES_XML_START_ELEMENT_TYPE, size);
    out.extend_from_slice(&(-1i32).to_le_bytes()); // element ns (none)
    out.extend_from_slice(&pool.idx(el.name).to_le_bytes());
    out.extend_from_slice(&20u16.to_le_bytes()); // attributeStart
    out.extend_from_slice(&20u16.to_le_bytes()); // attributeSize
    out.extend_from_slice(&(el.attrs.len() as u16).to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // idIndex
    out.extend_from_slice(&0u16.to_le_bytes()); // classIndex
    out.extend_from_slice(&0u16.to_le_bytes()); // styleIndex
    for attr in &el.attrs {
        let ns = if attr.android { android_ns } else { -1 };
        let (data_type, data, raw): (u8, u32, i32) = match &attr.value {
            AttrValue::Str(s) => {
                let i = pool.idx(s) as u32;
                (TYPE_STRING, i, i as i32)
            }
            AttrValue::Bool(b) => {
                (TYPE_INT_BOOLEAN, if *b { 0xFFFF_FFFF } else { 0 }, -1)
            }
            AttrValue::IntDec(v, raw) => (TYPE_INT_DEC, *v, pool.idx(raw)),
            AttrValue::IntHex(v, raw) => (TYPE_INT_HEX, *v, pool.idx(raw)),
        };
        out.extend_from_slice(&ns.to_le_bytes());
        out.extend_from_slice(&pool.idx(attr.name).to_le_bytes());
        out.extend_from_slice(&raw.to_le_bytes());
        out.extend_from_slice(&8u16.to_le_bytes()); // Res_value size
        out.push(0u8); // res0
        out.push(data_type);
        out.extend_from_slice(&data.to_le_bytes());
    }
    for child in &el.children {
        emit_element(out, child, pool, android_ns);
    }
    // END_ELEMENT: 16-byte header + ns + name.
    push_chunk_header(out, RES_XML_END_ELEMENT_TYPE, 24);
    out.extend_from_slice(&(-1i32).to_le_bytes());
    out.extend_from_slice(&pool.idx(el.name).to_le_bytes());
}

/// Serialize an element tree to a complete binary AXML file.
pub fn emit(root: &Element) -> Vec<u8> {
    let mut pool = Pool::new();
    pool.intern(ANDROID_PREFIX);
    let android_ns_uri = pool.intern(ANDROID_URI);
    collect(root, &mut pool);
    let pool_chunk = build_pool(&pool);
    let android_ns = android_ns_uri as i32;

    let mut body = Vec::new();
    // RES_XML_START_NAMESPACE: prefix "android" + the res/android URI.
    push_chunk_header(&mut body, RES_XML_START_NAMESPACE_TYPE, 24);
    body.extend_from_slice(&pool.idx(ANDROID_PREFIX).to_le_bytes());
    body.extend_from_slice(&android_ns.to_le_bytes());
    emit_element(&mut body, root, &pool, android_ns);
    push_chunk_header(&mut body, RES_XML_END_NAMESPACE_TYPE, 24);
    body.extend_from_slice(&pool.idx(ANDROID_PREFIX).to_le_bytes());
    body.extend_from_slice(&android_ns.to_le_bytes());

    let total = 8 + pool_chunk.len() + body.len();
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(&RES_XML_TYPE.to_le_bytes());
    out.extend_from_slice(&8u16.to_le_bytes()); // headerSize
    out.extend_from_slice(&(total as u32).to_le_bytes());
    out.extend_from_slice(&pool_chunk);
    out.extend_from_slice(&body);
    out
}

/* ------------------------------ the manifest ----------------------------- */

/// `ActivityInfo.LAUNCH_SINGLE_TASK`.
const LAUNCH_MODE_SINGLE_TASK: u32 = 2;
/// `keyboardHidden|orientation|screenLayout|screenSize|smallestScreenSize`
/// (`0x20|0x80|0x100|0x400|0x800`).
const CONFIG_CHANGES_ALL: u32 = 0x20 | 0x80 | 0x100 | 0x400 | 0x800;
/// `WindowManager.LayoutParams.SOFT_INPUT_ADJUST_RESIZE`.
const SOFT_INPUT_ADJUST_RESIZE: u32 = 0x10;

/// The `AndroidManifest.xml` element tree of the Rust launcher.
///
/// `lib_name` is the `android.app.lib_name` meta-data value — the name
/// `NativeActivity` loads as `lib<lib_name>.so` (must match the packed
/// `lib/arm64-v8a/` entry, e.g. `owen_app` → `libowen_app.so`).
pub fn manifest_tree(lib_name: &str) -> Element {
    // <uses-permission android:name="…"/> ×9
    let permissions: Vec<Element> = PERMISSIONS
        .iter()
        .map(|p| Element::new("uses-permission", vec![a_str("name", p)]))
        .collect();

    let mut manifest = Element::new(
        "manifest",
        vec![
            Attr { android: false, name: "package", value: AttrValue::Str(PACKAGE.to_string()) },
            a_int("versionCode", VERSION_CODE, "18"),
            a_str("versionName", VERSION_NAME),
        ],
    );
    for p in permissions {
        manifest = manifest.child(p);
    }
    manifest = manifest.child(Element::new(
        "uses-sdk",
        vec![
            a_int("minSdkVersion", MIN_SDK, "24"),
            a_int("targetSdkVersion", TARGET_SDK, "34"),
        ],
    ));

    let activity = Element::new(
        "activity",
        vec![
            a_str("name", "android.app.NativeActivity"),
            a_bool("exported", true),
            a_hex("launchMode", LAUNCH_MODE_SINGLE_TASK, "singleTask"),
            a_hex(
                "configChanges",
                CONFIG_CHANGES_ALL,
                "orientation|screenSize|keyboardHidden|smallestScreenSize|screenLayout",
            ),
            a_hex("windowSoftInputMode", SOFT_INPUT_ADJUST_RESIZE, "adjustResize"),
        ],
    )
    .child(Element::new(
        "meta-data",
        vec![a_str("name", META_LIB_NAME), a_str("value", lib_name)],
    ))
    .child(
        Element::new("intent-filter", vec![])
            .child(Element::new("action", vec![a_str("name", "android.intent.action.MAIN")]))
            .child(Element::new(
                "category",
                vec![a_str("name", "android.intent.category.LAUNCHER")],
            )),
    );

    let receiver = Element::new(
        "receiver",
        vec![a_str("name", INSTALL_RESULT_RECEIVER), a_bool("exported", false)],
    )
    .child(Element::new(
        "intent-filter",
        vec![],
    )
    .child(Element::new("action", vec![a_str("name", ACTION_INSTALL_RESULT)])));

    let application = Element::new(
        "application",
        vec![
            a_str("label", APP_LABEL),
            a_bool("hasCode", true),
            a_bool("extractNativeLibs", true),
            a_bool("allowBackup", true),
            a_bool("supportsRtl", true),
            a_bool("debuggable", false),
        ],
    )
    .child(activity)
    .child(receiver);

    manifest.child(application)
}

/// Serialize the launcher's `AndroidManifest.xml` (binary AXML).
pub fn build_manifest(lib_name: &str) -> Vec<u8> {
    emit(&manifest_tree(lib_name))
}

/* --------------------------- roundtrip self-test -------------------------- */

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axml::StringPool;

    fn u16_at(d: &[u8], off: usize) -> u16 {
        u16::from_le_bytes([d[off], d[off + 1]])
    }
    fn u32_at(d: &[u8], off: usize) -> u32 {
        u32::from_le_bytes([d[off], d[off + 1], d[off + 2], d[off + 3]])
    }
    fn i32_at(d: &[u8], off: usize) -> i32 {
        i32::from_le_bytes([d[off], d[off + 1], d[off + 2], d[off + 3]])
    }

    /// A parsed START_ELEMENT chunk (pool indexes resolved to strings).
    struct StartEl<'a> {
        name: &'a str,
        attrs: Vec<(&'a str, u8, u32, Option<&'a str>, bool)>, // (name, type, data, raw, android_ns)
    }

    /// Walk every chunk, parse elements with typed attributes, and assert the
    /// chunk chain covers the file exactly (a malformed size anywhere breaks).
    fn walk<'a>(data: &'a [u8], pool: &'a StringPool) -> (Vec<StartEl<'a>>, usize) {
        assert_eq!(u16_at(data, 0), RES_XML_TYPE, "file header type");
        assert_eq!(u16_at(data, 2), 8, "file header size");
        assert_eq!(u32_at(data, 4) as usize, data.len(), "file header total size");
        assert_eq!(u16_at(data, 8), RES_STRING_POOL_TYPE, "first chunk is the pool");
        let pool_size = u32_at(data, 12) as usize;
        assert_eq!(pool_size % 4, 0, "pool chunk is 4-aligned");

        let mut elements = Vec::new();
        let mut start_count = 0usize;
        let mut end_count = 0usize;
        let mut ns_count = 0usize;
        let mut off = 8 + pool_size;
        while off < data.len() {
            let ctype = u16_at(data, off);
            let csize = u32_at(data, off + 4) as usize;
            assert!(csize >= 16 && off + csize <= data.len(), "chunk {ctype:#x} size");
            match ctype {
                RES_XML_START_NAMESPACE_TYPE | RES_XML_END_NAMESPACE_TYPE => {
                    assert_eq!(csize, 24, "namespace chunk size");
                    assert_eq!(i32_at(data, off + 16), pool_idx(pool, "android"));
                    assert_eq!(
                        i32_at(data, off + 20),
                        pool_idx(pool, ANDROID_URI)
                    );
                    ns_count += 1;
                }
                RES_XML_START_ELEMENT_TYPE => {
                    let name = pool_str(pool, i32_at(data, off + 20));
                    let attr_start = u16_at(data, off + 24);
                    let attr_size = u16_at(data, off + 26);
                    let attr_count = u16_at(data, off + 28) as usize;
                    assert_eq!(attr_start, 20, "attributeStart");
                    assert_eq!(attr_size, 20, "attributeSize");
                    assert_eq!(csize, 16 + 20 + 20 * attr_count, "start element size");
                    let mut attrs = Vec::with_capacity(attr_count);
                    for i in 0..attr_count {
                        let a = off + 36 + 20 * i;
                        let ns = i32_at(data, a);
                        let aname = pool_str(pool, i32_at(data, a + 4));
                        let raw = i32_at(data, a + 8);
                        let vsize = u16_at(data, a + 12);
                        let res0 = data[a + 14];
                        let vtype = data[a + 15];
                        let vdata = u32_at(data, a + 16);
                        assert_eq!(vsize, 8, "Res_value size");
                        assert_eq!(res0, 0, "Res_value res0");
                        attrs.push((
                            aname,
                            vtype,
                            vdata,
                            if raw >= 0 { Some(pool_str(pool, raw)) } else { None },
                            ns == pool_idx(pool, ANDROID_URI),
                        ));
                    }
                    elements.push(StartEl { name, attrs });
                    start_count += 1;
                }
                RES_XML_END_ELEMENT_TYPE => {
                    assert_eq!(csize, 24, "end element size");
                    assert_eq!(i32_at(data, off + 16), -1, "end element ns");
                    end_count += 1;
                }
                other => panic!("unexpected chunk {other:#x} at {off}"),
            }
            off += csize;
        }
        assert_eq!(off, data.len(), "chunks cover the file exactly");
        assert_eq!(start_count, end_count, "balanced start/end elements");
        assert_eq!(ns_count, 2, "one namespace pair");
        (elements, start_count)
    }

    fn pool_idx(pool: &StringPool, s: &str) -> i32 {
        pool.strings
            .iter()
            .position(|x| x == s)
            .map(|i| i as i32)
            .unwrap_or_else(|| panic!("string {s:?} missing from pool"))
    }

    fn pool_str(pool: &StringPool, i: i32) -> &str {
        &pool.strings[i as usize]
    }

    #[test]
    fn manifest_roundtrip_with_existing_parser() {
        let data = build_manifest("owen_app");

        // --- 1. string pool parses back with the EXISTING reader ---
        let pool_size = u32_at(&data, 12) as usize;
        let pool = StringPool::parse_pool(&data[8..8 + pool_size])
            .expect("the existing axml reader must parse our pool");
        assert!(pool.is_utf8(), "UTF-8 flag");
        assert_eq!(pool.style_count, 0, "no styles");
        assert_eq!(pool.strings_start, 28 + 4 * pool.strings.len());

        let strings: Vec<&str> = pool.strings.iter().map(|s| s.as_str()).collect();
        // SORTED flag (0x01) deliberately NOT set, UTF-8 (0x100) set.
        assert_eq!(pool.flags, UTF8_FLAG, "UTF-8 pool, unsorted, no styles");
        // manifest identity
        for expected in [
            PACKAGE,
            VERSION_NAME,
            "18",
            APP_LABEL,
            META_LIB_NAME,
            "owen_app",
            "android.app.NativeActivity",
            "singleTask",
            "adjustResize",
            "orientation|screenSize|keyboardHidden|smallestScreenSize|screenLayout",
            INSTALL_RESULT_RECEIVER,
            ACTION_INSTALL_RESULT,
            "android.intent.action.MAIN",
            "android.intent.category.LAUNCHER",
            "24",
            "34",
        ] {
            assert!(
                strings.contains(&expected),
                "pool must contain {expected:?} (roundtrip broken)"
            );
        }
        // all 9 permissions
        for perm in PERMISSIONS {
            assert!(
                strings.contains(&perm),
                "pool must contain permission {perm:?}"
            );
        }
        assert_eq!(
            strings.iter().filter(|s| s.starts_with("android.permission.")).count(),
            9,
            "exactly the 9 original permissions"
        );

        // --- 2. structural walk: elements, typed attrs ---
        let (elements, count) = walk(&data, &pool);
        assert_eq!(count, 20, "elements: manifest + 9 perms + sdk + app + activity + meta + filter + action + category + receiver + filter + action");

        let manifest = &elements[0];
        assert_eq!(manifest.name, "manifest");
        let package_attr = manifest
            .attrs
            .iter()
            .find(|(n, ..)| *n == "package")
            .expect("package attr");
        assert!(!package_attr.4, "package has no namespace");
        assert_eq!(package_attr.1, TYPE_STRING);
        assert_eq!(pool_str(&pool, package_attr.2 as i32), PACKAGE);

        let version_code = manifest
            .attrs
            .iter()
            .find(|(n, ..)| *n == "versionCode")
            .expect("versionCode attr");
        assert_eq!(version_code.1, TYPE_INT_DEC);
        assert_eq!(version_code.2, 18);
        assert_eq!(version_code.3, Some("18"));

        // the manifest children in order
        let names: Vec<&str> = elements.iter().map(|e| e.name).collect();
        assert_eq!(
            names,
            vec![
                "manifest",
                "uses-permission",
                "uses-permission",
                "uses-permission",
                "uses-permission",
                "uses-permission",
                "uses-permission",
                "uses-permission",
                "uses-permission",
                "uses-permission",
                "uses-sdk",
                "application",
                "activity",
                "meta-data",
                "intent-filter",
                "action",
                "category",
                "receiver",
                "intent-filter",
                "action",
            ]
        );

        // uses-sdk: min 24 / target 34
        let uses_sdk = &elements[10];
        let min_sdk = uses_sdk.attrs.iter().find(|(n, ..)| *n == "minSdkVersion").unwrap();
        assert_eq!((min_sdk.1, min_sdk.2, min_sdk.3), (TYPE_INT_DEC, 24, Some("24")));
        let target = uses_sdk.attrs.iter().find(|(n, ..)| *n == "targetSdkVersion").unwrap();
        assert_eq!((target.1, target.2, target.3), (TYPE_INT_DEC, 34, Some("34")));

        // application booleans
        let application = &elements[11];
        for (name, value) in [
            ("hasCode", 0xFFFF_FFFFu32),
            ("extractNativeLibs", 0xFFFF_FFFF),
            ("allowBackup", 0xFFFF_FFFF),
            ("supportsRtl", 0xFFFF_FFFF),
            ("debuggable", 0),
        ] {
            let attr = application.attrs.iter().find(|(n, ..)| *n == name).unwrap();
            assert_eq!((attr.1, attr.2), (TYPE_INT_BOOLEAN, value), "application {name}");
        }

        // activity: name + exported + launchMode singleTask + configChanges + windowSoftInputMode
        let activity = &elements[12];
        let activity_name = activity.attrs.iter().find(|(n, ..)| *n == "name").unwrap();
        assert_eq!(activity_name.1, TYPE_STRING);
        assert_eq!(pool_str(&pool, activity_name.2 as i32), "android.app.NativeActivity");
        let exported = activity.attrs.iter().find(|(n, ..)| *n == "exported").unwrap();
        assert_eq!((exported.1, exported.2), (TYPE_INT_BOOLEAN, 0xFFFF_FFFF));
        let launch = activity.attrs.iter().find(|(n, ..)| *n == "launchMode").unwrap();
        assert_eq!((launch.1, launch.2, launch.3), (TYPE_INT_HEX, 2, Some("singleTask")));
        let config = activity.attrs.iter().find(|(n, ..)| *n == "configChanges").unwrap();
        assert_eq!(
            (config.1, config.2),
            (TYPE_INT_HEX, 0x20 | 0x80 | 0x100 | 0x400 | 0x800)
        );
        assert_eq!(
            config.3,
            Some("orientation|screenSize|keyboardHidden|smallestScreenSize|screenLayout")
        );
        let soft = activity
            .attrs
            .iter()
            .find(|(n, ..)| *n == "windowSoftInputMode")
            .unwrap();
        assert_eq!((soft.1, soft.2, soft.3), (TYPE_INT_HEX, 0x10, Some("adjustResize")));

        // meta-data: android.app.lib_name → owen_app
        let meta = &elements[13];
        let meta_name = meta.attrs.iter().find(|(n, ..)| *n == "name").unwrap();
        assert_eq!(pool_str(&pool, meta_name.2 as i32), META_LIB_NAME);
        let meta_value = meta.attrs.iter().find(|(n, ..)| *n == "value").unwrap();
        assert_eq!(meta_value.1, TYPE_STRING);
        assert_eq!(pool_str(&pool, meta_value.2 as i32), "owen_app");

        // receiver
        let receiver = &elements[17];
        let receiver_name = receiver.attrs.iter().find(|(n, ..)| *n == "name").unwrap();
        assert_eq!(pool_str(&pool, receiver_name.2 as i32), INSTALL_RESULT_RECEIVER);
        let receiver_exported = receiver.attrs.iter().find(|(n, ..)| *n == "exported").unwrap();
        assert_eq!((receiver_exported.1, receiver_exported.2), (TYPE_INT_BOOLEAN, 0));

        // action names
        let main_action = elements[15]
            .attrs
            .iter()
            .find(|(n, ..)| *n == "name")
            .map(|(_, _, data, _, _)| *data as i32)
            .unwrap();
        assert_eq!(pool_str(&pool, main_action), "android.intent.action.MAIN");
        let install_action = elements[19]
            .attrs
            .iter()
            .find(|(n, ..)| *n == "name")
            .map(|(_, _, data, _, _)| *data as i32)
            .unwrap();
        assert_eq!(pool_str(&pool, install_action), ACTION_INSTALL_RESULT);
    }

    #[test]
    fn lib_name_is_honored() {
        let data = build_manifest("owenapp");
        let pool_size = u32_at(&data, 12) as usize;
        let pool = StringPool::parse_pool(&data[8..8 + pool_size]).unwrap();
        assert!(pool.strings.iter().any(|s| s == "owenapp"));
        assert!(!pool.strings.iter().any(|s| s == "owen_app"));
    }
}
