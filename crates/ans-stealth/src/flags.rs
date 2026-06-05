//! Chrome launch flags for anti-detection.
//!
//! Organized by stealth level. Standard mode applies the minimum
//! effective set. Aggressive adds experimental flags. Paranoid
//! applies everything including flags that may affect performance.

use super::StealthLevel;

/// Return Chrome CLI flags for a given stealth level.
#[must_use]
pub fn chrome_flags(level: StealthLevel) -> Vec<String> {
    let base = vec![
        // ── Critical automation masking ──────────────────────
        "--disable-blink-features=AutomationControlled".to_string(),
        "--disable-features=ChromeWhatsNewUI,TranslateUI,InterestFeedContentSuggestions,OptimizationHints,MediaRouter,ProcessPerSiteUpToMainFrameThreshold,DialMediaRouteProvider,CalculateNativeWinOcclusion,BlockInsecurePrivateNetworkRequests,InterestFeedContentSuggestions,CertificateTransparencyComponentUpdater,AutofillServerCommunication,PasswordLeakDetection,SafeBrowsingEnhancedProtection,SafeBrowsingUpdate,PrivacySandboxSettings,ChromeWhatsNewUI,DownloadBubble,DownloadBubbleV2,BackForwardCache".to_string(),
        // ── Prevent automation flag leaks ────────────────────
        "--disable-field-trial-config".to_string(),
        "--disable-hang-monitor".to_string(),
        "--disable-prompt-on-repost".to_string(),
        "--disable-component-update".to_string(),
        "--disable-domain-reliability".to_string(),
        "--disable-ipc-flooding-protection".to_string(),
        "--no-pings".to_string(),
        "--no-service-autorun".to_string(),
    ];

    let mut flags = base;

    if level >= StealthLevel::Standard {
        flags.extend(vec![
            "--disable-web-security".to_string(),
            "--disable-features=SignedHTTPExchange,IsolateOrigins,site-per-process".to_string(),
            "--enable-features=NetworkService,NetworkServiceInProcess".to_string(),
            "--disable-client-side-phishing-detection".to_string(),
            "--disable-component-extensions-with-background-pages".to_string(),
            "--disable-crash-reporter".to_string(),
            "--disable-breakpad".to_string(),
            "--enable-automation".to_string(), // ironic but helps some CDP operations
        ]);
    }

    if level >= StealthLevel::Aggressive {
        flags.extend(vec![
            "--disable-features=VizDisplayCompositor".to_string(),
            "--disable-logging".to_string(),
            "--disable-background-timer-throttling".to_string(),
            "--disable-renderer-backgrounding".to_string(),
            "--disable-backgrounding-occluded-windows".to_string(),
            "--disable-popup-blocking".to_string(),
            "--disable-password-generation".to_string(),
            "--disable-save-password-bubble".to_string(),
            "--disable-infobars".to_string(),
            "--force-color-profile=srgb".to_string(),
            // Aggressive: virtual display buffer
            "--use-gl=swiftshader".to_string(),
            "--use-angle=swiftshader".to_string(),
        ]);
    }

    if level >= StealthLevel::Paranoid {
        flags.extend(vec![
            "--disable-features=AudioServiceOutOfProcess".to_string(),
            "--disable-breakpad".to_string(),
            "--disable-crashpad".to_string(),
            "--disable-dev-shm-usage".to_string(),
            "--disable-gpu-sandbox".to_string(),
            "--disable-software-rasterizer".to_string(),
            "--enable-webgl".to_string(),
            "--enable-gpu-rasterization".to_string(),
            "--ignore-gpu-blocklist".to_string(),
            "--use-fake-device-for-media-stream".to_string(),
            "--use-fake-ui-for-media-stream".to_string(),
            "--disable-notifications".to_string(),
            "--disable-geolocation".to_string(),
            "--disable-reading-from-canvas".to_string(), // anti-canvas-fingerprinting
        ]);
    }

    flags
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_includes_automation_controlled() {
        let flags = chrome_flags(StealthLevel::Standard);
        assert!(flags.contains(&"--disable-blink-features=AutomationControlled".to_string()));
    }

    #[test]
    fn test_aggressive_has_more_flags_than_standard() {
        let std = chrome_flags(StealthLevel::Standard);
        let agg = chrome_flags(StealthLevel::Aggressive);
        assert!(agg.len() > std.len());
    }

    #[test]
    fn test_paranoid_has_most_flags() {
        let agg = chrome_flags(StealthLevel::Aggressive);
        let par = chrome_flags(StealthLevel::Paranoid);
        assert!(par.len() > agg.len());
    }

    #[test]
    fn test_off_returns_only_base() {
        let flags = chrome_flags(StealthLevel::Off);
        // Off still includes the critical AutomationControlled flag
        assert!(flags.contains(&"--disable-blink-features=AutomationControlled".to_string()));
        // But not the aggressive/paranoid flags
        assert!(!flags.contains(&"--force-color-profile=srgb".to_string()));
    }
}
