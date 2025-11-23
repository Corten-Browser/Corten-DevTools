//! Elements Inspector implementation
//!
//! Provides DOM element inspection and editing via Chrome DevTools Protocol.
//! Implements FEAT-022: Elements Inspector.
//!
//! Features:
//! - DOM tree view
//! - Attribute editing
//! - Styles panel integration

use async_trait::async_trait;
use cdp_types::domains::dom::NodeId;
use cdp_types::CdpError;
use protocol_handler::DomainHandler;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn};

use crate::mock_browser::MockBrowser;
use crate::types::HighlightConfig;

/// Element info for the elements panel
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ElementInfo {
    /// Node ID
    pub node_id: NodeId,
    /// Tag name
    pub tag_name: String,
    /// Node type
    pub node_type: u32,
    /// Attributes as key-value pairs
    pub attributes: HashMap<String, String>,
    /// Inner HTML (truncated for display)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inner_html: Option<String>,
    /// Outer HTML
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outer_html: Option<String>,
    /// Child count
    pub child_count: u32,
    /// Whether the node has shadow roots
    #[serde(default)]
    pub has_shadow_roots: bool,
    /// Whether the node is content editable
    #[serde(default)]
    pub is_content_editable: bool,
}

/// Style modification entry
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StyleModification {
    /// Property name
    pub property_name: String,
    /// Property value
    pub property_value: String,
    /// Priority (e.g., "important")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
}

/// Parameters for getOuterHTML
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetOuterHTMLParams {
    /// Node ID
    pub node_id: NodeId,
}

/// Parameters for setOuterHTML
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetOuterHTMLParams {
    /// Node ID
    pub node_id: NodeId,
    /// New outer HTML
    pub outer_html: String,
}

/// Parameters for setNodeValue
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetNodeValueParams {
    /// Node ID
    pub node_id: NodeId,
    /// New value
    pub value: String,
}

/// Parameters for setNodeName
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetNodeNameParams {
    /// Node ID
    pub node_id: NodeId,
    /// New name
    pub name: String,
}

/// Parameters for removeNode
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveNodeParams {
    /// Node ID
    pub node_id: NodeId,
}

/// Parameters for copyTo
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyToParams {
    /// Node ID to copy
    pub node_id: NodeId,
    /// Target parent node ID
    pub target_node_id: NodeId,
    /// Insert before this node (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insert_before_node_id: Option<NodeId>,
}

/// Parameters for moveTo
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveToParams {
    /// Node ID to move
    pub node_id: NodeId,
    /// Target parent node ID
    pub target_node_id: NodeId,
    /// Insert before this node (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insert_before_node_id: Option<NodeId>,
}

/// Parameters for setStyleText
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetStyleTextParams {
    /// Node ID
    pub node_id: NodeId,
    /// Style text
    pub style_text: String,
}

/// Element state tracking
#[derive(Debug, Clone, Default)]
pub struct ElementState {
    /// Currently selected element
    pub selected_element: Option<NodeId>,
    /// Currently hovered element
    pub hovered_element: Option<NodeId>,
    /// Inspect mode enabled
    pub inspect_mode_enabled: bool,
    /// Style modifications applied
    pub style_modifications: HashMap<NodeId, Vec<StyleModification>>,
}

/// Elements Inspector
///
/// Provides comprehensive DOM element inspection and editing capabilities
/// including tree view, attribute editing, and styles panel integration.
pub struct ElementsInspector {
    /// Mock browser for testing
    browser: Arc<MockBrowser>,
    /// Element state
    element_state: Arc<RwLock<ElementState>>,
}

impl ElementsInspector {
    /// Create a new Elements Inspector
    pub fn new() -> Self {
        Self {
            browser: Arc::new(MockBrowser::new()),
            element_state: Arc::new(RwLock::new(ElementState::default())),
        }
    }

    /// Create with custom browser (for testing)
    pub fn with_browser(browser: Arc<MockBrowser>) -> Self {
        Self {
            browser,
            element_state: Arc::new(RwLock::new(ElementState::default())),
        }
    }

