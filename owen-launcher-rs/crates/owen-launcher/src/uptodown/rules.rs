#![forbid(unsafe_code)]
//! 1:1 port of `VersionRules.kt` (object `VersionRules`).

/// `VersionRules.VersionCheck` — the outcome of [`VersionRules::check`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionCheck {
    pub allowed: bool,
    pub blocked: bool,
    pub reason: String,
}

/// 1:1 port of the Kotlin `object VersionRules`. Every function keeps the
/// original name (snake_case), the original constants/messages and the
/// original step order.
pub struct VersionRules;

impl VersionRules {
    /// `VersionRules.check(versionInput, matches)`.
    pub fn check(version_input: &str, matches: &[String]) -> VersionCheck {
        let parsed = match Self::parse(version_input) {
            Some(parsed) => parsed,
            None => {
                return VersionCheck {
                    allowed: false,
                    blocked: false,
                    reason: "Invalid version format. Use for example 13.40 or 33.00".to_string(),
                }
            }
        };
        let (major, minor, _patch) = parsed;
        if (major <= 12 && (major != 12 || minor < 61)) || (major >= 33 && (major != 33 || minor != 0))
        {
            return VersionCheck {
                allowed: false,
                blocked: true,
                reason: "Only versions from 12.61 to 33.00 are supported".to_string(),
            };
        }
        for entry in matches {
            if let Some(stripped) = entry.strip_prefix('-') {
                let blocked_version = stripped.trim();
                if !blocked_version.is_empty() && Self::matches(version_input, blocked_version) {
                    return VersionCheck {
                        allowed: false,
                        blocked: true,
                        reason: format!("Version {version_input} is blocked"),
                    };
                }
            }
        }
        VersionCheck {
            allowed: true,
            blocked: false,
            reason: String::new(),
        }
    }

    /// `VersionRules.parse(versionInput): Triple<Int, Int, Int>?`.
    pub fn parse(version_input: &str) -> Option<(i32, i32, i32)> {
        // cleaned = versionInput.trim().substringBefore('-').substringBefore("Android").trim()
        let cleaned = version_input.trim();
        let cleaned = cleaned.split('-').next().unwrap_or("");
        let cleaned = cleaned.split("Android").next().unwrap_or("");
        let cleaned = cleaned.trim();
        if cleaned.is_empty() {
            return None;
        }
        let parts: Vec<&str> = cleaned.split('.').collect();
        if parts.is_empty() || parts.len() > 3 {
            return None;
        }
        let mut numbers: Vec<i32> = Vec::with_capacity(parts.len());
        for part in parts {
            numbers.push(part.trim().parse::<i32>().ok()?);
        }
        while numbers.len() < 3 {
            numbers.push(0);
        }
        Some((numbers[0], numbers[1], numbers[2]))
    }

    /// `VersionRules.matches(versionInput, candidateVersion)`.
    pub fn matches(version_input: &str, candidate_version: &str) -> bool {
        let input = match Self::parse(version_input) {
            Some(parsed) => parsed,
            None => return false,
        };
        let candidate = match Self::parse(candidate_version) {
            Some(parsed) => parsed,
            None => return false,
        };
        if input.0 != candidate.0 {
            return false;
        }
        if input.1 == 0 || input.1 == candidate.1 {
            return input.2 == 0 || input.2 == candidate.2;
        }
        false
    }

