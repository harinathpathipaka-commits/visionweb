//! DOM distillation engine.
//!
//! Walks the raw CDP DOM tree (from `DOM.getDocument`) and produces a
//! [`DistilledPage`] in one pass. Mode controls what gets stripped vs. kept.
//!
//! Pipeline: walk → filter → annotate → segment → detect distractions.

use std::collections::HashMap;

use ans_core::distill::{
    DistillMode, DistilledElement, DistilledPage, InteractiveElement, InteractiveType,
};
use ans_core::immune::{DistractionFlag, DistractionKind, ImmuneAction};
use serde_json::Value as Json;

use crate::semantic;

/// Maximum elements to extract from any page. Pages exceeding this
/// are truncated to prevent memory exhaustion on DOM bombs.
const MAX_ELEMENTS: usize = 5000;

/// HTML tags that represent interactive elements.
const INTERACTIVE_TAGS: &[&str] = &[
    "a", "button", "input", "select", "textarea", "option", "form",
];

/// Tags that are purely presentational — always stripped in `TextOnly` mode,
/// and only kept in other modes if they contain text.
const PRESENTATIONAL_TAGS: &[&str] = &[
    "script", "style", "noscript", "svg", "path", "meta", "link", "br", "hr",
];

/// The distiller. Stateless — created once, reused across pages.
pub struct Distiller;

impl Distiller {
    /// Process raw CDP `DOM.getDocument` JSON into a [`DistilledPage`].
    ///
    /// `raw_dom` is the full response value from the CDP command.
    /// `mode` controls the distillation aggressiveness.
    #[must_use] 
    pub fn process(
        &self,
        raw_dom: &Json,
        mode: DistillMode,
        url: &str,
        title: &str,
    ) -> DistilledPage {
        let root = &raw_dom["root"];
        let mut ctx = WalkContext::new(mode);

        walk_node(root, 0, &mut ctx);

        // Segment into semantic blocks
        let semantic_blocks = semantic::segment_blocks(&ctx.elements);

        // Detect distractions from semantic blocks
        let distraction_flags = detect_distractions(&semantic_blocks, &ctx.elements);

        DistilledPage {
            mode,
            url: url.to_string(),
            title: title.to_string(),
            elements: ctx.elements,
            interactive: ctx.interactive,
            semantic_blocks,
            distraction_flags,
            timestamp: chrono::Utc::now(),
        }
    }
}

/// Mutable state accumulated during a single tree walk.
struct WalkContext {
    mode: DistillMode,
    elements: Vec<DistilledElement>,
    interactive: Vec<InteractiveElement>,
    /// Maps CDP nodeId → index into `elements`. Not persisted; used only
    /// during the walk to resolve child references.
    node_map: HashMap<i32, usize>,
}

impl WalkContext {
    fn new(mode: DistillMode) -> Self {
        Self {
            mode,
            elements: Vec::with_capacity(512),
            interactive: Vec::with_capacity(64),
            node_map: HashMap::with_capacity(512),
        }
    }

    const fn at_capacity(&self) -> bool {
        self.elements.len() >= MAX_ELEMENTS
    }
}

/// Recursively walk a CDP DOM node and its children.
fn walk_node(node: &Json, depth: usize, ctx: &mut WalkContext) {
    if ctx.at_capacity() {
        return;
    }

    let node_type = node["nodeType"].as_i64().unwrap_or(-1);

    match node_type {
        1 => walk_element(node, depth, ctx),
        3 => {
            // Text node — its value is accumulated by the parent element walker,
            // so we don't create a separate entry unless there's no parent context.
        }
        _ => {
            // Document (9), DocumentType (10), or unknown — recurse into children
            if let Some(children) = node["children"].as_array() {
                for child in children {
                    walk_node(child, depth + 1, ctx);
                }
            }
        }
    }
}

