//! Shared types for inspector bridges
//!
//! This module provides common types used across DOM and Render inspector bridges.

use cdp_types::domains::dom::NodeId;
use serde::{Deserialize, Serialize};

/// Highlight configuration for node highlighting
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HighlightConfig {
    /// Whether to show info tooltip
    #[serde(default)]
    pub show_info: bool,
    /// Whether to show rulers
    #[serde(default)]
    pub show_rulers: bool,
    /// Whether to show accessibility info
    #[serde(default)]
    pub show_accessibility_info: bool,
    /// Whether to show extension lines
    #[serde(default)]
    pub show_extension_lines: bool,
    /// Content box highlight color
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_color: Option<RGBA>,
    /// Padding highlight color
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding_color: Option<RGBA>,
    /// Border highlight color
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_color: Option<RGBA>,
    /// Margin highlight color
    #[serde(skip_serializing_if = "Option::is_none")]
    pub margin_color: Option<RGBA>,
}

impl Default for HighlightConfig {
    fn default() -> Self {
        Self {
            show_info: true,
            show_rulers: false,
            show_accessibility_info: false,
            show_extension_lines: false,
            content_color: Some(RGBA::new(111, 168, 220, 0.66)),
            padding_color: Some(RGBA::new(147, 196, 125, 0.55)),
            border_color: Some(RGBA::new(255, 229, 153, 0.66)),
            margin_color: Some(RGBA::new(246, 178, 107, 0.66)),
        }
    }
}

/// RGBA color representation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RGBA {
    /// Red component (0-255)
    pub r: u8,
    /// Green component (0-255)
    pub g: u8,
    /// Blue component (0-255)
    pub b: u8,
    /// Alpha component (0-1)
    pub a: f64,
}

impl RGBA {
    /// Create a new RGBA color
    pub fn new(r: u8, g: u8, b: u8, a: f64) -> Self {
        Self { r, g, b, a }
    }
}

/// Box model representation for an element
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BoxModel {
    /// Content box quad (4 points)
    pub content: Quad,
    /// Padding box quad (4 points)
    pub padding: Quad,
    /// Border box quad (4 points)
    pub border: Quad,
    /// Margin box quad (4 points)
    pub margin: Quad,
    /// Element width
    pub width: f64,
    /// Element height
    pub height: f64,
    /// Shape outside info
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shape_outside: Option<ShapeOutsideInfo>,
}

/// Quad representation (4 points: x1,y1,x2,y2,x3,y3,x4,y4)
pub type Quad = [f64; 8];

/// Shape outside info
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ShapeOutsideInfo {
    /// Shape bounds
    pub bounds: Quad,
    /// Shape path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shape: Option<Vec<serde_json::Value>>,
    /// Margin shape path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub margin_shape: Option<Vec<serde_json::Value>>,
}

/// DOM mutation type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MutationType {
    /// Child node added
    ChildListAdded,
    /// Child node removed
    ChildListRemoved,
    /// Attribute changed
    AttributeModified,
    /// Character data changed
    CharacterDataModified,
    /// Subtree modified
    SubtreeModified,
}

/// DOM mutation record
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MutationRecord {
    /// Type of mutation
    pub mutation_type: MutationType,
    /// Target node ID
    pub target_node_id: NodeId,
    /// Added node IDs (for child list mutations)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub added_node_ids: Option<Vec<NodeId>>,
    /// Removed node IDs (for child list mutations)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub removed_node_ids: Option<Vec<NodeId>>,
    /// Previous sibling node ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_sibling_id: Option<NodeId>,
    /// Attribute name (for attribute mutations)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attribute_name: Option<String>,
    /// Attribute old value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_value: Option<String>,
}

/// Layer information for render tree
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LayerInfo {
    /// Layer ID
    pub layer_id: String,
    /// Parent layer ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_layer_id: Option<String>,
    /// Associated DOM node ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<NodeId>,
    /// Layer offset X
    pub offset_x: f64,
    /// Layer offset Y
    pub offset_y: f64,
    /// Layer width
    pub width: f64,
    /// Layer height
    pub height: f64,
    /// Transform matrix
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transform: Option<[f64; 16]>,
    /// Compositing reasons
    #[serde(default)]
    pub compositing_reasons: Vec<String>,
    /// Whether layer paints to display
    #[serde(default)]
    pub draws_content: bool,
}

/// Search result for node search
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    /// Search ID
    pub search_id: String,
    /// Number of results
    pub result_count: u32,
}

/// Node selection state
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SelectionState {
    /// Currently selected node
    pub selected_node: Option<NodeId>,
    /// Currently highlighted node
    pub highlighted_node: Option<NodeId>,
    /// Highlight configuration
    pub highlight_config: HighlightConfig,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgba_new() {
        let color = RGBA::new(255, 128, 64, 0.5);
        assert_eq!(color.r, 255);
        assert_eq!(color.g, 128);
        assert_eq!(color.b, 64);
        assert!((color.a - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_highlight_config_default() {
        let config = HighlightConfig::default();
        assert!(config.show_info);
        assert!(!config.show_rulers);
        assert!(config.content_color.is_some());
    }

    #[test]
    fn test_box_model_serialization() {
        let box_model = BoxModel {
            content: [0.0, 0.0, 100.0, 0.0, 100.0, 100.0, 0.0, 100.0],
            padding: [0.0, 0.0, 100.0, 0.0, 100.0, 100.0, 0.0, 100.0],
            border: [0.0, 0.0, 100.0, 0.0, 100.0, 100.0, 0.0, 100.0],
            margin: [0.0, 0.0, 100.0, 0.0, 100.0, 100.0, 0.0, 100.0],
            width: 100.0,
            height: 100.0,
            shape_outside: None,
        };

        let json = serde_json::to_string(&box_model).expect("Failed to serialize");
        assert!(json.contains("\"content\""));
        assert!(json.contains("\"width\":100"));
    }

    #[test]
    fn test_mutation_record_serialization() {
        let record = MutationRecord {
            mutation_type: MutationType::AttributeModified,
            target_node_id: NodeId(1),
            added_node_ids: None,
            removed_node_ids: None,
            previous_sibling_id: None,
            attribute_name: Some("class".to_string()),
            old_value: Some("old-class".to_string()),
        };

        let json = serde_json::to_string(&record).expect("Failed to serialize");
        assert!(json.contains("attributeModified"));
        assert!(json.contains("class"));
    }

    #[test]
    fn test_layer_info_creation() {
        let layer = LayerInfo {
            layer_id: "layer-1".to_string(),
            parent_layer_id: None,
            node_id: Some(NodeId(1)),
            offset_x: 0.0,
            offset_y: 0.0,
            width: 800.0,
            height: 600.0,
            transform: None,
            compositing_reasons: vec!["root".to_string()],
            draws_content: true,
        };

        assert_eq!(layer.layer_id, "layer-1");
        assert!(layer.draws_content);
    }

    #[test]
    fn test_selection_state_default() {
        let state = SelectionState::default();
        assert!(state.selected_node.is_none());
        assert!(state.highlighted_node.is_none());
        assert!(state.highlight_config.show_info);
    }
}
