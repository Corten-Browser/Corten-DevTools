//! Accessibility Inspector implementation
//!
//! Provides accessibility tree inspection via Chrome DevTools Protocol.
//! Implements FEAT-024: Accessibility Inspector.
//!
//! Features:
//! - Accessibility tree inspection
//! - ARIA attributes inspection
//! - Contrast ratio checking

use async_trait::async_trait;
use cdp_types::domains::dom::NodeId;
use cdp_types::CdpError;
use protocol_handler::DomainHandler;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn};

use crate::mock_browser::MockBrowser;
use crate::types::RGBA;

/// Accessibility tree node
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AXNode {
    /// Unique node ID
    pub node_id: String,
    /// Whether this node is ignored for accessibility
    #[serde(default)]
    pub ignored: bool,
    /// Ignored reasons
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignored_reasons: Option<Vec<AXProperty>>,
    /// Role of the node
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<AXValue>,
    /// Name of the node
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<AXValue>,
    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<AXValue>,
    /// Value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<AXValue>,
    /// Properties
    #[serde(default)]
    pub properties: Vec<AXProperty>,
    /// Child node IDs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub child_ids: Option<Vec<String>>,
    /// Backend DOM node ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend_dom_node_id: Option<NodeId>,
}

/// Accessibility value
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AXValue {
    /// Value type
    #[serde(rename = "type")]
    pub value_type: AXValueType,
    /// Value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
    /// Related nodes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub related_nodes: Option<Vec<AXRelatedNode>>,
    /// Sources of the value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sources: Option<Vec<AXValueSource>>,
}

/// Accessibility value type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum AXValueType {
    Boolean,
    Tristate,
    BooleanOrUndefined,
    Idref,
    IdrefList,
    Integer,
    Node,
    NodeList,
    Number,
    String,
    ComputedString,
    Token,
    TokenList,
    DomRelation,
    Role,
    InternalRole,
    ValueUndefined,
}

/// Accessibility property
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AXProperty {
    /// Property name
    pub name: AXPropertyName,
    /// Property value
    pub value: AXValue,
}

/// Accessibility property name
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum AXPropertyName {
    // States
    Busy,
    Disabled,
    Editable,
    Focusable,
    Focused,
    Hidden,
    HiddenRoot,
    Invalid,
    Keyshortcuts,
    Settable,
    Roledescription,
    Live,
    Atomic,
    Relevant,
    Root,
    Autocomplete,
    HasPopup,
    Level,
    Multiselectable,
    Orientation,
    Multiline,
    Readonly,
    Required,
    Valuemin,
    Valuemax,
    Valuetext,
    Checked,
    Expanded,
    Modal,
    Pressed,
    Selected,
    // Custom property for ignored reason
    Reason,
}

/// Accessibility related node
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AXRelatedNode {
    /// Backend DOM node ID
    pub backend_dom_node_id: NodeId,
    /// ID reference
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idref: Option<String>,
    /// Text
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// Accessibility value source
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AXValueSource {
    /// Source type
    #[serde(rename = "type")]
    pub source_type: AXValueSourceType,
    /// Value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<AXValue>,
    /// Attribute name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attribute: Option<String>,
    /// Attribute value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attribute_value: Option<AXValue>,
    /// Whether superseded
    #[serde(default)]
    pub superseded: bool,
    /// Native source
    #[serde(skip_serializing_if = "Option::is_none")]
    pub native_source: Option<AXValueNativeSourceType>,
    /// Native source value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub native_source_value: Option<AXValue>,
    /// Invalid
    #[serde(default)]
    pub invalid: bool,
    /// Invalid reason
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invalid_reason: Option<String>,
}

/// Accessibility value source type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum AXValueSourceType {
    Attribute,
    Implicit,
    Style,
    Contents,
    Placeholder,
    RelatedElement,
}

/// Accessibility native source type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum AXValueNativeSourceType {
    Description,
    Figcaption,
    Label,
    Labelfor,
    Labelwrapped,
    Legend,
    Rubyannotation,
    Tablecaption,
    Title,
    Other,
}

/// Contrast information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ContrastInfo {
    /// Contrast ratio
    pub contrast_ratio: f64,
    /// Foreground color
    pub foreground_color: RGBA,
    /// Background color
    pub background_color: RGBA,
    /// Whether AA compliant for normal text
    pub aa_normal: bool,
    /// Whether AA compliant for large text
    pub aa_large: bool,
    /// Whether AAA compliant for normal text
    pub aaa_normal: bool,
    /// Whether AAA compliant for large text
    pub aaa_large: bool,
}

