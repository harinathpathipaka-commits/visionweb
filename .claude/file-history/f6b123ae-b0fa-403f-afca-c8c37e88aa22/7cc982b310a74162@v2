//! Serialization between core domain types and protobuf-generated types.
//!
//! Converts [`DistilledPage`] ↔ [`DistilledDom`] for the gRPC server.

use ans_core::distill::{
    BlockType, BoundingBox, DistillMode, DistilledElement, DistilledPage, InteractiveElement,
    InteractiveType,
};
use ans_core::immune::{DistractionFlag, DistractionKind, ImmuneAction};

// ── Core → Proto ──────────────────────────────────────────────────────

/// Convert a core [`DistilledPage`] into the protobuf [`DistilledDom`] message.
pub fn distilled_page_to_proto(page: &DistilledPage) -> ans_proto::ans::DistilledDom {
    ans_proto::ans::DistilledDom {
        mode: page.mode.as_str().to_string(),
        url: page.url.clone(),
        title: page.title.clone(),
        elements: page.elements.iter().map(element_to_proto).collect(),
        interactive: page.interactive.iter().map(interactive_to_proto).collect(),
        semantic_blocks: page
            .semantic_blocks
            .iter()
            .map(semantic_block_to_proto)
            .collect(),
        distraction_flags: page
            .distraction_flags
            .iter()
            .map(distraction_to_proto)
            .collect(),
        timestamp: page.timestamp.timestamp_millis(),
    }
}

fn element_to_proto(el: &DistilledElement) -> ans_proto::ans::DistilledElement {
    ans_proto::ans::DistilledElement {
        tag: el.tag.clone(),
        text: el.text.clone(),
        attributes: el.attributes.clone(),
        role: el.role.clone().unwrap_or_default(),
        is_visible: el.is_visible,
        bounding_box: el.bounding_box.map(bbox_to_proto),
        children: el.children.iter().map(|&c| c as i32).collect(),
    }
}

fn interactive_to_proto(el: &InteractiveElement) -> ans_proto::ans::InteractiveElement {
    ans_proto::ans::InteractiveElement {
        selector: el.selector.clone(),
        element_type: interactive_type_to_str(&el.element_type).to_string(),
        label: el.label.clone(),
        placeholder: el.placeholder.clone().unwrap_or_default(),
        current_value: el.current_value.clone().unwrap_or_default(),
        is_visible: el.is_visible,
        is_enabled: el.is_enabled,
        bounding_box: el.bounding_box.map(bbox_to_proto),
    }
}

fn semantic_block_to_proto(
    block: &ans_core::distill::SemanticBlock,
) -> ans_proto::ans::SemanticBlock {
    ans_proto::ans::SemanticBlock {
        block_type: block_type_to_str(&block.block_type).to_string(),
        text_content: block.text_content.clone(),
        element_indices: block.element_indices.iter().map(|&i| i as i32).collect(),
        goal_relevance_score: block.goal_relevance_score,
    }
}

/// Convert a core [`DistractionFlag`] to the proto message.
#[must_use] 
pub fn distraction_to_proto(flag: &DistractionFlag) -> ans_proto::ans::DistractionFlag {
    ans_proto::ans::DistractionFlag {
        kind: distraction_kind_to_str(&flag.kind).to_string(),
        element_selector: flag.element_selector.clone(),
        confidence: flag.confidence,
        suggested_action: immune_action_to_str(&flag.suggested_action).to_string(),
    }
}

const fn bbox_to_proto(bb: BoundingBox) -> ans_proto::ans::BoundingBox {
    ans_proto::ans::BoundingBox {
        x: bb.x,
        y: bb.y,
        width: bb.width,
        height: bb.height,
    }
}

// ── Proto → Core ──────────────────────────────────────────────────────

/// Convert a proto [`DistilledDom`] back to a core [`DistilledPage`].
pub fn proto_to_distilled_page(proto: &ans_proto::ans::DistilledDom) -> DistilledPage {
    DistilledPage {
        mode: str_to_distill_mode(&proto.mode),
        url: proto.url.clone(),
        title: proto.title.clone(),
        elements: proto.elements.iter().map(proto_to_element).collect(),
        interactive: proto.interactive.iter().map(proto_to_interactive).collect(),
        semantic_blocks: proto
            .semantic_blocks
            .iter()
            .map(proto_to_semantic_block)
            .collect(),
        distraction_flags: proto
            .distraction_flags
            .iter()
            .map(proto_to_distraction_flag)
            .collect(),
        timestamp: chrono::DateTime::from_timestamp_millis(proto.timestamp)
            .unwrap_or_else(chrono::Utc::now),
    }
}

fn proto_to_element(el: &ans_proto::ans::DistilledElement) -> DistilledElement {
    DistilledElement {
        tag: el.tag.clone(),
        text: el.text.clone(),
        attributes: el.attributes.clone(),
        role: if el.role.is_empty() {
            None
        } else {
            Some(el.role.clone())
        },
        is_visible: el.is_visible,
        bounding_box: el.bounding_box.as_ref().map(|bb| BoundingBox {
            x: bb.x,
            y: bb.y,
            width: bb.width,
            height: bb.height,
        }),
        children: el.children.iter().map(|&c| c as usize).collect(),
    }
}

