#![forbid(unsafe_code)]
//! 1:1 port of `UptodownVersion.kt` (data class `UptodownVersion`).

/// A version entry of the Uptodown versions list (`UptodownVersion.kt`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct UptodownVersion {
    /// e.g. `"13.40-CL-14009477-Android"` (Kotlin: `version`).
    pub version: String,
    /// `versionURL.versionID` (Kotlin: `versionId`).
    pub version_id: i64,
    /// `"apk"` / `"xapk"` (Kotlin: `kindFile`).
    pub kind_file: String,
    /// e.g. `"33"` (Kotlin: `sdkVersion`).
    pub sdk_version: String,
    /// Human-readable last update (Kotlin: `lastUpdate`).
    pub last_update: String,
}

impl UptodownVersion {
    /// `kindFile.equals("xapk", ignoreCase = true)`.
    pub fn is_xapk(&self) -> bool {
        self.kind_file.eq_ignore_ascii_case("xapk")
    }

    /// `version.substringBefore(' ').substringBefore('-')`.
    pub fn short_version(&self) -> String {
        let before_space = self.version.split(' ').next().unwrap_or(&self.version);
        let before_dash = before_space.split('-').next().unwrap_or(before_space);
        before_dash.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_xapk_matches_case_insensitive() {
        let mk = |kind: &str| UptodownVersion {
            version: "13.40".into(),
            version_id: 1,
            kind_file: kind.into(),
            sdk_version: String::new(),
            last_update: String::new(),
        };
        assert!(mk("xapk").is_xapk());
        assert!(mk("XAPK").is_xapk());
        assert!(mk("Xapk").is_xapk());
        assert!(!mk("apk").is_xapk());
        assert!(!mk("").is_xapk());
    }

    #[test]
    fn short_version_cuts_space_then_dash() {
        let mk = |version: &str| UptodownVersion {
            version: version.into(),
            version_id: 1,
            kind_file: "apk".into(),
            sdk_version: String::new(),
            last_update: String::new(),
        };
        assert_eq!(mk("13.40").short_version(), "13.40");
        assert_eq!(mk("13.40-CL-14009477-Android").short_version(), "13.40");
        assert_eq!(mk("13.40-CL-14009477 Android").short_version(), "13.40");
        assert_eq!(mk("33.00").short_version(), "33.00");
    }
}
