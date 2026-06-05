/// What kind of distraction was detected on the page.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DistractionKind {
    Ad,
    Popup,
    CookieBanner,
    NewsletterModal,
    Redirect,
    AutoPlayVideo,
    Survey,
    Notification,
    Unknown,
}

/// The immune system's recommended response to a distraction.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ImmuneAction {
    /// Click the close/dismiss button to remove the element.
    Dismiss,
    /// Prevent the agent from interacting with this element at all.
    Block,
    /// Go back to the previous page (for redirects).
    NavigateBack,
    /// Hide from the agent's perception but leave on the page.
    Suppress,
    /// False positive — allow through unchanged.
    Ignore,
}

/// A flagged distraction element.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DistractionFlag {
    pub kind: DistractionKind,
    pub element_selector: String,
    /// 0.0 (maybe) to 1.0 (definitely a distraction).
    pub confidence: f32,
    pub suggested_action: ImmuneAction,
}

// ── Prompt Injection Defense ───────────────────────────────

/// Result of scanning page content for prompt injection.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InjectionScanResult {
    /// 0.0 (safe) to 1.0 (definite injection attempt).
    pub score: f32,
    /// Specific content that was flagged.
    pub flagged_content: Vec<InjectionFlag>,
    /// Recommended action.
    pub action: InjectionAction,
}

/// A piece of content flagged as a potential injection.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InjectionFlag {
    pub content_snippet: String,
    pub pattern_matched: String,
    pub location: InjectionLocation,
    pub confidence: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum InjectionLocation {
    HiddenDiv,
    DisplayNone,
    AriaLabel,
    CssPseudoElement,
    ZeroWidthChars,
    Homoglyph,
    MetaTag,
    AltText,
    InlineScript,
    Unknown,
}

impl InjectionLocation {
    #[must_use] 
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::HiddenDiv => "hidden_div",
            Self::DisplayNone => "display_none",
            Self::AriaLabel => "aria_label",
            Self::CssPseudoElement => "css_pseudo",
            Self::ZeroWidthChars => "zero_width_chars",
            Self::Homoglyph => "homoglyph",
            Self::MetaTag => "meta_tag",
            Self::AltText => "alt_text",
            Self::InlineScript => "inline_script",
            Self::Unknown => "unknown",
        }
    }

    #[must_use] 
    pub fn parse(s: &str) -> Self {
        match s {
            "hidden_div" => Self::HiddenDiv,
            "display_none" => Self::DisplayNone,
            "aria_label" => Self::AriaLabel,
            "css_pseudo" => Self::CssPseudoElement,
            "zero_width_chars" => Self::ZeroWidthChars,
            "homoglyph" => Self::Homoglyph,
            "meta_tag" => Self::MetaTag,
            "alt_text" => Self::AltText,
            "inline_script" => Self::InlineScript,
            _ => Self::Unknown,
        }
    }
}

/// The score threshold bands that determine the injection response.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum InjectionScore {
    /// score < 0.3 — content is safe.
    Allow,
    /// 0.3 ≤ score < 0.7 — strip flagged content, pass the rest.
    Sanitize,
    /// score ≥ 0.7 — page is malicious, block entirely.
    Block,
}

impl InjectionScore {
    #[must_use] 
    pub fn from_score(score: f32) -> Self {
        if score >= 0.7 {
            Self::Block
        } else if score >= 0.3 {
            Self::Sanitize
        } else {
            Self::Allow
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum InjectionAction {
    Allow,
    Sanitize(Vec<String>),
    Block(String),
}

impl InjectionAction {
    /// Map a score to the appropriate action band.
    #[must_use] 
    pub fn from_score(score: f32) -> Self {
        if score >= 0.7 {
            Self::Block("injection_score >= 0.7".into())
        } else if score >= 0.3 {
            Self::Sanitize(vec!["flagged_content_removed".into()])
        } else {
            Self::Allow
        }
    }

    #[must_use] 
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Sanitize(_) => "sanitize",
            Self::Block(_) => "block",
        }
    }
}
