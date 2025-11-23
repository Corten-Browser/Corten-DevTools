//! Render Inspector Bridge implementation
//!
//! Provides a bridge for render tree inspection via Chrome DevTools Protocol.
//! Implements FEAT-020: Render Inspector Bridge.

use async_trait::async_trait;
use cdp_types::domains::css::ComputedStyles;
use cdp_types::domains::dom::NodeId;
use cdp_types::CdpError;
use protocol_handler::DomainHandler;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tracing::{debug, warn};

use crate::mock_browser::MockBrowser;
use crate::types::{BoxModel, LayerInfo};

/// Parameters for getting box model
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBoxModelParams {
    /// Node ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<NodeId>,
    /// Backend node ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend_node_id: Option<u32>,
    /// Object ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_id: Option<String>,
}

/// Parameters for getting computed styles
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetComputedStyleParams {
    /// Node ID
    pub node_id: NodeId,
}

/// Parameters for getting matched styles
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetMatchedStylesParams {
    /// Node ID
    pub node_id: NodeId,
}

/// Parameters for getting inline styles
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetInlineStylesParams {
    /// Node ID
    pub node_id: NodeId,
}

/// CSS Rule match
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleMatch {
    /// CSS rule
    pub rule: CSSRule,
    /// Matching selector indices
    pub matching_selectors: Vec<u32>,
}

/// CSS Rule
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CSSRule {
    /// Selector list
    pub selector_list: SelectorList,
    /// Origin
    pub origin: String,
    /// Style declaration
    pub style: CSSStyle,
}

/// Selector list
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectorList {
    /// Individual selectors
    pub selectors: Vec<SelectorValue>,
    /// Combined text
    pub text: String,
}

/// Selector value
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectorValue {
    /// Selector text
    pub text: String,
}

/// CSS Style
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CSSStyle {
    /// CSS properties
    pub css_properties: Vec<CSSPropertyValue>,
    /// Shorthand entries
    pub shorthand_entries: Vec<ShorthandEntry>,
}

/// CSS Property value
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CSSPropertyValue {
    /// Property name
    pub name: String,
    /// Property value
    pub value: String,
}

/// Shorthand entry
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShorthandEntry {
    /// Name
    pub name: String,
    /// Value
    pub value: String,
}

/// Render Inspector Bridge
///
/// Provides a bridge for render tree inspection including box model,
/// computed styles, and layer tree representation.
pub struct RenderInspectorBridge {
    /// Mock browser for testing
    browser: Arc<MockBrowser>,
}

impl RenderInspectorBridge {
    /// Create a new Render Inspector Bridge
    ///
    /// # Example
    /// ```
    /// use inspector_bridges::RenderInspectorBridge;
    ///
    /// let bridge = RenderInspectorBridge::new();
    /// ```
    pub fn new() -> Self {
        Self {
            browser: Arc::new(MockBrowser::new()),
        }
    }

    /// Create with custom browser (for testing)
    pub fn with_browser(browser: Arc<MockBrowser>) -> Self {
        Self { browser }
    }

    /// Get box model for a node
    async fn get_box_model(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("RenderInspector.getBoxModel called");

        let params: GetBoxModelParams = serde_json::from_value(
            params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?,
        )
        .map_err(|e| CdpError::invalid_params(format!("Invalid parameters: {}", e)))?;

        let node_id = params
            .node_id
            .ok_or_else(|| CdpError::invalid_params("nodeId required"))?;

        let box_model = self.browser.get_box_model(node_id).ok_or_else(|| {
            CdpError::server_error(-32000, format!("Node {} not found", node_id.0))
        })?;

        let response = serde_json::json!({
            "model": box_model
        });

        Ok(response)
    }

    /// Get computed style for a node
    async fn get_computed_style_for_node(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("RenderInspector.getComputedStyleForNode called");

        let params: GetComputedStyleParams = serde_json::from_value(
            params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?,
        )
        .map_err(|e| CdpError::invalid_params(format!("Invalid parameters: {}", e)))?;

        let styles = self
            .browser
            .get_computed_styles(params.node_id)
            .ok_or_else(|| {
                CdpError::server_error(-32000, format!("Node {} not found", params.node_id.0))
            })?;

        let response = serde_json::json!({
            "computedStyle": styles.properties
        });

        Ok(response)
    }

    /// Get matched styles for a node
    async fn get_matched_styles_for_node(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("RenderInspector.getMatchedStylesForNode called");

        let params: GetMatchedStylesParams = serde_json::from_value(
            params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?,
        )
        .map_err(|e| CdpError::invalid_params(format!("Invalid parameters: {}", e)))?;

        if !self.browser.node_exists(params.node_id) {
            return Err(CdpError::server_error(
                -32000,
                format!("Node {} not found", params.node_id.0),
            ));
        }

        // Return mock matched styles
        let response = serde_json::json!({
            "matchedCSSRules": [
                {
                    "rule": {
                        "selectorList": {
                            "selectors": [{ "text": "*" }],
                            "text": "*"
                        },
                        "origin": "user-agent",
                        "style": {
                            "cssProperties": [
                                { "name": "display", "value": "block" },
                                { "name": "margin", "value": "0" }
                            ],
                            "shorthandEntries": []
                        }
                    },
                    "matchingSelectors": [0]
                }
            ],
            "inherited": [],
            "cssKeyframesRules": []
        });

        Ok(response)
    }