    /// Get element info for a node
    async fn get_element_info(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("ElementsInspector.getElementInfo called");

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Params {
            node_id: NodeId,
        }

        let params: Params = serde_json::from_value(
            params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?,
        )
        .map_err(|e| CdpError::invalid_params(format!("Invalid parameters: {}", e)))?;

        let node = self.browser.get_node(params.node_id).ok_or_else(|| {
            CdpError::server_error(-32000, format!("Node {} not found", params.node_id.0))
        })?;

        // Parse attributes into HashMap
        let mut attributes = HashMap::new();
        if let Some(attrs) = &node.attributes {
            for i in (0..attrs.len()).step_by(2) {
                if let (Some(name), Some(value)) = (attrs.get(i), attrs.get(i + 1)) {
                    attributes.insert(name.clone(), value.clone());
                }
            }
        }

        let element_info = ElementInfo {
            node_id: params.node_id,
            tag_name: node.node_name.clone(),
            node_type: node.node_type as u32,
            attributes,
            inner_html: None,
            outer_html: None,
            child_count: node.child_node_count.unwrap_or(0),
            has_shadow_roots: false,
            is_content_editable: false,
        };

        serde_json::to_value(element_info)
            .map_err(|e| CdpError::internal_error(format!("Serialization error: {}", e)))
    }

    /// Get outer HTML for a node
    async fn get_outer_html(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("ElementsInspector.getOuterHTML called");

        let params: GetOuterHTMLParams = serde_json::from_value(
            params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?,
        )
        .map_err(|e| CdpError::invalid_params(format!("Invalid parameters: {}", e)))?;

        let node = self.browser.get_node(params.node_id).ok_or_else(|| {
            CdpError::server_error(-32000, format!("Node {} not found", params.node_id.0))
        })?;

        // Generate mock outer HTML based on node
        let outer_html = format!("<{0}></{0}>", node.node_name.to_lowercase());

        Ok(serde_json::json!({
            "outerHTML": outer_html
        }))
    }

    /// Set outer HTML for a node
    async fn set_outer_html(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("ElementsInspector.setOuterHTML called");

        let params: SetOuterHTMLParams = serde_json::from_value(
            params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?,
        )
        .map_err(|e| CdpError::invalid_params(format!("Invalid parameters: {}", e)))?;

        if !self.browser.node_exists(params.node_id) {
            return Err(CdpError::server_error(
                -32000,
                format!("Node {} not found", params.node_id.0),
            ));
        }

        // In a real implementation, this would parse and replace the HTML
        // For mock, we just validate and return success
        if params.outer_html.is_empty() {
            return Err(CdpError::invalid_params("outerHTML cannot be empty"));
        }

        Ok(serde_json::json!({}))
    }

    /// Set node value (for text nodes)
    async fn set_node_value(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("ElementsInspector.setNodeValue called");

        let params: SetNodeValueParams = serde_json::from_value(
            params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?,
        )
        .map_err(|e| CdpError::invalid_params(format!("Invalid parameters: {}", e)))?;

        if !self.browser.node_exists(params.node_id) {
            return Err(CdpError::server_error(
                -32000,
                format!("Node {} not found", params.node_id.0),
            ));
        }

        Ok(serde_json::json!({}))
    }

    /// Set node name (tag name)
    async fn set_node_name(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("ElementsInspector.setNodeName called");

        let params: SetNodeNameParams = serde_json::from_value(
            params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?,
        )
        .map_err(|e| CdpError::invalid_params(format!("Invalid parameters: {}", e)))?;

        if !self.browser.node_exists(params.node_id) {
            return Err(CdpError::server_error(
                -32000,
                format!("Node {} not found", params.node_id.0),
            ));
        }

        // Return the new node ID (in real impl, might be different)
        Ok(serde_json::json!({
            "nodeId": params.node_id
        }))
    }

    /// Remove a node from the DOM
    async fn remove_node(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("ElementsInspector.removeNode called");

        let params: RemoveNodeParams = serde_json::from_value(
            params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?,
        )
        .map_err(|e| CdpError::invalid_params(format!("Invalid parameters: {}", e)))?;

        if !self.browser.node_exists(params.node_id) {
            return Err(CdpError::server_error(
                -32000,
                format!("Node {} not found", params.node_id.0),
            ));
        }

        // In a real implementation, this would remove the node
        Ok(serde_json::json!({}))
    }