fn proto_to_interactive(el: &ans_proto::ans::InteractiveElement) -> InteractiveElement {
    InteractiveElement {
        selector: el.selector.clone(),
        element_type: str_to_interactive_type(&el.element_type),
        label: el.label.clone(),
        placeholder: if el.placeholder.is_empty() {
            None
        } else {
            Some(el.placeholder.clone())
        },
        current_value: if el.current_value.is_empty() {
            None
        } else {
            Some(el.current_value.clone())
        },
        is_visible: el.is_visible,
        is_enabled: el.is_enabled,
        bounding_box: el.bounding_box.as_ref().map(|bb| BoundingBox {
            x: bb.x,
            y: bb.y,
            width: bb.width,
            height: bb.height,
        }),
    }
}

fn proto_to_semantic_block(
    block: &ans_proto::ans::SemanticBlock,
) -> ans_core::distill::SemanticBlock {
    ans_core::distill::SemanticBlock {
        block_type: str_to_block_type(&block.block_type),
        text_content: block.text_content.clone(),
        element_indices: block.element_indices.iter().map(|&i| i as usize).collect(),
        goal_relevance_score: block.goal_relevance_score,
    }
}

fn proto_to_distraction_flag(flag: &ans_proto::ans::DistractionFlag) -> DistractionFlag {
    DistractionFlag {
        kind: str_to_distraction_kind(&flag.kind),
        element_selector: flag.element_selector.clone(),
        confidence: flag.confidence,
        suggested_action: str_to_immune_action(&flag.suggested_action),
    }
}

// ── String ↔ Enum converters ──────────────────────────────────────────

fn str_to_distill_mode(s: &str) -> DistillMode {
    match s {
        "text_only" => DistillMode::TextOnly,
        "input_fields" => DistillMode::InputFields,
        _ => DistillMode::AllFields,
    }
}

const fn interactive_type_to_str(t: &InteractiveType) -> &'static str {
    match t {
        InteractiveType::Button => "button",
        InteractiveType::Input => "input",
        InteractiveType::Select => "select",
        InteractiveType::Textarea => "textarea",
        InteractiveType::Link => "link",
        InteractiveType::Checkbox => "checkbox",
        InteractiveType::Radio => "radio",
    }
}

fn str_to_interactive_type(s: &str) -> InteractiveType {
    match s {
        "input" => InteractiveType::Input,
        "select" => InteractiveType::Select,
        "textarea" => InteractiveType::Textarea,
        "link" => InteractiveType::Link,
        "checkbox" => InteractiveType::Checkbox,
        "radio" => InteractiveType::Radio,
        _ => InteractiveType::Button,
    }
}

const fn block_type_to_str(t: &BlockType) -> &'static str {
    match t {
        BlockType::Navigation => "navigation",
        BlockType::MainContent => "main_content",
        BlockType::Form => "form",
        BlockType::Sidebar => "sidebar",
        BlockType::Footer => "footer",
        BlockType::Ad => "ad",
        BlockType::CookieBanner => "cookie_banner",
        BlockType::Popup => "popup",
        BlockType::Unknown => "unknown",
    }
}

fn str_to_block_type(s: &str) -> BlockType {
    match s {
        "navigation" => BlockType::Navigation,
        "main_content" => BlockType::MainContent,
        "form" => BlockType::Form,
        "sidebar" => BlockType::Sidebar,
        "footer" => BlockType::Footer,
        "ad" => BlockType::Ad,
        "cookie_banner" => BlockType::CookieBanner,
        "popup" => BlockType::Popup,
        _ => BlockType::Unknown,
    }
}

const fn distraction_kind_to_str(k: &DistractionKind) -> &'static str {
    match k {
        DistractionKind::Ad => "ad",
        DistractionKind::Popup => "popup",
        DistractionKind::CookieBanner => "cookie_banner",
        DistractionKind::NewsletterModal => "newsletter_modal",
        DistractionKind::Redirect => "redirect",
        DistractionKind::AutoPlayVideo => "autoplay_video",
        DistractionKind::Survey => "survey",
        DistractionKind::Notification => "notification",
        DistractionKind::Unknown => "unknown",
    }
}

fn str_to_distraction_kind(s: &str) -> DistractionKind {
    match s {
        "ad" => DistractionKind::Ad,
        "popup" => DistractionKind::Popup,
        "cookie_banner" => DistractionKind::CookieBanner,
        "newsletter_modal" => DistractionKind::NewsletterModal,
        "redirect" => DistractionKind::Redirect,
        "autoplay_video" => DistractionKind::AutoPlayVideo,
        "survey" => DistractionKind::Survey,
        "notification" => DistractionKind::Notification,
        _ => DistractionKind::Unknown,
    }
}

const fn immune_action_to_str(a: &ImmuneAction) -> &'static str {
    match a {
        ImmuneAction::Dismiss => "dismiss",
        ImmuneAction::Block => "block",
        ImmuneAction::Suppress => "suppress",
        ImmuneAction::Ignore => "ignore",
        ImmuneAction::NavigateBack => "navigate_back",
    }
}

fn str_to_immune_action(s: &str) -> ImmuneAction {
    match s {
        "dismiss" => ImmuneAction::Dismiss,
        "block" => ImmuneAction::Block,
        "suppress" => ImmuneAction::Suppress,
        "navigate_back" => ImmuneAction::NavigateBack,
        _ => ImmuneAction::Ignore,
    }
}
