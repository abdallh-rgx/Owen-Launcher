//! Fortnite version detection from binary images.
//!
//! Android builds embed the release string in two forms depending on the
//! engine era:
//!
//! * ASCII:  `++Fortnite+Release-13.40-CL-14009477`
//! * UTF-16LE (inside literal pools): same text with interleaved zero bytes.
//!
//! Old builds also embed an engine version such as
//! `++UE4+Release-4.23-CL-0` — the Fortnite token always wins.

/// A parsed Fortnite version.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct FortniteVersion {
    pub major: u32,
    pub minor: u32,
    pub cl: u64,
    /// The raw release string that was found.
    pub release_string: String,
}

impl std::fmt::Display for FortniteVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{:02}", self.major, self.minor)
    }
}

impl FortniteVersion {
    /// Season chapter/season pair for known versions (Chapter 1 started at
    /// 1.7-ish; the mapping below covers the mobile-relevant range).
    pub fn season(&self) -> (u32, u32) {
        let v = (self.major, self.minor);
        match v {
            (1, 7..=8) => (1, 1),
            (1, 9..=11) => (1, 2),
            (1, 12..=15) => (1, 3),
            (1, 16..=19) => (1, 4),
            (1, 20..=23) => (1, 5),
            (1, 24..=27) => (1, 6),
            (1, 28..=31) => (1, 7),
            (1, 32..=35) => (1, 8),
            (1, 36..=39) => (1, 9),
            (1, 40..=43) => (1, 10),
            (1, 44..=47) => (1, 11),
            (2, 0..=7) => (2, 1),
            (2, 8..=15) => (2, 2),
            (2, 16..=23) => (2, 3),
            (2, 24..=31) => (2, 4),
            (2, 32..=39) => (2, 5),
            (2, 40..=47) => (2, 6),
            (2, 48..=55) => (2, 7),
            (2, 56..=63) => (2, 8),
            (3, 0..=7) => (3, 1),
            (3, 8..=15) => (3, 2),
            (3, 16..=23) => (3, 3),
            (3, 24..=31) => (3, 4),
            (4, 0..=7) => (4, 1),
            (4, 8..=15) => (4, 2),
            (4, 16..=23) => (4, 3),
            (4, 24..=31) => (4, 4),
            (5, 0..=7) => (5, 1),
            (5, 8..=15) => (5, 2),
            (5, 16..=23) => (5, 3),
            (6, 0..=7) => (6, 1),
            _ => (0, 0),
        }
    }
}

/// Detect the Fortnite version from raw image bytes (searches ASCII first,
/// then UTF-16LE).
pub fn detect_from_bytes(data: &[u8]) -> Option<FortniteVersion> {
    if let Some(v) = find_release_ascii(data) {
        return Some(v);
    }
    find_release_wide(data)
}

fn parse_release(s: &str) -> Option<FortniteVersion> {
    // ++Fortnite+Release-<major>.<minor>-CL-<cl>
    let rest = s.strip_prefix("++Fortnite+Release-")?;
    let mut parts = rest.splitn(2, "-CL-");
    let ver = parts.next()?;
    let cl = parts.next().and_then(|c| c.parse().ok()).unwrap_or(0);
    let mut dot = ver.split('.');
    let major = dot.next()?.parse().ok()?;
    let minor = dot.next().unwrap_or("0").parse().ok()?;
    Some(FortniteVersion {
        major,
        minor,
        cl,
        release_string: s.to_string(),
    })
}

fn find_release_ascii(data: &[u8]) -> Option<FortniteVersion> {
    let needle = b"++Fortnite+Release-";
    let mut cursor = 0usize;
    while let Some(rel) = find_sub(data, needle, cursor) {
        // Take up to 64 printable bytes.
        let end = (rel + 64).min(data.len());
        let slice = &data[rel..end];
        let len = slice
            .iter()
            .position(|&b| !(b.is_ascii_graphic() || b == b' '))
            .unwrap_or(slice.len());
        let text = String::from_utf8_lossy(&slice[..len]).into_owned();
        if let Some(v) = parse_release(&text) {
            return Some(v);
        }
        cursor = rel + 1;
    }
    None
}

fn find_release_wide(data: &[u8]) -> Option<FortniteVersion> {
    let needle = b"++Fortnite+Release-";
    // UTF-16LE: interleave with zeros.
    let mut wide: Vec<u8> = Vec::with_capacity(needle.len() * 2);
    for &b in needle {
        wide.push(b);
        wide.push(0);
    }
    let mut cursor = 0usize;
    while let Some(rel) = find_sub(data, &wide, cursor) {
        let mut text = String::new();
        let mut i = rel;
        while i + 1 < data.len() && data[i + 1] == 0 && data[i].is_ascii_graphic() {
            text.push(data[i] as char);
            i += 2;
        }
        if let Some(v) = parse_release(&text) {
            return Some(v);
        }
        cursor = rel + 1;
    }
    None
}

fn find_sub(haystack: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack[from..]
        .windows(needle.len())
        .position(|w| w == needle)
        .map(|p| p + from)
}