    /// Copy node to another location
    async fn copy_to(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("ElementsInspector.copyTo called");

        let params: CopyToParams = serde_json::from_value(
            params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?,
        )
        .map_err(|e| CdpError::invalid_params(format!("Invalid parameters: {}", e)))?;

        if !self.browser.node_exists(params.node_id) {
            return Err(CdpError::server_error(
                -32000,
                format!("Source node {} not found", params.node_id.0),
            ));
        }

        if !self.browser.node_exists(params.target_node_id) {
            return Err(CdpError::server_error(
                -32000,
                format!("Target node {} not found", params.target_node_id.0),
            ));
        }

        // Return new node ID for the copy
        Ok(serde_json::json!({
            "nodeId": params.node_id.0 + 1000 // Mock new node ID
        }))
    }

    /// Move node to another location
    async fn move_to(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("ElementsInspector.moveTo called");

        let params: MoveToParams = serde_json::from_value(
            params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?,
        )
        .map_err(|e| CdpError::invalid_params(format!("Invalid parameters: {}", e)))?;

        if !self.browser.node_exists(params.node_id) {
            return Err(CdpError::server_error(
                -32000,
                format!("Source node {} not found", params.node_id.0),
            ));
        }

        if !self.browser.node_exists(params.target_node_id) {
            return Err(CdpError::server_error(
                -32000,
                format!("Target node {} not found", params.target_node_id.0),
            ));
        }

        Ok(serde_json::json!({
            "nodeId": params.node_id
        }))
    }

    /// Set inline styles via style text
    async fn set_style_text(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("ElementsInspector.setStyleText called");

        let params: SetStyleTextParams = serde_json::from_value(
            params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?,
        )
        .map_err(|e| CdpError::invalid_params(format!("Invalid parameters: {}", e)))?;

        if !self.browser.node_exists(params.node_id) {
            return Err(CdpError::server_error(
                -32000,
                format!("Node {} not found", params.node_id.0),
            ));
        }

        // Store the style modification
        let mut state = self.element_state.write().await;
        let modification = StyleModification {
            property_name: "style".to_string(),
            property_value: params.style_text.clone(),
            priority: None,
        };
        state
            .style_modifications
            .entry(params.node_id)
            .or_default()
            .push(modification);

        Ok(serde_json::json!({
            "style": {
                "cssProperties": [],
                "shorthandEntries": [],
                "cssText": params.style_text
            }
        }))
    }

    /// Enable inspect mode
    async fn enable_inspect_mode(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("ElementsInspector.enableInspectMode called");

        let highlight_config: HighlightConfig = params
            .map(|p| serde_json::from_value(p).ok())
            .flatten()
            .unwrap_or_default();

        let mut state = self.element_state.write().await;
        state.inspect_mode_enabled = true;

        Ok(serde_json::json!({
            "enabled": true,
            "highlightConfig": highlight_config
        }))
    }

    /// Disable inspect mode
    async fn disable_inspect_mode(&self, _params: Option<Value>) -> Result<Value, CdpError> {
        debug!("ElementsInspector.disableInspectMode called");

        let mut state = self.element_state.write().await;
        state.inspect_mode_enabled = false;
        state.hovered_element = None;

        Ok(serde_json::json!({
            "enabled": false
        }))
    }

    /// Select element
    async fn select_element(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("ElementsInspector.selectElement called");

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

        let mut state = self.element_state.write().await;
        state.selected_element = Some(params.node_id);

        Ok(serde_json::json!({
            "nodeId": params.node_id
        }))
    }

    /// Get element state
    pub async fn get_element_state(&self) -> ElementState {
        self.element_state.read().await.clone()
    }

    /// Get browser (for testing)
    pub fn browser(&self) -> &MockBrowser {
        &self.browser
    }
}

impl Default for ElementsInspector {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DomainHandler for ElementsInspector {
    fn name(&self) -> &str {
        "ElementsInspector"
    }

