//! DOM Inspector Bridge implementation
//!
//! Provides a bridge between the Chrome DevTools Protocol DOM domain and the browser's DOM.
//! Implements FEAT-017: DOM Inspector Bridge.

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
use crate::types::{HighlightConfig, MutationRecord, SearchResult, SelectionState};

/// Parameters for DOM.getDocument
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetDocumentParams {
    /// Maximum depth of the tree to return
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth: Option<i32>,
    /// Whether to pierce through iframes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pierce: Option<bool>,
}

/// Parameters for DOM.requestChildNodes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestChildNodesParams {
    /// Node ID to request children for
    pub node_id: NodeId,
    /// Maximum depth
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth: Option<i32>,
    /// Whether to pierce through iframes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pierce: Option<bool>,
}

/// Parameters for DOM.querySelector
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuerySelectorParams {
    /// Node ID to query on
    pub node_id: NodeId,
    /// CSS selector
    pub selector: String,
}

/// Parameters for DOM.querySelectorAll
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuerySelectorAllParams {
    /// Node ID to query on
    pub node_id: NodeId,
    /// CSS selector
    pub selector: String,
}

/// Parameters for DOM.setAttributeValue
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetAttributeValueParams {
    /// Node ID
    pub node_id: NodeId,
    /// Attribute name
    pub name: String,
    /// Attribute value
    pub value: String,
}

/// Parameters for DOM.removeAttribute
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveAttributeParams {
    /// Node ID
    pub node_id: NodeId,
    /// Attribute name
    pub name: String,
}

/// Parameters for DOM.highlightNode
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HighlightNodeParams {
    /// Highlight configuration
    pub highlight_config: HighlightConfig,
    /// Node ID to highlight
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<NodeId>,
    /// Object ID to highlight (alternative to node_id)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_id: Option<String>,
}

/// Parameters for DOM.performSearch
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformSearchParams {
    /// Search query
    pub query: String,
    /// Include user agent shadow DOM
    #[serde(default)]
    pub include_user_agent_shadow_dom: bool,
}

/// Parameters for DOM.getSearchResults
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSearchResultsParams {
    /// Search ID
    pub search_id: String,
    /// Start index
    pub from_index: u32,
    /// End index
    pub to_index: u32,
}

/// Parameters for DOM.discardSearchResults
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscardSearchResultsParams {
    /// Search ID
    pub search_id: String,
}

/// DOM Inspector Bridge
///
/// Provides a bridge between CDP DOM domain and the browser's DOM implementation.
/// Supports node traversal, selection, highlighting, mutation tracking, and search.
pub struct DomInspectorBridge {
    /// Mock browser for testing
    browser: Arc<MockBrowser>,
    /// Selection state
    selection_state: Arc<RwLock<SelectionState>>,
}

impl DomInspectorBridge {
    /// Create a new DOM Inspector Bridge
    ///
    /// # Example
    /// ```
    /// use inspector_bridges::DomInspectorBridge;
    ///
    /// let bridge = DomInspectorBridge::new();
    /// ```
    pub fn new() -> Self {
        Self {
            browser: Arc::new(MockBrowser::new()),
            selection_state: Arc::new(RwLock::new(SelectionState::default())),
        }
    }

    /// Create with custom browser (for testing)
    pub fn with_browser(browser: Arc<MockBrowser>) -> Self {
        Self {
            browser,
            selection_state: Arc::new(RwLock::new(SelectionState::default())),
        }
    }

    /// Get the document root
    async fn get_document(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("DOMInspector.getDocument called");

        let depth = params
            .and_then(|p| serde_json::from_value::<GetDocumentParams>(p).ok())
            .and_then(|p| p.depth)
            .unwrap_or(1) as u32;

        let root = self.browser.get_node_with_children(NodeId(1), depth);

        match root {
            Some(node) => {
                let response = serde_json::json!({ "root": node });
                Ok(response)
            }
            None => Err(CdpError::internal_error("Failed to get document")),
        }
    }

