//! Semantic block segmentation.
//!
//! Groups distilled elements into semantic regions: navigation,
//! main content, forms, sidebars, footers, ads, cookie banners.
//! Uses heuristics (position, tag, role, ARIA landmarks) — no ML in v1.

use ans_core::distill::{BlockType, DistilledElement, SemanticBlock};

/// Heuristic pattern matchers for common DOM landmarks.
const NAV_TAGS: &[&str] = &["nav", "header"];
const MAIN_TAGS: &[&str] = &["main", "article", "section"];
const FORM_TAGS: &[&str] = &["form"];
const SIDEBAR_TAGS: &[&str] = &["aside"];
const FOOTER_TAGS: &[&str] = &["footer"];

const AD_CLASS_PATTERNS: &[&str] = &[
    "ad",
    "advertisement",
    "sponsor",
    "banner-ad",
    "display-ad",
    "google-ad",
    "dfp-ad",
    "ad-container",
    "promoted",
];
const COOKIE_PATTERNS: &[&str] = &[
    "cookie",
    "gdpr",
    "consent",
    "privacy-policy",
    "cookie-banner",
    "cookie-notice",
];
const POPUP_PATTERNS: &[&str] = &["popup", "modal", "overlay", "dialog", "lightbox"];

/// Classify a single element into a block type using tag, role, text, and context.
#[must_use] 
pub fn classify_element(
    tag: &str,
    role: Option<&str>,
    text: &str,
    class_id_hint: &str,
) -> BlockType {
    let tag_lower = tag.to_lowercase();
    let hint_lower = class_id_hint.to_lowercase();
    let text_lower = text.to_lowercase();
    let combined = format!("{} {} {}", hint_lower, text_lower, role.unwrap_or(""));

    // ARIA roles are highest-signal — check first
    if let Some(r) = role {
        match r {
            "navigation" | "banner" => return BlockType::Navigation,
            "main" => return BlockType::MainContent,
            "form" | "search" => return BlockType::Form,
            "complementary" => return BlockType::Sidebar,
            "contentinfo" => return BlockType::Footer,
            "dialog" | "alertdialog" => return BlockType::Popup,
            _ => {}
        }
    }

    // Tag-level classification
    if NAV_TAGS.contains(&tag_lower.as_str()) {
        return BlockType::Navigation;
    }
    if MAIN_TAGS.contains(&tag_lower.as_str()) {
        return BlockType::MainContent;
    }
    if FORM_TAGS.contains(&tag_lower.as_str()) {
        return BlockType::Form;
    }
    if SIDEBAR_TAGS.contains(&tag_lower.as_str()) {
        return BlockType::Sidebar;
    }
    if FOOTER_TAGS.contains(&tag_lower.as_str()) {
        return BlockType::Footer;
    }

    // Distraction detection via class/id/text patterns
    for pat in COOKIE_PATTERNS {
        if combined.contains(pat) {
            return BlockType::CookieBanner;
        }
    }
    for pat in POPUP_PATTERNS {
        if hint_lower.contains(pat) {
            return BlockType::Popup;
        }
    }
    for pat in AD_CLASS_PATTERNS {
        if hint_lower.contains(pat) {
            return BlockType::Ad;
        }
    }

    // Heuristic: dense links cluster → likely navigation
    if tag_lower == "div" || tag_lower == "ul" {
        let link_count = text_lower.matches("href").count();
        if link_count >= 3 {
            return BlockType::Navigation;
        }
    }

    BlockType::Unknown
}

/// Segment a flat list of elements into semantic blocks.
///
/// Groups consecutive elements that share a semantic region.
/// Elements that can't be classified are attached to the nearest
/// classified ancestor or grouped as Unknown.
#[must_use] 
pub fn segment_blocks(elements: &[DistilledElement]) -> Vec<SemanticBlock> {
    if elements.is_empty() {
        return vec![];
    }

    let mut blocks: Vec<SemanticBlock> = Vec::new();
    let mut current_type = BlockType::Unknown;
    let mut current_text = String::new();
    let mut current_indices: Vec<usize> = Vec::new();

    for (i, el) in elements.iter().enumerate() {
        let role = el.role.as_deref();
        let raw_attrs: String = el
            .attributes
            .iter()
            .fold(String::new(), |mut acc, (k, v)| {
                use std::fmt::Write;
                let _ = write!(acc, "{k}={v} ");
                acc
            });
        let class_id_hint = format!(
            "{} {} {}",
            el.attributes.get("class").map_or("", std::string::String::as_str),
            el.attributes.get("id").map_or("", std::string::String::as_str),
            raw_attrs
        );

        let block_type = classify_element(&el.tag, role, &el.text, &class_id_hint);

        if block_type == current_type {
            // Extend current block
            current_text.push(' ');
            current_text.push_str(&el.text);
            current_indices.push(i);
        } else {
            // Flush previous block
            if !current_indices.is_empty() {
                blocks.push(SemanticBlock {
                    block_type: current_type.clone(),
                    text_content: current_text.trim().to_string(),
                    element_indices: current_indices.clone(),
                    goal_relevance_score: default_relevance(&current_type),
                });
            }
            current_type = block_type;
            current_text.clone_from(&el.text);
            current_indices = vec![i];
        }
    }

    // Flush final block
    if !current_indices.is_empty() {
        let relevance = default_relevance(&current_type);
        blocks.push(SemanticBlock {
            block_type: current_type,
            text_content: current_text.trim().to_string(),
            element_indices: current_indices,
            goal_relevance_score: relevance,
        });
    }

    blocks
}

const fn default_relevance(block_type: &BlockType) -> f32 {
    match block_type {
        BlockType::MainContent => 1.0,
        BlockType::Form => 0.9,
        BlockType::Navigation => 0.5,
        BlockType::Sidebar | BlockType::Unknown => 0.3,
        BlockType::Footer => 0.2,
        BlockType::Ad | BlockType::CookieBanner => 0.0,
        BlockType::Popup => 0.1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nav_classification() {
        assert_eq!(
            classify_element("nav", None, "Home About", ""),
            BlockType::Navigation
        );
        assert_eq!(
            classify_element("div", Some("navigation"), "links", ""),
            BlockType::Navigation
        );
    }

    #[test]
    fn test_ad_detection() {
        assert_eq!(
            classify_element("div", None, "Buy now!", "banner-ad-container sponsored"),
            BlockType::Ad
        );
    }

    #[test]
    fn test_cookie_banner() {
        assert_eq!(
            classify_element("div", None, "This site uses cookies", "cookie-banner"),
            BlockType::CookieBanner
        );
    }

    #[test]
    fn test_segment_empty() {
        let result = segment_blocks(&[]);
        assert!(result.is_empty());
    }
}