    async fn handle_method(&self, method: &str, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("ElementsInspector domain handling method: {}", method);

        match method {
            "getElementInfo" => self.get_element_info(params).await,
            "getOuterHTML" => self.get_outer_html(params).await,
            "setOuterHTML" => self.set_outer_html(params).await,
            "setNodeValue" => self.set_node_value(params).await,
            "setNodeName" => self.set_node_name(params).await,
            "removeNode" => self.remove_node(params).await,
            "copyTo" => self.copy_to(params).await,
            "moveTo" => self.move_to(params).await,
            "setStyleText" => self.set_style_text(params).await,
            "enableInspectMode" => self.enable_inspect_mode(params).await,
            "disableInspectMode" => self.disable_inspect_mode(params).await,
            "selectElement" => self.select_element(params).await,
            _ => {
                warn!("Unknown ElementsInspector method: {}", method);
                Err(CdpError::method_not_found(format!(
                    "ElementsInspector.{}",
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
        let inspector = ElementsInspector::new();
        assert_eq!(inspector.name(), "ElementsInspector");
    }

    #[tokio::test]
    async fn test_get_element_info() {
        let inspector = ElementsInspector::new();
        let params = json!({ "nodeId": 6 });

        let result = inspector.get_element_info(Some(params)).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        assert_eq!(value["nodeId"], 6);
        assert!(value["tagName"].is_string());
    }

    #[tokio::test]
    async fn test_get_element_info_not_found() {
        let inspector = ElementsInspector::new();
        let params = json!({ "nodeId": 99999 });

        let result = inspector.get_element_info(Some(params)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_outer_html() {
        let inspector = ElementsInspector::new();
        let params = json!({ "nodeId": 6 });

        let result = inspector.get_outer_html(Some(params)).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        assert!(value["outerHTML"].is_string());
    }

    #[tokio::test]
    async fn test_set_outer_html() {
        let inspector = ElementsInspector::new();
        let params = json!({
            "nodeId": 6,
            "outerHtml": "<div>New content</div>"
        });

        let result = inspector.set_outer_html(Some(params)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_set_outer_html_empty() {
        let inspector = ElementsInspector::new();
        let params = json!({
            "nodeId": 6,
            "outerHtml": ""
        });

        let result = inspector.set_outer_html(Some(params)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_set_node_value() {
        let inspector = ElementsInspector::new();
        let params = json!({
            "nodeId": 6,
            "value": "New text content"
        });

        let result = inspector.set_node_value(Some(params)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_remove_node() {
        let inspector = ElementsInspector::new();
        let params = json!({ "nodeId": 6 });

        let result = inspector.remove_node(Some(params)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_copy_to() {
        let inspector = ElementsInspector::new();
        let params = json!({
            "nodeId": 6,
            "targetNodeId": 1
        });

        let result = inspector.copy_to(Some(params)).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        assert!(value["nodeId"].is_number());
    }

    #[tokio::test]
    async fn test_move_to() {
        let inspector = ElementsInspector::new();
        let params = json!({
            "nodeId": 6,
            "targetNodeId": 1
        });

        let result = inspector.move_to(Some(params)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_set_style_text() {
        let inspector = ElementsInspector::new();
        let params = json!({
            "nodeId": 6,
            "styleText": "color: red; font-size: 14px;"
        });

        let result = inspector.set_style_text(Some(params)).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        assert!(value["style"].is_object());
    }

    #[tokio::test]
    async fn test_enable_inspect_mode() {
        let inspector = ElementsInspector::new();
        let params = json!({
            "showInfo": true,
            "showRulers": true
        });

        let result = inspector.enable_inspect_mode(Some(params)).await;
        assert!(result.is_ok());

        let state = inspector.get_element_state().await;
        assert!(state.inspect_mode_enabled);
    }

    #[tokio::test]
    async fn test_disable_inspect_mode() {
        let inspector = ElementsInspector::new();

        // Enable first
        inspector.enable_inspect_mode(None).await.unwrap();

        // Then disable
        let result = inspector.disable_inspect_mode(None).await;
        assert!(result.is_ok());

        let state = inspector.get_element_state().await;
        assert!(!state.inspect_mode_enabled);
    }

    #[tokio::test]
    async fn test_select_element() {
        let inspector = ElementsInspector::new();
        let params = json!({ "nodeId": 6 });

        let result = inspector.select_element(Some(params)).await;
        assert!(result.is_ok());

        let state = inspector.get_element_state().await;
        assert_eq!(state.selected_element, Some(NodeId(6)));
    }

    #[tokio::test]
    async fn test_unknown_method() {
        let inspector = ElementsInspector::new();
        let result = inspector.handle_method("unknownMethod", None).await;
        assert!(result.is_err());
    }
}
