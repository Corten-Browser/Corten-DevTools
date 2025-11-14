//! CSS domain handler implementation
//!
//! Implements the Chrome DevTools Protocol CSS domain for inspecting and manipulating
//! CSS styles.

use async_trait::async_trait;
use cdp_types::CdpError;
use protocol_handler::DomainHandler;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tracing::{debug, warn};

use crate::mock_dom::MockDomBridge;

/// Parameters for CSS.getComputedStyleForNode
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GetComputedStyleForNodeParams {
    /// Node ID
    pub node_id: cdp_types::domains::dom::NodeId,
}

/// CSS domain handler
///
/// Handles CDP commands for CSS inspection and manipulation.
/// Uses a mock DOM bridge for testing; in production this would connect to the browser's CSS engine.
pub struct CssDomain {
    /// Mock DOM bridge for testing (provides computed styles)
    dom_bridge: Arc<MockDomBridge>,
}

impl CssDomain {
    /// Create a new CSS domain handler
    ///
    /// # Example
    /// ```
    /// use dom_domain::CssDomain;
    ///
    /// let css = CssDomain::new();
    /// ```
    pub fn new() -> Self {
        Self {
            dom_bridge: Arc::new(MockDomBridge::new()),
        }
    }

    /// Handle the getComputedStyleForNode method
    ///
    /// Returns the computed styles for a given DOM node.
    async fn get_computed_style_for_node(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!(
            "CSS.getComputedStyleForNode called with params: {:?}",
            params
        );

        let params: GetComputedStyleForNodeParams = serde_json::from_value(
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

        // Get computed styles
        let computed_styles = self.dom_bridge.get_computed_styles(params.node_id);

        // Create response matching CDP format
        let response = serde_json::json!({
            "computedStyle": computed_styles.properties
        });

        Ok(response)
    }
}

impl Default for CssDomain {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DomainHandler for CssDomain {
    fn name(&self) -> &str {
        "CSS"
    }

    async fn handle_method(&self, method: &str, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("CSS domain handling method: {}", method);

        match method {
            "getComputedStyleForNode" => self.get_computed_style_for_node(params).await,
            _ => {
                warn!("Unknown CSS method: {}", method);
                Err(CdpError::method_not_found(format!("CSS.{}", method)))
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
        let css = CssDomain::new();
        assert_eq!(css.name(), "CSS");
    }

    #[tokio::test]
    async fn test_get_computed_style_for_node() {
        let css = CssDomain::new();

        let params = json!({
            "nodeId": 2
        });

        let result = css.get_computed_style_for_node(Some(params)).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        assert!(value["computedStyle"].is_array());

        let styles = value["computedStyle"].as_array().unwrap();
        assert!(!styles.is_empty());

        // Check for expected properties
        let has_display = styles
            .iter()
            .any(|prop| prop["name"] == "display" && prop["value"] == "block");
        assert!(has_display);
    }

    #[tokio::test]
    async fn test_get_computed_style_invalid_node() {
        let css = CssDomain::new();

        let params = json!({
            "nodeId": 99999
        });

        let result = css.get_computed_style_for_node(Some(params)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_computed_style_missing_params() {
        let css = CssDomain::new();
        let result = css.get_computed_style_for_node(None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_handle_method_unknown() {
        let css = CssDomain::new();
        let result = css.handle_method("unknownMethod", None).await;
        assert!(result.is_err());
    }
}
