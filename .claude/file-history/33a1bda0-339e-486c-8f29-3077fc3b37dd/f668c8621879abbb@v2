//! Anti-detection engine for the Agent Nervous System.
//!
//! Protects Chromium instances from bot detection by masking browser
//! automation signals at three layers:
//!
//! - **Launch flags**: Chrome CLI flags that suppress automation markers
//! - **CDP patches**: JavaScript injected before page scripts via
//!   `Page.addScriptToEvaluateOnNewDocument`
//! - **Behavioral**: Humanized mouse movement (bezier curves) and
//!   variable typing delays
//!
//! # Adaptive escalation
//!
//! The engine integrates with the 5 Eyes perception layer. When the
//! Error Detector flags a page as bot-detected (Cloudflare, captcha,
//! DataDome), the stealth level can be escalated dynamically:
//!
//! ```text
//! Off → Standard → Aggressive → Paranoid
//! ```
//!
//! Each level adds more evasion techniques. Higher levels cost more
//! CPU (canvas/WebGL spoofing, deeper DOM patches).
//!
//! # Fingerprint consistency
//!
//! Unlike `puppeteer-extra-plugin-stealth` which randomizes per page,
//! ANS maintains per-session fingerprint consistency — the same
//! browser identity across all pages in a session. This matches
//! real user behavior (you don't change GPUs between tabs).

pub mod flags;
pub mod humanize;
pub mod patches;
pub mod profiles;

use serde::{Deserialize, Serialize};

pub use profiles::FingerprintProfile;

/// Stealth configuration, created once per browser session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StealthConfig {
    /// Current evasion level. May be escalated dynamically.
    pub level: StealthLevel,
    /// Browser fingerprint profile (consistent per session).
    pub profile: FingerprintProfile,
    /// Enable humanized mouse/keyboard behavior.
    pub humanize: bool,
    /// Enable adaptive escalation (auto-escalate on bot detection).
    pub adaptive: bool,
}

impl Default for StealthConfig {
    fn default() -> Self {
        Self {
            level: StealthLevel::Aggressive,
            profile: FingerprintProfile::windows_chrome(),
            humanize: true,
            adaptive: true,
        }
    }
}

impl StealthConfig {
    /// Create a stealth config suitable for production use.
    /// Aggressive level with Windows Chrome profile.
    #[must_use]
    pub fn standard() -> Self {
        Self {
            level: StealthLevel::Aggressive,
            ..Self::default()
        }
    }

    /// Maximum stealth — paranoid level with all evasions.
    #[must_use]
    #[allow(dead_code)]
    pub fn paranoid() -> Self {
        Self {
            level: StealthLevel::Paranoid,
            profile: FingerprintProfile::windows_chrome().with_random_variation(),
            humanize: true,
            adaptive: false,
        }
    }

    /// Create a config suitable for internal/trusted pages.
    /// Disables all stealth (faster, no overhead).
    #[must_use]
    pub fn off() -> Self {
        Self {
            level: StealthLevel::Off,
            ..Self::default()
        }
    }

    /// Get Chrome CLI flags for the current level.
    #[must_use]
    pub fn chrome_flags(&self) -> Vec<String> {
        flags::chrome_flags(self.level)
    }

    /// Get CDP init scripts for the current level.
    #[must_use]
    pub fn init_scripts(&self) -> Vec<String> {
        patches::init_scripts(self.level, &self.profile)
    }

    /// Get the self-audit script that verifies evasions are working.
    #[must_use]
    pub fn audit_script() -> String {
        patches::audit_script()
    }

    /// Escalate stealth level if adaptive mode is enabled.
    /// Called when the Error Detector (5 Eyes) flags bot detection.
    pub fn escalate(&mut self) {
        if !self.adaptive {
            return;
        }
        self.level = self.level.escalate();
        tracing::warn!(
            level = ?self.level,
            "Stealth level escalated due to bot detection signal"
        );
    }

    /// De-escalate stealth level (called after leaving a protected page).
    pub fn de_escalate(&mut self) {
        self.level = self.level.de_escalate();
    }
}

/// Stealth level — how aggressively to evade detection.
///
/// Levels are ordered: higher = more evasion, more CPU cost.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum StealthLevel {
    /// No stealth — use for internal/trusted pages. Zero overhead.
    Off = 0,
    /// Core evasions only — webdriver, chrome, plugins, languages.
    /// Covers ~90% of detection vectors. Minimal overhead.
    Standard = 1,
    /// Full evasions — adds canvas, audio, WebGL spoofing.
    /// Moderate CPU overhead (~5%).
    Aggressive = 2,
    /// All evasions including experimental patches.
    /// May affect page behavior slightly. Highest CPU cost (~10%).
    Paranoid = 3,
}

impl StealthLevel {
    /// Escalate to the next level. Cannot go above Paranoid.
    #[must_use]
    pub fn escalate(self) -> Self {
        match self {
            Self::Off => Self::Standard,
            Self::Standard => Self::Aggressive,
            Self::Aggressive | Self::Paranoid => Self::Paranoid,
        }
    }

    /// De-escalate to the previous level. Cannot go below Off.
    #[must_use]
    pub fn de_escalate(self) -> Self {
        match self {
            Self::Off | Self::Standard => Self::Off,
            Self::Aggressive => Self::Standard,
            Self::Paranoid => Self::Aggressive,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_aggressive() {
        let config = StealthConfig::default();
        assert_eq!(config.level, StealthLevel::Aggressive);
        assert!(config.humanize);
        assert!(config.adaptive);
    }

    #[test]
    fn test_escalation_chain() {
        assert_eq!(StealthLevel::Off.escalate(), StealthLevel::Standard);
        assert_eq!(StealthLevel::Standard.escalate(), StealthLevel::Aggressive);
        assert_eq!(StealthLevel::Aggressive.escalate(), StealthLevel::Paranoid);
        assert_eq!(StealthLevel::Paranoid.escalate(), StealthLevel::Paranoid);
    }

    #[test]
    fn test_de_escalation_chain() {
        assert_eq!(StealthLevel::Paranoid.de_escalate(), StealthLevel::Aggressive);
        assert_eq!(StealthLevel::Aggressive.de_escalate(), StealthLevel::Standard);
        assert_eq!(StealthLevel::Standard.de_escalate(), StealthLevel::Off);
        assert_eq!(StealthLevel::Off.de_escalate(), StealthLevel::Off);
    }

    #[test]
    fn test_off_config_has_no_scripts() {
        let config = StealthConfig::off();
        assert!(config.init_scripts().is_empty());
    }

    #[test]
    fn test_standard_config_has_flags_and_scripts() {
        let config = StealthConfig::standard();
        assert!(!config.chrome_flags().is_empty());
        assert!(!config.init_scripts().is_empty());
    }

    #[test]
    fn test_escalate_only_when_adaptive() {
        let mut config = StealthConfig::standard();
        config.adaptive = false;
        config.escalate();
        assert_eq!(config.level, StealthLevel::Aggressive); // unchanged: adaptive=false blocks escalation
    }

    #[test]
    fn test_escalate_works_when_adaptive() {
        let mut config = StealthConfig::standard();
        config.adaptive = true;
        config.escalate();
        assert_eq!(config.level, StealthLevel::Paranoid); // Aggressive → Paranoid
    }

    #[test]
    fn test_audit_script_is_valid() {
        let audit = StealthConfig::audit_script();
        assert!(audit.contains("webdriver"));
        assert!(audit.contains("JSON.stringify"));
    }
}