    /// Request child nodes for a parent
    async fn request_child_nodes(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("DOMInspector.requestChildNodes called");

        let params: RequestChildNodesParams = serde_json::from_value(
            params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?,
        )
        .map_err(|e| CdpError::invalid_params(format!("Invalid parameters: {}", e)))?;

        let depth = params.depth.unwrap_or(1) as u32;

        if let Some(node) = self.browser.get_node_with_children(params.node_id, depth) {
            // In CDP, this triggers a setChildNodes event
            // For now, we return the children directly
            let response = serde_json::json!({
                "parentId": params.node_id,
                "nodes": node.children.unwrap_or_default()
            });
            Ok(response)
        } else {
            Err(CdpError::server_error(
                -32000,
                format!("Node {} not found", params.node_id.0),
            ))
        }
    }

    /// Query for a single element
    async fn query_selector(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("DOMInspector.querySelector called");

        let params: QuerySelectorParams = serde_json::from_value(
            params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?,
        )
        .map_err(|e| CdpError::invalid_params(format!("Invalid parameters: {}", e)))?;

        if !self.browser.node_exists(params.node_id) {
            return Err(CdpError::server_error(
                -32000,
                format!("Node {} not found", params.node_id.0),
            ));
        }

        let result = self
            .browser
            .query_selector(params.node_id, &params.selector);

        let response = serde_json::json!({
            "nodeId": result
        });

        Ok(response)
    }

    /// Query for all matching elements
    async fn query_selector_all(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("DOMInspector.querySelectorAll called");

        let params: QuerySelectorAllParams = serde_json::from_value(
            params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?,
        )
        .map_err(|e| CdpError::invalid_params(format!("Invalid parameters: {}", e)))?;

        if !self.browser.node_exists(params.node_id) {
            return Err(CdpError::server_error(
                -32000,
                format!("Node {} not found", params.node_id.0),
            ));
        }

        let results = self
            .browser
            .query_selector_all(params.node_id, &params.selector);

        let response = serde_json::json!({
            "nodeIds": results
        });

        Ok(response)
    }

    /// Set attribute value on a node
    async fn set_attribute_value(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("DOMInspector.setAttributeValue called");

        let params: SetAttributeValueParams = serde_json::from_value(
            params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?,
        )
        .map_err(|e| CdpError::invalid_params(format!("Invalid parameters: {}", e)))?;

        self.browser
            .set_attribute(params.node_id, &params.name, &params.value)
            .map_err(|e| CdpError::server_error(-32000, e))?;

        Ok(serde_json::json!({}))
    }

    /// Remove attribute from a node
    async fn remove_attribute(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("DOMInspector.removeAttribute called");

        let params: RemoveAttributeParams = serde_json::from_value(
            params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?,
        )
        .map_err(|e| CdpError::invalid_params(format!("Invalid parameters: {}", e)))?;

        self.browser
            .remove_attribute(params.node_id, &params.name)
            .map_err(|e| CdpError::server_error(-32000, e))?;

        Ok(serde_json::json!({}))
    }

    /// Highlight a node
    async fn highlight_node(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("DOMInspector.highlightNode called");

        let params: HighlightNodeParams = serde_json::from_value(
            params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?,
        )
        .map_err(|e| CdpError::invalid_params(format!("Invalid parameters: {}", e)))?;

        let node_id = params.node_id;

        // Validate node exists if provided
        if let Some(id) = node_id {
            if !self.browser.node_exists(id) {
                return Err(CdpError::server_error(
                    -32000,
                    format!("Node {} not found", id.0),
                ));
            }
        }

        // Update selection state
        {
            let mut state = self.selection_state.write().await;
            state.highlighted_node = node_id;
            state.highlight_config = params.highlight_config;
        }

        Ok(serde_json::json!({}))
    }

    /// Hide highlight
    async fn hide_highlight(&self, _params: Option<Value>) -> Result<Value, CdpError> {
        debug!("DOMInspector.hideHighlight called");

        {
            let mut state = self.selection_state.write().await;
            state.highlighted_node = None;
        }

        Ok(serde_json::json!({}))
    }

    /// Perform search in DOM
    async fn perform_search(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("DOMInspector.performSearch called");

        let params: PerformSearchParams = serde_json::from_value(
            params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?,
        )
        .map_err(|e| CdpError::invalid_params(format!("Invalid parameters: {}", e)))?;

        let (search_id, result_count) = self
            .browser
            .perform_search(&params.query, params.include_user_agent_shadow_dom);

        let response = SearchResult {
            search_id,
            result_count,
        };

        serde_json::to_value(response)
            .map_err(|e| CdpError::internal_error(format!("Serialization error: {}", e)))
    }

