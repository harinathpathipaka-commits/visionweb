//! Diff-specific types: tree nodes, edit operations, cost functions.
//!
//! Each DOM element becomes a `DiffNode` with a label derived from its
//! tag + text prefix + role. The element-identity matching algorithm
//! uses these labels to pair elements across page snapshots.

use ans_core::distill::DistilledElement;

/// A single edit operation produced by the differ.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditOp {
    Insert {
        index: usize,
        label: String,
    },
    Delete {
        index: usize,
    },
    Relabel {
        index: usize,
        old: String,
        new: String,
    },
}

/// A node in the labeled ordered tree used for diffing.
#[derive(Debug, Clone)]
pub struct DiffNode {
    pub id: usize,
    /// Identity label: `format!("{tag}|{text_prefix}|{role}")`
    pub label: String,
    /// Indices into the tree's node array for children.
    pub children: Vec<usize>,
    /// Index into the source `DistilledPage.elements`.
    pub element_index: usize,
}

/// An element that appeared in the new page but not in the old.
#[derive(Debug, Clone)]
pub struct ElementChange {
    pub tag: String,
    pub text: String,
    pub selector: String,
    pub index: i32,
}

/// An element whose attributes changed between snapshots.
#[derive(Debug, Clone)]
pub struct ElementModification {
    pub selector: String,
    pub attribute_name: String,
    pub old_value: String,
    pub new_value: String,
}

/// Build an identity label from a distilled element.
#[must_use] 
pub fn identity_label(el: &DistilledElement) -> String {
    let text_prefix: String = el.text.chars().take(50).collect();
    let role = el.role.as_deref().unwrap_or("");
    format!("{}|{}|{}", el.tag, text_prefix, role)
}

/// Build a CSS selector from element attributes.
#[must_use] 
pub fn element_selector(el: &DistilledElement) -> String {
    if let Some(id) = el.attributes.get("id") {
        if !id.is_empty() && is_valid_css_id(id) {
            return format!("#{id}");
        }
    }
    if let Some(class) = el.attributes.get("class") {
        let classes: Vec<&str> = class
            .split_whitespace()
            .filter(|c| is_valid_css_class(c))
            .take(2)
            .collect();
        if !classes.is_empty() {
            return format!("{}.{}", el.tag, classes.join("."));
        }
    }
    el.tag.clone()
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

/// Build a flat tree array from `DistilledPage` elements.
///
/// The elements are already stored in preorder (from the DOM walk),
/// so the tree structure is encoded in the `children` indices.
#[must_use] 
pub fn build_tree(elements: &[DistilledElement]) -> Vec<DiffNode> {
    elements
        .iter()
        .enumerate()
        .map(|(i, el)| DiffNode {
            id: i,
            label: identity_label(el),
            children: el.children.clone(),
            element_index: i,
        })
        .collect()
}

/// Diff output produced by [`PageDiffer::diff`].
#[derive(Debug, Clone)]
pub struct DiffOutput {
    pub added: Vec<ElementChange>,
    pub removed: Vec<ElementChange>,
    pub modified: Vec<ElementModification>,
    pub visual_diff_pct: f32,
    pub summary: String,
    pub is_same_page: bool,
}

impl DiffOutput {
    /// No changes detected.
    #[must_use] 
    pub fn empty() -> Self {
        Self {
            added: vec![],
            removed: vec![],
            modified: vec![],
            visual_diff_pct: 0.0,
            summary: "no_change".into(),
            is_same_page: true,
        }
    }
}