/// Process an element node (nodeType=1).
fn walk_element(node: &Json, depth: usize, ctx: &mut WalkContext) {
    let tag = node["nodeName"]
        .as_str()
        .unwrap_or("unknown")
        .to_lowercase();

    // Always skip presentational tags
    if PRESENTATIONAL_TAGS.contains(&tag.as_str()) {
        return;
    }

    let attributes = parse_attributes(node);
    let role = infer_role(&tag, &attributes);
    let text = collect_text(node, &tag);
    let is_visible = is_element_visible(&attributes, node);

    // Decide whether to keep this element based on mode
    let is_interactive = INTERACTIVE_TAGS.contains(&tag.as_str());

    let keep = match ctx.mode {
        DistillMode::TextOnly => {
            // Keep only elements that have visible text content
            !text.trim().is_empty()
        }
        DistillMode::InputFields => {
            // Keep text-bearing AND interactive elements
            !text.trim().is_empty() || is_interactive
        }
        DistillMode::AllFields => {
            // Keep everything (except presentational tags, already filtered)
            true
        }
    };

    if !keep {
        // Still recurse into children — they might be keepable
        if let Some(children) = node["children"].as_array() {
            for child in children {
                walk_node(child, depth + 1, ctx);
            }
        }
        return;
    }

    // Walk children first so we know their indices
    let child_start = ctx.elements.len();
    if let Some(children) = node["children"].as_array() {
        for child in children {
            walk_node(child, depth + 1, ctx);
        }
    }
    let child_end = ctx.elements.len();

    let children: Vec<usize> = (child_start..child_end).collect();

    let node_id = node["nodeId"].as_i64().unwrap_or(-1) as i32;

    let element = DistilledElement {
        tag,
        text,
        attributes,
        role,
        is_visible,
        bounding_box: None, // Bounding boxes require separate CDP calls (Phase 2 refinement)
        children,
    };

    if ctx.at_capacity() {
        return;
    }

    let idx = ctx.elements.len();
    ctx.node_map.insert(node_id, idx);
    ctx.elements.push(element);

    // Identify interactive elements
    if is_interactive {
        let el = &ctx.elements[idx];
        if let Some(interactive) = build_interactive(el, idx) {
            ctx.interactive.push(interactive);
        }
    }
}

/// Parse CDP attributes array `["key1", "val1", "key2", "val2"]` into a `HashMap`.
fn parse_attributes(node: &Json) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if let Some(arr) = node["attributes"].as_array() {
        let mut i = 0;
        while i + 1 < arr.len() {
            if let (Some(k), Some(v)) = (arr[i].as_str(), arr[i + 1].as_str()) {
                map.insert(k.to_string(), v.to_string());
            }
            i += 2;
        }
    }
    map
}

/// Infer an ARIA role from tag name and explicit role attribute.
fn infer_role(tag: &str, attrs: &HashMap<String, String>) -> Option<String> {
    // Explicit role attribute takes precedence
    if let Some(role) = attrs.get("role") {
        if !role.is_empty() {
            return Some(role.clone());
        }
    }

    // Implicit roles from HTML spec
    match tag {
        "a" if attrs.get("href").is_some() => Some("link".into()),
        "button" => Some("button".into()),
        "nav" => Some("navigation".into()),
        "main" => Some("main".into()),
        "form" => Some("form".into()),
        "input" => {
            let input_type = attrs.get("type").map_or("text", std::string::String::as_str);
            match input_type {
                "checkbox" => Some("checkbox".into()),
                "radio" => Some("radio".into()),
                "submit" | "button" => Some("button".into()),
                _ => Some("textbox".into()),
            }
        }
        "select" => Some("combobox".into()),
        "textarea" => Some("textbox".into()),
        "img" => Some("img".into()),
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => Some("heading".into()),
        "ul" | "ol" => Some("list".into()),
        "li" => Some("listitem".into()),
        "table" => Some("table".into()),
        "header" => Some("banner".into()),
        "footer" => Some("contentinfo".into()),
        "aside" => Some("complementary".into()),
        _ => None,
    }
}

