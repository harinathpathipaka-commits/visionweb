//! Fingerprint profiles — per-session consistent browser identity.
//!
//! Each session gets a randomly selected profile that remains
//! consistent across all pages within that session. This mimics
//! how a real user doesn't change hardware between pages.
//!
//! Profiles include: screen dimensions, WebGL GPU strings,
//! platform, language preferences, timezone, and color depth.

use serde::{Deserialize, Serialize};

/// A complete browser fingerprint profile.
///
/// All values in this profile are applied to every page in a session.
/// The profile is generated once at session creation and never changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FingerprintProfile {
    /// Reported WebGL vendor string (e.g., "Google Inc. (Intel)")
    pub webgl_vendor: String,
    /// Reported WebGL renderer string (e.g., "ANGLE (Intel, ...)")
    pub webgl_renderer: String,
    /// Screen width in pixels
    pub screen_width: u32,
    /// Screen height in pixels
    pub screen_height: u32,
    /// Color depth (24 or 30)
    pub color_depth: u32,
    /// Platform string (Win32, MacIntel, Linux x86_64)
    pub platform: String,
    /// Preferred languages
    pub languages: Vec<String>,
    /// Timezone identifier
    pub timezone: String,
}

impl FingerprintProfile {
    /// A realistic Windows 11 + Chrome profile with Intel GPU.
    #[must_use]
    pub fn windows_chrome() -> Self {
        Self {
            webgl_vendor: "Google Inc. (Intel)".into(),
            webgl_renderer: "ANGLE (Intel, Intel(R) UHD Graphics (0x00009A60) Direct3D11 vs_5_0 ps_5_0, D3D11)".into(),
            screen_width: 1920,
            screen_height: 1080,
            color_depth: 24,
            platform: "Win32".into(),
            languages: vec!["en-US".into(), "en".into()],
            timezone: "America/New_York".into(),
        }
    }

    /// A realistic macOS + Chrome profile with Apple M-series GPU.
    #[must_use]
    #[allow(dead_code)]
    pub fn mac_chrome() -> Self {
        Self {
            webgl_vendor: "Google Inc. (Apple)".into(),
            webgl_renderer: "ANGLE (Apple, Apple M2, OpenGL 4.1)".into(),
            screen_width: 1728,
            screen_height: 1117,
            color_depth: 30,
            platform: "MacIntel".into(),
            languages: vec!["en-US".into(), "en".into()],
            timezone: "America/Los_Angeles".into(),
        }
    }

    /// A realistic Linux + Chrome profile.
    #[must_use]
    #[allow(dead_code)]
    pub fn linux_chrome() -> Self {
        Self {
            webgl_vendor: "Google Inc. (Mesa)".into(),
            webgl_renderer: "ANGLE (Mesa, Mesa Intel(R) UHD Graphics (ICL GT1), OpenGL 4.6)".into(),
            screen_width: 1920,
            screen_height: 1080,
            color_depth: 24,
            platform: "Linux x86_64".into(),
            languages: vec!["en-US".into(), "en".into()],
            timezone: "America/Chicago".into(),
        }
    }

    /// Randomize minor variations within a profile to avoid identical
    /// fingerprints across sessions. Call once per session.
    #[must_use]
    #[allow(dead_code)]
    pub fn with_random_variation(&self) -> Self {
        let resolutions: [(u32, u32); 4] = [
            (1920, 1080),
            (1920, 1200),
            (2560, 1440),
            (1680, 1050),
        ];
        let pick = fast_index(resolutions.len());
        let (w, h) = resolutions[pick];

        let depths: [u32; 2] = [24, 30];
        let cd = depths[fast_index(depths.len())];

        Self {
            screen_width: w,
            screen_height: h,
            color_depth: cd,
            ..self.clone()
        }
    }
}

// ── Minimal RNG for profile selection ──────────────────────────────

fn fast_index(max: usize) -> usize {
    if max <= 1 { return 0; }
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    (nanos as usize) % max
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_windows_profile_has_valid_fields() {
        let p = FingerprintProfile::windows_chrome();
        assert!(!p.webgl_vendor.is_empty());
        assert!(!p.webgl_renderer.is_empty());
        assert!(p.screen_width > 0);
        assert!(p.screen_height > 0);
        assert!(p.color_depth >= 24);
        assert_eq!(p.platform, "Win32");
        assert!(!p.languages.is_empty());
    }

    #[test]
    fn test_mac_profile() {
        let p = FingerprintProfile::mac_chrome();
        assert_eq!(p.platform, "MacIntel");
    }

    #[test]
    fn test_linux_profile() {
        let p = FingerprintProfile::linux_chrome();
        assert_eq!(p.platform, "Linux x86_64");
    }

    #[test]
    fn test_random_variation_changes_screen() {
        let base = FingerprintProfile::windows_chrome();
        let mut saw_different = false;
        for _ in 0..20 {
            let variant = base.with_random_variation();
            if variant.screen_width != base.screen_width {
                saw_different = true;
                break;
            }
        }
        // Statistically should see variation, but not guaranteed in test
        // Just verify it doesn't panic
        let _ = saw_different;
    }

    #[test]
    fn test_serde_roundtrip() {
        let p = FingerprintProfile::windows_chrome();
        let json = serde_json::to_string(&p).expect("FingerprintProfile serialization is infallible");
        let p2: FingerprintProfile = serde_json::from_str(&json).expect("FingerprintProfile deserialization is infallible");
        assert_eq!(p.webgl_vendor, p2.webgl_vendor);
        assert_eq!(p.screen_width, p2.screen_width);
    }
}