    /// Get inline styles for a node
    async fn get_inline_styles_for_node(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("RenderInspector.getInlineStylesForNode called");

        let params: GetInlineStylesParams = serde_json::from_value(
            params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?,
        )
        .map_err(|e| CdpError::invalid_params(format!("Invalid parameters: {}", e)))?;

        if !self.browser.node_exists(params.node_id) {
            return Err(CdpError::server_error(
                -32000,
                format!("Node {} not found", params.node_id.0),
            ));
        }

        // Return mock inline styles (empty since our test nodes don't have inline styles)
        let response = serde_json::json!({
            "inlineStyle": {
                "cssProperties": [],
                "shorthandEntries": []
            },
            "attributesStyle": null
        });

        Ok(response)
    }

    /// Get layer tree
    async fn get_layer_tree(&self, _params: Option<Value>) -> Result<Value, CdpError> {
        debug!("RenderInspector.getLayerTree called");

        let layers = self.browser.get_layer_tree();

        let response = serde_json::json!({
            "layers": layers
        });

        Ok(response)
    }

    /// Compose layers snapshot
    async fn compose_layers(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("RenderInspector.composeLayers called");

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct ComposeLayersParams {
            layer_id: String,
        }

        let params: ComposeLayersParams = serde_json::from_value(
            params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?,
        )
        .map_err(|e| CdpError::invalid_params(format!("Invalid parameters: {}", e)))?;

        let layers = self.browser.get_layer_tree();
        let layer_exists = layers.iter().any(|l| l.layer_id == params.layer_id);

        if !layer_exists {
            return Err(CdpError::server_error(
                -32000,
                format!("Layer {} not found", params.layer_id),
            ));
        }

        // Return mock snapshot ID
        let response = serde_json::json!({
            "snapshotId": format!("snapshot-{}", params.layer_id)
        });

        Ok(response)
    }

    /// Get layer by ID
    async fn get_layer(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("RenderInspector.getLayer called");

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct GetLayerParams {
            layer_id: String,
        }

        let params: GetLayerParams = serde_json::from_value(
            params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?,
        )
        .map_err(|e| CdpError::invalid_params(format!("Invalid parameters: {}", e)))?;

        let layers = self.browser.get_layer_tree();
        let layer = layers
            .into_iter()
            .find(|l| l.layer_id == params.layer_id)
            .ok_or_else(|| {
                CdpError::server_error(-32000, format!("Layer {} not found", params.layer_id))
            })?;

        let response = serde_json::json!({
            "layer": layer
        });

        Ok(response)
    }

    /// Enable layer tree tracking
    async fn enable_layer_tree(&self, _params: Option<Value>) -> Result<Value, CdpError> {
        debug!("RenderInspector.enableLayerTree called");
        Ok(serde_json::json!({}))
    }

    /// Disable layer tree tracking
    async fn disable_layer_tree(&self, _params: Option<Value>) -> Result<Value, CdpError> {
        debug!("RenderInspector.disableLayerTree called");
        Ok(serde_json::json!({}))
    }

    /// Get underlying browser (for testing)
    pub fn browser(&self) -> &MockBrowser {
        &self.browser
    }

    /// Get box model directly (for external use)
    pub fn get_element_box_model(&self, node_id: NodeId) -> Option<BoxModel> {
        self.browser.get_box_model(node_id)
    }

    /// Get computed styles directly (for external use)
    pub fn get_element_computed_styles(&self, node_id: NodeId) -> Option<ComputedStyles> {
        self.browser.get_computed_styles(node_id)
    }

    /// Get layer tree directly (for external use)
    pub fn get_layer_tree_snapshot(&self) -> Vec<LayerInfo> {
        self.browser.get_layer_tree()
    }
}

impl Default for RenderInspectorBridge {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DomainHandler for RenderInspectorBridge {
    fn name(&self) -> &str {
        "RenderInspector"
    }