/// Collect visible text from an element's subtree.
fn collect_text(node: &Json, _tag: &str) -> String {
    let mut buf = String::new();
    collect_text_recursive(node, &mut buf);
    // Normalize whitespace
    let result: String = buf.split_whitespace().collect::<Vec<_>>().join(" ");
    // Truncate for memory safety
    if result.len() > 500 {
        result[..500].to_string()
    } else {
        result
    }
}

fn collect_text_recursive(node: &Json, buf: &mut String) {
    let node_type = node["nodeType"].as_i64().unwrap_or(-1);
    match node_type {
        3 => {
            // Text node
            if let Some(text) = node["nodeValue"].as_str() {
                buf.push_str(text);
            }
        }
        1 => {
            // Element node — recurse into children, add space between elements
            if let Some(children) = node["children"].as_array() {
                for child in children {
                    collect_text_recursive(child, buf);
                    buf.push(' ');
                }
            }
        }
        _ => {
            if let Some(children) = node["children"].as_array() {
                for child in children {
                    collect_text_recursive(child, buf);
                }
            }
        }
    }
}

/// Heuristic visibility check based on common hidden patterns.
fn is_element_visible(attrs: &HashMap<String, String>, _node: &Json) -> bool {
    let style = attrs.get("style").map_or("", std::string::String::as_str);
    let hidden = attrs.get("hidden");
    let aria_hidden = attrs.get("aria-hidden");
    let class = attrs.get("class").map_or("", std::string::String::as_str);

    if hidden.is_some() {
        return false;
    }
    if aria_hidden.is_some_and(|v| v == "true") {
        return false;
    }
    if style.contains("display: none")
        || style.contains("display:none")
        || style.contains("visibility: hidden")
        || style.contains("visibility:hidden")
    {
        return false;
    }
    if class.contains("hidden") || class.contains("sr-only") || class.contains("d-none") {
        return false;
    }

    true
}

/// Build an [`InteractiveElement`] from a distilled element if it's interactive.
fn build_interactive(el: &DistilledElement, _idx: usize) -> Option<InteractiveElement> {
    let element_type = match el.tag.as_str() {
        "a" => InteractiveType::Link,
        "button" => InteractiveType::Button,
        "input" => {
            let input_type = el
                .attributes
                .get("type")
                .map_or("text", std::string::String::as_str);
            match input_type {
                "checkbox" => InteractiveType::Checkbox,
                "radio" => InteractiveType::Radio,
                "submit" | "button" => InteractiveType::Button,
                _ => InteractiveType::Input,
            }
        }
        "select" | "option" => InteractiveType::Select,
        "textarea" => InteractiveType::Textarea,
        _ => return None,
    };

    let selector = build_selector(el);

    Some(InteractiveElement {
        selector,
        element_type,
        label: el.text.clone(),
        placeholder: el.attributes.get("placeholder").cloned(),
        current_value: el.attributes.get("value").cloned(),
        is_visible: el.is_visible,
        is_enabled: !el.attributes.contains_key("disabled"),
        bounding_box: el.bounding_box,
    })
}

/// Build a CSS selector string for an element.
fn build_selector(el: &DistilledElement) -> String {
    if let Some(id) = el.attributes.get("id") {
        if !id.is_empty() && is_valid_css_id(id) {
            return format!("#{id}");
        }
    }

    // Build tag + class selector
    let mut sel = el.tag.clone();
    if let Some(class) = el.attributes.get("class") {
        let classes: Vec<&str> = class
            .split_whitespace()
            .filter(|c| is_valid_css_class(c))
            .take(2)
            .collect();
        if !classes.is_empty() {
            sel.push('.');
            sel.push_str(&classes.join("."));
        }
    }
    sel
}

fn is_valid_css_id(id: &str) -> bool {
    id.chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
}