/// ARIA attributes info
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AriaAttributesInfo {
    /// Node ID
    pub node_id: NodeId,
    /// Role
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// ARIA label
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aria_label: Option<String>,
    /// ARIA labelledby
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aria_labelledby: Option<String>,
    /// ARIA describedby
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aria_describedby: Option<String>,
    /// ARIA live
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aria_live: Option<String>,
    /// ARIA expanded
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aria_expanded: Option<bool>,
    /// ARIA hidden
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aria_hidden: Option<bool>,
    /// ARIA disabled
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aria_disabled: Option<bool>,
    /// All ARIA attributes
    pub all_attributes: Vec<AriaAttribute>,
}

/// Single ARIA attribute
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AriaAttribute {
    /// Attribute name
    pub name: String,
    /// Attribute value
    pub value: String,
}

/// Accessibility inspector state
#[derive(Debug, Clone, Default)]
pub struct AccessibilityState {
    /// Whether accessibility inspection is enabled
    pub enabled: bool,
    /// Selected node for inspection
    pub selected_node: Option<NodeId>,
}

/// Accessibility Inspector
///
/// Provides accessibility tree inspection, ARIA attributes inspection,
/// and contrast ratio checking.
pub struct AccessibilityInspector {
    /// Mock browser for testing
    browser: Arc<MockBrowser>,
    /// Inspector state
    state: Arc<RwLock<AccessibilityState>>,
}

impl AccessibilityInspector {
    /// Create a new Accessibility Inspector
    pub fn new() -> Self {
        Self {
            browser: Arc::new(MockBrowser::new()),
            state: Arc::new(RwLock::new(AccessibilityState::default())),
        }
    }

    /// Create with custom browser (for testing)
    pub fn with_browser(browser: Arc<MockBrowser>) -> Self {
        Self {
            browser,
            state: Arc::new(RwLock::new(AccessibilityState::default())),
        }
    }

    /// Enable accessibility inspection
    async fn enable(&self, _params: Option<Value>) -> Result<Value, CdpError> {
        debug!("AccessibilityInspector.enable called");

        let mut state = self.state.write().await;
        state.enabled = true;

        Ok(serde_json::json!({}))
    }

    /// Disable accessibility inspection
    async fn disable(&self, _params: Option<Value>) -> Result<Value, CdpError> {
        debug!("AccessibilityInspector.disable called");

        let mut state = self.state.write().await;
        state.enabled = false;
        state.selected_node = None;

        Ok(serde_json::json!({}))
    }

    /// Get accessibility tree
    async fn get_full_ax_tree(&self, _params: Option<Value>) -> Result<Value, CdpError> {
        debug!("AccessibilityInspector.getFullAXTree called");

        // Build mock accessibility tree
        let nodes = vec![
            AXNode {
                node_id: "ax-1".to_string(),
                ignored: false,
                ignored_reasons: None,
                role: Some(AXValue {
                    value_type: AXValueType::Role,
                    value: Some(serde_json::json!("WebArea")),
                    related_nodes: None,
                    sources: None,
                }),
                name: Some(AXValue {
                    value_type: AXValueType::ComputedString,
                    value: Some(serde_json::json!("Test Page")),
                    related_nodes: None,
                    sources: None,
                }),
                description: None,
                value: None,
                properties: vec![],
                child_ids: Some(vec!["ax-2".to_string(), "ax-3".to_string()]),
                backend_dom_node_id: Some(NodeId(1)),
            },
            AXNode {
                node_id: "ax-2".to_string(),
                ignored: false,
                ignored_reasons: None,
                role: Some(AXValue {
                    value_type: AXValueType::Role,
                    value: Some(serde_json::json!("heading")),
                    related_nodes: None,
                    sources: None,
                }),
                name: Some(AXValue {
                    value_type: AXValueType::ComputedString,
                    value: Some(serde_json::json!("Hello World")),
                    related_nodes: None,
                    sources: None,
                }),
                description: None,
                value: None,
                properties: vec![AXProperty {
                    name: AXPropertyName::Level,
                    value: AXValue {
                        value_type: AXValueType::Integer,
                        value: Some(serde_json::json!(1)),
                        related_nodes: None,
                        sources: None,
                    },
                }],
                child_ids: None,
                backend_dom_node_id: Some(NodeId(4)),
            },
            AXNode {
                node_id: "ax-3".to_string(),
                ignored: false,
                ignored_reasons: None,
                role: Some(AXValue {
                    value_type: AXValueType::Role,
                    value: Some(serde_json::json!("generic")),
                    related_nodes: None,
                    sources: None,
                }),
                name: None,
                description: None,
                value: None,
                properties: vec![],
                child_ids: Some(vec!["ax-4".to_string()]),
                backend_dom_node_id: Some(NodeId(6)),
            },
            AXNode {
                node_id: "ax-4".to_string(),
                ignored: false,
                ignored_reasons: None,
                role: Some(AXValue {
                    value_type: AXValueType::Role,
                    value: Some(serde_json::json!("paragraph")),
                    related_nodes: None,
                    sources: None,
                }),
                name: Some(AXValue {
                    value_type: AXValueType::ComputedString,
                    value: Some(serde_json::json!("This is a test page")),
                    related_nodes: None,
                    sources: None,
                }),
                description: None,
                value: None,
                properties: vec![],
                child_ids: None,
                backend_dom_node_id: Some(NodeId(7)),
            },
        ];

        Ok(serde_json::json!({
            "nodes": nodes
        }))
    }