    /// Get search results
    async fn get_search_results(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("DOMInspector.getSearchResults called");

        let params: GetSearchResultsParams = serde_json::from_value(
            params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?,
        )
        .map_err(|e| CdpError::invalid_params(format!("Invalid parameters: {}", e)))?;

        let node_ids =
            self.browser
                .get_search_results(&params.search_id, params.from_index, params.to_index);

        let response = serde_json::json!({
            "nodeIds": node_ids
        });

        Ok(response)
    }

    /// Discard search results
    async fn discard_search_results(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("DOMInspector.discardSearchResults called");

        let params: DiscardSearchResultsParams = serde_json::from_value(
            params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?,
        )
        .map_err(|e| CdpError::invalid_params(format!("Invalid parameters: {}", e)))?;

        self.browser.discard_search_results(&params.search_id);

        Ok(serde_json::json!({}))
    }

    /// Get a specific node
    async fn describe_node(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("DOMInspector.describeNode called");

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct DescribeNodeParams {
            node_id: Option<NodeId>,
            depth: Option<i32>,
        }

        let params: DescribeNodeParams = serde_json::from_value(
            params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?,
        )
        .map_err(|e| CdpError::invalid_params(format!("Invalid parameters: {}", e)))?;

        let node_id = params
            .node_id
            .ok_or_else(|| CdpError::invalid_params("nodeId required"))?;

        let depth = params.depth.unwrap_or(0) as u32;

        let node = self
            .browser
            .get_node_with_children(node_id, depth)
            .ok_or_else(|| {
                CdpError::server_error(-32000, format!("Node {} not found", node_id.0))
            })?;

        let response = serde_json::json!({
            "node": node
        });

        Ok(response)
    }

    /// Subscribe to DOM mutations
    pub fn subscribe_mutations(&self) -> tokio::sync::broadcast::Receiver<MutationRecord> {
        self.browser.subscribe_mutations()
    }

    /// Get current selection state
    pub async fn get_selection_state(&self) -> SelectionState {
        self.selection_state.read().await.clone()
    }

    /// Get underlying browser (for testing)
    pub fn browser(&self) -> &MockBrowser {
        &self.browser
    }
}

impl Default for DomInspectorBridge {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DomainHandler for DomInspectorBridge {
    fn name(&self) -> &str {
        "DOMInspector"
    }