fn is_valid_css_class(class: &str) -> bool {
    !class.is_empty()
        && class
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
}

/// Detect distractions by scanning semantic blocks for ad/cookie/popup patterns.
fn detect_distractions(
    blocks: &[ans_core::distill::SemanticBlock],
    elements: &[DistilledElement],
) -> Vec<DistractionFlag> {
    let mut flags = Vec::new();

    for block in blocks {
        let (kind, action) = match block.block_type {
            ans_core::distill::BlockType::Ad => (DistractionKind::Ad, ImmuneAction::Suppress),
            ans_core::distill::BlockType::CookieBanner => {
                (DistractionKind::CookieBanner, ImmuneAction::Dismiss)
            }
            ans_core::distill::BlockType::Popup => (DistractionKind::Popup, ImmuneAction::Dismiss),
            _ => continue,
        };

        // Build selector from the first element in the block
        let selector = block
            .element_indices
            .first()
            .and_then(|&i| elements.get(i))
            .map(build_selector)
            .unwrap_or_default();

        flags.push(DistractionFlag {
            kind,
            element_selector: selector,
            confidence: 0.8,
            suggested_action: action,
        });
    }

    flags
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_text_node(text: &str) -> Json {
        serde_json::json!({
            "nodeId": 99,
            "nodeType": 3,
            "nodeName": "#text",
            "nodeValue": text
        })
    }

    fn make_element(tag: &str, attrs: Vec<(&str, &str)>, children: Vec<Json>) -> Json {
        let attr_arr: Vec<String> = attrs
            .iter()
            .flat_map(|(k, v)| vec![k.to_string(), v.to_string()])
            .collect();
        serde_json::json!({
            "nodeId": 1,
            "nodeType": 1,
            "nodeName": tag,
            "attributes": attr_arr,
            "children": children
        })
    }

    #[test]
    fn test_text_only_mode_strips_empty_divs() {
        let raw = serde_json::json!({
            "root": {
                "nodeId": 0,
                "nodeType": 9,
                "nodeName": "#document",
                "children": [
                    make_element("div", vec![], vec![])
                ]
            }
        });

        let d = Distiller;
        let page = d.process(&raw, DistillMode::TextOnly, "about:blank", "Test");
        // Empty div with no text should be stripped in TextOnly mode
        assert!(page.elements.is_empty());
    }

    #[test]
    fn test_text_only_keeps_text_content() {
        let raw = serde_json::json!({
            "root": {
                "nodeId": 0,
                "nodeType": 9,
                "nodeName": "#document",
                "children": [
                    make_element("div", vec![], vec![
                        make_text_node("Hello World")
                    ])
                ]
            }
        });

        let d = Distiller;
        let page = d.process(&raw, DistillMode::TextOnly, "about:blank", "Test");
        assert_eq!(page.elements.len(), 1);
        assert!(page.elements[0].text.contains("Hello World"));
    }

    #[test]
    fn test_input_fields_mode_detects_button() {
        let raw = serde_json::json!({
            "root": {
                "nodeId": 0,
                "nodeType": 9,
                "nodeName": "#document",
                "children": [
                    make_element("button", vec![("id", "submit-btn")], vec![
                        make_text_node("Submit")
                    ])
                ]
            }
        });

        let d = Distiller;
        let page = d.process(&raw, DistillMode::InputFields, "about:blank", "Test");
        assert_eq!(page.interactive.len(), 1);
        assert_eq!(page.interactive[0].element_type, InteractiveType::Button);
    }

    #[test]
    fn test_role_inference() {
        let mut attrs = HashMap::new();
        attrs.insert("type".into(), "checkbox".into());
        let role = infer_role("input", &attrs);
        assert_eq!(role, Some("checkbox".into()));

        let role = infer_role("button", &HashMap::new());
        assert_eq!(role, Some("button".into()));
    }
}