    /// Get accessibility node for a DOM node
    async fn get_ax_node_for_dom_node(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("AccessibilityInspector.getAXNodeForDOMNode called");

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Params {
            node_id: NodeId,
        }

        let params: Params = serde_json::from_value(
            params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?,
        )
        .map_err(|e| CdpError::invalid_params(format!("Invalid parameters: {}", e)))?;

        if !self.browser.node_exists(params.node_id) {
            return Err(CdpError::server_error(
                -32000,
                format!("Node {} not found", params.node_id.0),
            ));
        }

        let node = self.browser.get_node(params.node_id).ok_or_else(|| {
            CdpError::server_error(-32000, format!("Node {} not found", params.node_id.0))
        })?;

        // Generate mock AX node based on DOM node
        let ax_node = AXNode {
            node_id: format!("ax-{}", params.node_id.0),
            ignored: false,
            ignored_reasons: None,
            role: Some(AXValue {
                value_type: AXValueType::Role,
                value: Some(serde_json::json!(match node.node_name.to_lowercase().as_str() {
                    "div" => "generic",
                    "h1" => "heading",
                    "p" => "paragraph",
                    "button" => "button",
                    "a" => "link",
                    "input" => "textbox",
                    "html" => "WebArea",
                    "body" => "generic",
                    _ => "generic",
                })),
                related_nodes: None,
                sources: None,
            }),
            name: None,
            description: None,
            value: None,
            properties: vec![],
            child_ids: None,
            backend_dom_node_id: Some(params.node_id),
        };

        Ok(serde_json::json!({
            "node": ax_node
        }))
    }

    /// Get ARIA attributes for a DOM node
    async fn get_aria_attributes(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("AccessibilityInspector.getAriaAttributes called");

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Params {
            node_id: NodeId,
        }

        let params: Params = serde_json::from_value(
            params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?,
        )
        .map_err(|e| CdpError::invalid_params(format!("Invalid parameters: {}", e)))?;

        if !self.browser.node_exists(params.node_id) {
            return Err(CdpError::server_error(
                -32000,
                format!("Node {} not found", params.node_id.0),
            ));
        }

        let node = self.browser.get_node(params.node_id).ok_or_else(|| {
            CdpError::server_error(-32000, format!("Node {} not found", params.node_id.0))
        })?;

        // Extract ARIA attributes from node attributes
        let mut aria_info = AriaAttributesInfo {
            node_id: params.node_id,
            role: None,
            aria_label: None,
            aria_labelledby: None,
            aria_describedby: None,
            aria_live: None,
            aria_expanded: None,
            aria_hidden: None,
            aria_disabled: None,
            all_attributes: vec![],
        };

        if let Some(attrs) = &node.attributes {
            for i in (0..attrs.len()).step_by(2) {
                if let (Some(name), Some(value)) = (attrs.get(i), attrs.get(i + 1)) {
                    if name == "role" {
                        aria_info.role = Some(value.clone());
                    } else if name.starts_with("aria-") {
                        aria_info.all_attributes.push(AriaAttribute {
                            name: name.clone(),
                            value: value.clone(),
                        });

                        match name.as_str() {
                            "aria-label" => aria_info.aria_label = Some(value.clone()),
                            "aria-labelledby" => aria_info.aria_labelledby = Some(value.clone()),
                            "aria-describedby" => aria_info.aria_describedby = Some(value.clone()),
                            "aria-live" => aria_info.aria_live = Some(value.clone()),
                            "aria-expanded" => aria_info.aria_expanded = Some(value == "true"),
                            "aria-hidden" => aria_info.aria_hidden = Some(value == "true"),
                            "aria-disabled" => aria_info.aria_disabled = Some(value == "true"),
                            _ => {}
                        }
                    }
                }
            }
        }

        serde_json::to_value(aria_info)
            .map_err(|e| CdpError::internal_error(format!("Serialization error: {}", e)))
    }

