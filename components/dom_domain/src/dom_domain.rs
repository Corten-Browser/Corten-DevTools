//! DOM domain handler implementation
//!
//! Implements the Chrome DevTools Protocol DOM domain for inspecting and manipulating
//! the Document Object Model.

use async_trait::async_trait;
use cdp_types::{domains::dom::*, CdpError};
use protocol_handler::DomainHandler;
use serde_json::Value;
use std::sync::Arc;
use tracing::{debug, warn};

use crate::mock_dom::MockDomBridge;

/// DOM domain handler
///
/// Handles CDP commands for DOM inspection and manipulation.
/// Uses a mock DOM bridge for testing; in production this would connect to the browser's DOM.
pub struct DomDomain {
    /// Mock DOM bridge for testing
    dom_bridge: Arc<MockDomBridge>,
}

impl DomDomain {
    /// Create a new DOM domain handler
    ///
    /// # Example
    /// ```
    /// use dom_domain::DomDomain;
    ///
    /// let dom = DomDomain::new();
    /// ```
    pub fn new() -> Self {
        Self {
            dom_bridge: Arc::new(MockDomBridge::new()),
        }
    }

    /// Handle the getDocument method
    ///
    /// Returns the root DOM node for the current document.
    async fn get_document(&self) -> Result<Value, CdpError> {
        debug!("DOM.getDocument called");

        let root_node = self
            .dom_bridge
            .get_document()
            .ok_or_else(|| CdpError::internal_error("Failed to get document"))?;

        let response = GetDocumentResponse { root: root_node };

        serde_json::to_value(response)
            .map_err(|e| CdpError::internal_error(format!("Serialization error: {}", e)))
    }

    /// Handle the querySelector method
    ///
    /// Executes querySelector on a given node and returns the first matching element.
    async fn query_selector(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("DOM.querySelector called with params: {:?}", params);

        let params: QuerySelectorParams = serde_json::from_value(
            params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?,
        )
        .map_err(|e| CdpError::invalid_params(format!("Invalid parameters: {}", e)))?;

        // Verify the node exists
        if self.dom_bridge.get_node(params.node_id).is_none() {
            return Err(CdpError::server_error(
                -32000,
                format!("Node {} not found", params.node_id.0),
            ));
        }

        // Query for the selector
        let result_node_id = self
            .dom_bridge
            .query_selector(params.node_id, &params.selector);

        let response = QuerySelectorResponse {
            node_id: result_node_id,
        };

        serde_json::to_value(response)
            .map_err(|e| CdpError::internal_error(format!("Serialization error: {}", e)))
    }

    /// Handle the setAttributeValue method
    ///
    /// Sets an attribute value on a given node.
    async fn set_attribute_value(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("DOM.setAttributeValue called with params: {:?}", params);

        let params: SetAttributeValueParams = serde_json::from_value(
            params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?,
        )
        .map_err(|e| CdpError::invalid_params(format!("Invalid parameters: {}", e)))?;

        // Set the attribute
        self.dom_bridge
            .set_attribute(params.node_id, &params.name, &params.value)
            .map_err(|e| CdpError::server_error(-32000, e))?;

        // Return empty object for success
        Ok(serde_json::json!({}))
    }
}

impl Default for DomDomain {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DomainHandler for DomDomain {
    fn name(&self) -> &str {
        "DOM"
    }

    async fn handle_method(&self, method: &str, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("DOM domain handling method: {}", method);

        match method {
            "getDocument" => self.get_document().await,
            "querySelector" => self.query_selector(params).await,
            "setAttributeValue" => self.set_attribute_value(params).await,
            _ => {
                warn!("Unknown DOM method: {}", method);
                Err(CdpError::method_not_found(format!("DOM.{}", method)))
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
        let dom = DomDomain::new();
        assert_eq!(dom.name(), "DOM");
    }

    #[tokio::test]
    async fn test_get_document() {
        let dom = DomDomain::new();
        let result = dom.get_document().await;

        assert!(result.is_ok());
        let value = result.unwrap();

        assert!(value["root"].is_object());
        assert_eq!(value["root"]["nodeId"], 1);
        assert_eq!(value["root"]["nodeType"], 9);
    }

    #[tokio::test]
    async fn test_query_selector_success() {
        let dom = DomDomain::new();

        let params = json!({
            "nodeId": 1,
            "selector": "div"
        });

        let result = dom.query_selector(Some(params)).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        assert_eq!(value["nodeId"], 4);
    }

    #[tokio::test]
    async fn test_query_selector_not_found() {
        let dom = DomDomain::new();

        let params = json!({
            "nodeId": 1,
            "selector": ".nonexistent"
        });

        let result = dom.query_selector(Some(params)).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        assert!(value["nodeId"].is_null());
    }

    #[tokio::test]
    async fn test_query_selector_invalid_node() {
        let dom = DomDomain::new();

        let params = json!({
            "nodeId": 99999,
            "selector": "div"
        });

        let result = dom.query_selector(Some(params)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_query_selector_missing_params() {
        let dom = DomDomain::new();
        let result = dom.query_selector(None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_set_attribute_value() {
        let dom = DomDomain::new();

        let params = json!({
            "nodeId": 4,
            "name": "class",
            "value": "test-class"
        });

        let result = dom.set_attribute_value(Some(params)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_set_attribute_invalid_node() {
        let dom = DomDomain::new();

        let params = json!({
            "nodeId": 99999,
            "name": "class",
            "value": "test"
        });

        let result = dom.set_attribute_value(Some(params)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_handle_method_unknown() {
        let dom = DomDomain::new();
        let result = dom.handle_method("unknownMethod", None).await;
        assert!(result.is_err());
    }
}