    async fn handle_method(&self, method: &str, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("DOMInspector domain handling method: {}", method);

        match method {
            "getDocument" => self.get_document(params).await,
            "requestChildNodes" => self.request_child_nodes(params).await,
            "querySelector" => self.query_selector(params).await,
            "querySelectorAll" => self.query_selector_all(params).await,
            "setAttributeValue" => self.set_attribute_value(params).await,
            "removeAttribute" => self.remove_attribute(params).await,
            "highlightNode" => self.highlight_node(params).await,
            "hideHighlight" => self.hide_highlight(params).await,
            "performSearch" => self.perform_search(params).await,
            "getSearchResults" => self.get_search_results(params).await,
            "discardSearchResults" => self.discard_search_results(params).await,
            "describeNode" => self.describe_node(params).await,
            _ => {
                warn!("Unknown DOMInspector method: {}", method);
                Err(CdpError::method_not_found(format!(
                    "DOMInspector.{}",
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
        let bridge = DomInspectorBridge::new();
        assert_eq!(bridge.name(), "DOMInspector");
    }

    #[tokio::test]
    async fn test_get_document() {
        let bridge = DomInspectorBridge::new();
        let result = bridge.get_document(None).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        assert!(value["root"].is_object());
        assert_eq!(value["root"]["nodeId"], 1);
    }

    #[tokio::test]
    async fn test_get_document_with_depth() {
        let bridge = DomInspectorBridge::new();
        let params = json!({ "depth": 3 });
        let result = bridge.get_document(Some(params)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_request_child_nodes() {
        let bridge = DomInspectorBridge::new();
        let params = json!({ "nodeId": 1, "depth": 1 });
        let result = bridge.request_child_nodes(Some(params)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_query_selector_success() {
        let bridge = DomInspectorBridge::new();
        let params = json!({
            "nodeId": 1,
            "selector": "#container"
        });

        let result = bridge.query_selector(Some(params)).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        assert_eq!(value["nodeId"], 6);
    }

    #[tokio::test]
    async fn test_query_selector_not_found() {
        let bridge = DomInspectorBridge::new();
        let params = json!({
            "nodeId": 1,
            "selector": ".nonexistent"
        });

        let result = bridge.query_selector(Some(params)).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        assert!(value["nodeId"].is_null());
    }

    #[tokio::test]
    async fn test_query_selector_all() {
        let bridge = DomInspectorBridge::new();
        let params = json!({
            "nodeId": 1,
            "selector": "div"
        });

        let result = bridge.query_selector_all(Some(params)).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        assert!(value["nodeIds"].is_array());
    }

    #[tokio::test]
    async fn test_set_attribute_value() {
        let bridge = DomInspectorBridge::new();
        let params = json!({
            "nodeId": 6,
            "name": "data-test",
            "value": "test-value"
        });

        let result = bridge.set_attribute_value(Some(params)).await;
        assert!(result.is_ok());

        // Verify attribute was set
        let node = bridge.browser().get_node(NodeId(6)).unwrap();
        let attrs = node.attributes.unwrap();
        assert!(attrs.contains(&"data-test".to_string()));
    }

    #[tokio::test]
    async fn test_remove_attribute() {
        let bridge = DomInspectorBridge::new();

        // First set an attribute
        let set_params = json!({
            "nodeId": 6,
            "name": "data-remove",
            "value": "value"
        });
        bridge.set_attribute_value(Some(set_params)).await.unwrap();

        // Then remove it
        let remove_params = json!({
            "nodeId": 6,
            "name": "data-remove"
        });
        let result = bridge.remove_attribute(Some(remove_params)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_highlight_node() {
        let bridge = DomInspectorBridge::new();
        let params = json!({
            "highlightConfig": {
                "showInfo": true,
                "showRulers": false
            },
            "nodeId": 6
        });

        let result = bridge.highlight_node(Some(params)).await;
        assert!(result.is_ok());

        let state = bridge.get_selection_state().await;
        assert_eq!(state.highlighted_node, Some(NodeId(6)));
    }

    #[tokio::test]
    async fn test_hide_highlight() {
        let bridge = DomInspectorBridge::new();

        // First highlight a node
        let highlight_params = json!({
            "highlightConfig": {},
            "nodeId": 6
        });
        bridge.highlight_node(Some(highlight_params)).await.unwrap();

        // Then hide it
        let result = bridge.hide_highlight(None).await;
        assert!(result.is_ok());

        let state = bridge.get_selection_state().await;
        assert!(state.highlighted_node.is_none());
    }

    #[tokio::test]
    async fn test_perform_search() {
        let bridge = DomInspectorBridge::new();
        let params = json!({
            "query": "Hello",
            "includeUserAgentShadowDOM": false
        });

        let result = bridge.perform_search(Some(params)).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        assert!(value["searchId"].is_string());
        assert!(value["resultCount"].is_number());
    }

    #[tokio::test]
    async fn test_get_search_results() {
        let bridge = DomInspectorBridge::new();

        // First perform a search
        let search_params = json!({
            "query": "div",
            "includeUserAgentShadowDOM": false
        });
        let search_result = bridge.perform_search(Some(search_params)).await.unwrap();
        let search_id = search_result["searchId"].as_str().unwrap();

        // Then get results
        let get_params = json!({
            "searchId": search_id,
            "fromIndex": 0,
            "toIndex": 10
        });
        let result = bridge.get_search_results(Some(get_params)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_discard_search_results() {
        let bridge = DomInspectorBridge::new();

        // First perform a search
        let search_params = json!({
            "query": "div",
            "includeUserAgentShadowDOM": false
        });
        let search_result = bridge.perform_search(Some(search_params)).await.unwrap();
        let search_id = search_result["searchId"].as_str().unwrap();

        // Then discard results
        let discard_params = json!({ "searchId": search_id });
        let result = bridge.discard_search_results(Some(discard_params)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_describe_node() {
        let bridge = DomInspectorBridge::new();
        let params = json!({
            "nodeId": 6,
            "depth": 1
        });

        let result = bridge.describe_node(Some(params)).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        assert!(value["node"].is_object());
        assert_eq!(value["node"]["nodeId"], 6);
    }

    #[tokio::test]
    async fn test_unknown_method() {
        let bridge = DomInspectorBridge::new();
        let result = bridge.handle_method("unknownMethod", None).await;
        assert!(result.is_err());
    }
}