    /// Get contrast information for a node
    async fn get_contrast_info(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("AccessibilityInspector.getContrastInfo called");

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Params {
            node_id: NodeId,
        }

        let params: Params = serde_json::from_value(
            params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?,
        )
        .map_err(|e| CdpError::invalid_params(format!("Invalid parameters: {}", e)))?;

        if !self.browser.node_exists(params.node_id) {
            return Err(CdpError::server_error(
                -32000,
                format!("Node {} not found", params.node_id.0),
            ));
        }

        // Return mock contrast info
        // In real implementation, this would calculate actual contrast
        let contrast_info = ContrastInfo {
            contrast_ratio: 4.5,
            foreground_color: RGBA::new(0, 0, 0, 1.0),
            background_color: RGBA::new(255, 255, 255, 1.0),
            aa_normal: true,
            aa_large: true,
            aaa_normal: false,
            aaa_large: true,
        };

        serde_json::to_value(contrast_info)
            .map_err(|e| CdpError::internal_error(format!("Serialization error: {}", e)))
    }

    /// Query accessibility tree
    async fn query_ax_tree(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("AccessibilityInspector.queryAXTree called");

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Params {
            #[serde(default)]
            accessible_name: Option<String>,
            #[serde(default)]
            role: Option<String>,
        }

        let params: Params = params
            .map(|p| serde_json::from_value(p).ok())
            .flatten()
            .unwrap_or(Params {
                accessible_name: None,
                role: None,
            });

        // Return mock query results
        let mut nodes = vec![];

        // If searching for headings
        if params.role.as_deref() == Some("heading") {
            nodes.push(AXNode {
                node_id: "ax-2".to_string(),
                ignored: false,
                ignored_reasons: None,
                role: Some(AXValue {
                    value_type: AXValueType::Role,
                    value: Some(serde_json::json!("heading")),
                    related_nodes: None,
                    sources: None,
                }),
                name: Some(AXValue {
                    value_type: AXValueType::ComputedString,
                    value: Some(serde_json::json!("Hello World")),
                    related_nodes: None,
                    sources: None,
                }),
                description: None,
                value: None,
                properties: vec![],
                child_ids: None,
                backend_dom_node_id: Some(NodeId(4)),
            });
        }

        Ok(serde_json::json!({
            "nodes": nodes
        }))
    }

    /// Get partial accessibility tree
    async fn get_partial_ax_tree(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("AccessibilityInspector.getPartialAXTree called");

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Params {
            node_id: Option<NodeId>,
            #[serde(default)]
            depth: Option<i32>,
        }

        let params: Params = serde_json::from_value(
            params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?,
        )
        .map_err(|e| CdpError::invalid_params(format!("Invalid parameters: {}", e)))?;

        let node_id = params.node_id.unwrap_or(NodeId(1));

        if !self.browser.node_exists(node_id) {
            return Err(CdpError::server_error(
                -32000,
                format!("Node {} not found", node_id.0),
            ));
        }

        // Return partial tree starting from given node
        let node = AXNode {
            node_id: format!("ax-{}", node_id.0),
            ignored: false,
            ignored_reasons: None,
            role: Some(AXValue {
                value_type: AXValueType::Role,
                value: Some(serde_json::json!("generic")),
                related_nodes: None,
                sources: None,
            }),
            name: None,
            description: None,
            value: None,
            properties: vec![],
            child_ids: Some(vec![format!("ax-{}", node_id.0 + 1)]),
            backend_dom_node_id: Some(node_id),
        };

        Ok(serde_json::json!({
            "nodes": [node]
        }))
    }