    /// `VersionRules.normalize(versionInput)`.
    pub fn normalize(version_input: &str) -> String {
        match Self::parse(version_input) {
            None => version_input.trim().to_string(),
            Some((major, minor, patch)) => {
                if patch == 0 {
                    format!("{major}.{minor}")
                } else {
                    format!("{major}.{minor}.{patch}")
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(list: &[&str]) -> Vec<String> {
        list.iter().map(|v| v.to_string()).collect()
    }

    #[test]
    fn parse_variants() {
        assert_eq!(VersionRules::parse("13.40"), Some((13, 40, 0)));
        assert_eq!(VersionRules::parse("13.40-CL-14009477-Android"), Some((13, 40, 0)));
        assert_eq!(VersionRules::parse("12.61"), Some((12, 61, 0)));
        assert_eq!(VersionRules::parse("33.00"), Some((33, 0, 0)));
        assert_eq!(VersionRules::parse("5.41"), Some((5, 41, 0)));
        assert_eq!(VersionRules::parse("1.2.3"), Some((1, 2, 3)));
        assert_eq!(VersionRules::parse("13"), Some((13, 0, 0)));
        assert_eq!(VersionRules::parse(""), None);
        assert_eq!(VersionRules::parse("abc"), None);
        assert_eq!(VersionRules::parse("13.40.1.2"), None, "more than 3 parts");
        assert_eq!(VersionRules::parse("13.x"), None);
        assert_eq!(
            VersionRules::parse("13.40 Android"),
            Some((13, 40, 0)),
            "substringBefore(\"Android\")"
        );
    }

    #[test]
    fn check_range_rules() {
        assert!(VersionRules::check("13.40", &[]).allowed);
        assert!(VersionRules::check("12.61", &[]).allowed);
        assert!(VersionRules::check("33.00", &[]).allowed);
        let too_old = VersionRules::check("12.60", &[]);
        assert!(!too_old.allowed && too_old.blocked);
        assert_eq!(
            too_old.reason,
            "Only versions from 12.61 to 33.00 are supported"
        );
        let too_new = VersionRules::check("33.01", &[]);
        assert!(!too_new.allowed && too_new.blocked);
        let invalid = VersionRules::check("banana", &[]);
        assert!(!invalid.allowed && !invalid.blocked);
        assert_eq!(
            invalid.reason,
            "Invalid version format. Use for example 13.40 or 33.00"
        );
    }

    #[test]
    fn check_blocked_entries() {
        let blocked = VersionRules::check("13.40", &s(&["-13.40"]));
        assert!(!blocked.allowed && blocked.blocked);
        assert_eq!(blocked.reason, "Version 13.40 is blocked");
        // Entries without the "-" prefix are ignored.
        assert!(VersionRules::check("13.40", &s(&["13.40"])).allowed);
        // The blocked version is trimmed.
        assert!(!VersionRules::check("13.40", &s(&["-  13.40 "])).allowed);
        // Empty after trimming → ignored.
        assert!(VersionRules::check("13.40", &s(&["-  "])).allowed);
        // A different version does not match.
        assert!(VersionRules::check("13.40", &s(&["-16.00"])).allowed);
    }

    #[test]
    fn matches_wildcards() {
        // minor 0 = wildcard
        assert!(VersionRules::matches("13", "13.40"));
        // patch 0 = wildcard on the INPUT side only (`input.third == 0` in
        // the Kotlin original), so ("13.40", "13.40.1") matches but
        // ("13.40.1", "13.40") does not.
        assert!(VersionRules::matches("13.40", "13.40.1"));
        assert!(!VersionRules::matches("13.40.1", "13.40"));
        // exact
        assert!(VersionRules::matches("13.40", "13.40"));
        // major mismatch
        assert!(!VersionRules::matches("13.40", "14.40"));
        // minor mismatch
        assert!(!VersionRules::matches("13.40", "13.41"));
        // patch mismatch
        assert!(!VersionRules::matches("13.40.1", "13.40.2"));
        // unparseable input
        assert!(!VersionRules::matches("banana", "13.40"));
        assert!(!VersionRules::matches("13.40", "banana"));
    }

    #[test]
    fn normalize_variants() {
        assert_eq!(VersionRules::normalize("13.40-CL-14009477-Android"), "13.40");
        assert_eq!(VersionRules::normalize("13.40"), "13.40");
        assert_eq!(VersionRules::normalize("1.2.3"), "1.2.3");
        assert_eq!(VersionRules::normalize("banana "), "banana");
    }
}