    async fn handle_method(&self, method: &str, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("RenderInspector domain handling method: {}", method);

        match method {
            "getBoxModel" => self.get_box_model(params).await,
            "getComputedStyleForNode" => self.get_computed_style_for_node(params).await,
            "getMatchedStylesForNode" => self.get_matched_styles_for_node(params).await,
            "getInlineStylesForNode" => self.get_inline_styles_for_node(params).await,
            "getLayerTree" => self.get_layer_tree(params).await,
            "composeLayers" => self.compose_layers(params).await,
            "getLayer" => self.get_layer(params).await,
            "enableLayerTree" => self.enable_layer_tree(params).await,
            "disableLayerTree" => self.disable_layer_tree(params).await,
            _ => {
                warn!("Unknown RenderInspector method: {}", method);
                Err(CdpError::method_not_found(format!(
                    "RenderInspector.{}",
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
        let bridge = RenderInspectorBridge::new();
        assert_eq!(bridge.name(), "RenderInspector");
    }

    #[tokio::test]
    async fn test_get_box_model() {
        let bridge = RenderInspectorBridge::new();
        let params = json!({ "nodeId": 6 });

        let result = bridge.get_box_model(Some(params)).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        assert!(value["model"].is_object());
        assert!(value["model"]["content"].is_array());
        assert!(value["model"]["width"].is_number());
        assert!(value["model"]["height"].is_number());
    }

    #[tokio::test]
    async fn test_get_box_model_invalid_node() {
        let bridge = RenderInspectorBridge::new();
        let params = json!({ "nodeId": 99999 });

        let result = bridge.get_box_model(Some(params)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_computed_style_for_node() {
        let bridge = RenderInspectorBridge::new();
        let params = json!({ "nodeId": 6 });

        let result = bridge.get_computed_style_for_node(Some(params)).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        assert!(value["computedStyle"].is_array());

        let styles = value["computedStyle"].as_array().unwrap();
        assert!(!styles.is_empty());

        // Check for expected properties
        let has_display = styles.iter().any(|p| p["name"] == "display");
        assert!(has_display);
    }

    #[tokio::test]
    async fn test_get_computed_style_invalid_node() {
        let bridge = RenderInspectorBridge::new();
        let params = json!({ "nodeId": 99999 });

        let result = bridge.get_computed_style_for_node(Some(params)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_matched_styles_for_node() {
        let bridge = RenderInspectorBridge::new();
        let params = json!({ "nodeId": 6 });

        let result = bridge.get_matched_styles_for_node(Some(params)).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        assert!(value["matchedCSSRules"].is_array());
    }

    #[tokio::test]
    async fn test_get_inline_styles_for_node() {
        let bridge = RenderInspectorBridge::new();
        let params = json!({ "nodeId": 6 });

        let result = bridge.get_inline_styles_for_node(Some(params)).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        assert!(value["inlineStyle"].is_object());
    }

    #[tokio::test]
    async fn test_get_layer_tree() {
        let bridge = RenderInspectorBridge::new();

        let result = bridge.get_layer_tree(None).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        assert!(value["layers"].is_array());

        let layers = value["layers"].as_array().unwrap();
        assert!(!layers.is_empty());
        assert_eq!(layers[0]["layerId"], "root-layer");
    }

    #[tokio::test]
    async fn test_get_layer() {
        let bridge = RenderInspectorBridge::new();
        let params = json!({ "layerId": "root-layer" });

        let result = bridge.get_layer(Some(params)).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        assert!(value["layer"].is_object());
        assert_eq!(value["layer"]["layerId"], "root-layer");
    }

    #[tokio::test]
    async fn test_get_layer_not_found() {
        let bridge = RenderInspectorBridge::new();
        let params = json!({ "layerId": "nonexistent" });

        let result = bridge.get_layer(Some(params)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_compose_layers() {
        let bridge = RenderInspectorBridge::new();
        let params = json!({ "layerId": "root-layer" });

        let result = bridge.compose_layers(Some(params)).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        assert!(value["snapshotId"].is_string());
    }

    #[tokio::test]
    async fn test_enable_layer_tree() {
        let bridge = RenderInspectorBridge::new();
        let result = bridge.enable_layer_tree(None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_disable_layer_tree() {
        let bridge = RenderInspectorBridge::new();
        let result = bridge.disable_layer_tree(None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_unknown_method() {
        let bridge = RenderInspectorBridge::new();
        let result = bridge.handle_method("unknownMethod", None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_direct_api_get_box_model() {
        let bridge = RenderInspectorBridge::new();
        let box_model = bridge.get_element_box_model(NodeId(6));
        assert!(box_model.is_some());

        let bm = box_model.unwrap();
        assert!(bm.width > 0.0);
        assert!(bm.height > 0.0);
    }

    #[tokio::test]
    async fn test_direct_api_get_computed_styles() {
        let bridge = RenderInspectorBridge::new();
        let styles = bridge.get_element_computed_styles(NodeId(6));
        assert!(styles.is_some());
        assert!(!styles.unwrap().properties.is_empty());
    }

    #[tokio::test]
    async fn test_direct_api_get_layer_tree() {
        let bridge = RenderInspectorBridge::new();
        let layers = bridge.get_layer_tree_snapshot();
        assert!(!layers.is_empty());
    }
}