    /// Get state
    pub async fn get_state(&self) -> AccessibilityState {
        self.state.read().await.clone()
    }

    /// Get browser (for testing)
    pub fn browser(&self) -> &MockBrowser {
        &self.browser
    }
}

impl Default for AccessibilityInspector {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DomainHandler for AccessibilityInspector {
    fn name(&self) -> &str {
        "AccessibilityInspector"
    }

    async fn handle_method(&self, method: &str, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("AccessibilityInspector domain handling method: {}", method);

        match method {
            "enable" => self.enable(params).await,
            "disable" => self.disable(params).await,
            "getFullAXTree" => self.get_full_ax_tree(params).await,
            "getAXNodeForDOMNode" => self.get_ax_node_for_dom_node(params).await,
            "getAriaAttributes" => self.get_aria_attributes(params).await,
            "getContrastInfo" => self.get_contrast_info(params).await,
            "queryAXTree" => self.query_ax_tree(params).await,
            "getPartialAXTree" => self.get_partial_ax_tree(params).await,
            _ => {
                warn!("Unknown AccessibilityInspector method: {}", method);
                Err(CdpError::method_not_found(format!(
                    "AccessibilityInspector.{}",
                    method
                )))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_domain_name() {
        let inspector = AccessibilityInspector::new();
        assert_eq!(inspector.name(), "AccessibilityInspector");
    }

    #[tokio::test]
    async fn test_enable_disable() {
        let inspector = AccessibilityInspector::new();

        let result = inspector.enable(None).await;
        assert!(result.is_ok());

        let state = inspector.get_state().await;
        assert!(state.enabled);

        let result = inspector.disable(None).await;
        assert!(result.is_ok());

        let state = inspector.get_state().await;
        assert!(!state.enabled);
    }

    #[tokio::test]
    async fn test_get_full_ax_tree() {
        let inspector = AccessibilityInspector::new();

        let result = inspector.get_full_ax_tree(None).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        assert!(value["nodes"].is_array());

        let nodes = value["nodes"].as_array().unwrap();
        assert!(!nodes.is_empty());
    }

    #[tokio::test]
    async fn test_get_ax_node_for_dom_node() {
        let inspector = AccessibilityInspector::new();
        let params = json!({ "nodeId": 6 });

        let result = inspector.get_ax_node_for_dom_node(Some(params)).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        assert!(value["node"].is_object());
        assert!(value["node"]["role"].is_object());
    }

    #[tokio::test]
    async fn test_get_ax_node_for_dom_node_not_found() {
        let inspector = AccessibilityInspector::new();
        let params = json!({ "nodeId": 99999 });

        let result = inspector.get_ax_node_for_dom_node(Some(params)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_aria_attributes() {
        let inspector = AccessibilityInspector::new();
        let params = json!({ "nodeId": 6 });

        let result = inspector.get_aria_attributes(Some(params)).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        assert_eq!(value["nodeId"], 6);
    }

    #[tokio::test]
    async fn test_get_contrast_info() {
        let inspector = AccessibilityInspector::new();
        let params = json!({ "nodeId": 6 });

        let result = inspector.get_contrast_info(Some(params)).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        assert!(value["contrastRatio"].is_number());
        assert!(value["aaLarge"].is_boolean());
        assert!(value["aaNormal"].is_boolean());
    }

    #[tokio::test]
    async fn test_query_ax_tree() {
        let inspector = AccessibilityInspector::new();
        let params = json!({ "role": "heading" });

        let result = inspector.query_ax_tree(Some(params)).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        assert!(value["nodes"].is_array());
    }

    #[tokio::test]
    async fn test_get_partial_ax_tree() {
        let inspector = AccessibilityInspector::new();
        let params = json!({ "nodeId": 6, "depth": 2 });

        let result = inspector.get_partial_ax_tree(Some(params)).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        assert!(value["nodes"].is_array());
    }

    #[tokio::test]
    async fn test_unknown_method() {
        let inspector = AccessibilityInspector::new();
        let result = inspector.handle_method("unknownMethod", None).await;
        assert!(result.is_err());
    }
}
