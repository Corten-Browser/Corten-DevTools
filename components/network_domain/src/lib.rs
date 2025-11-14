//! Network monitoring and interception
//!
//! This module implements the Chrome DevTools Protocol (CDP) Network domain,
//! providing request/response monitoring, interception, and body retrieval capabilities.

use async_trait::async_trait;
use cdp_types::CdpError;
use dashmap::DashMap;
use parking_lot::RwLock;
use protocol_handler::DomainHandler;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::{debug, warn};

/// Information about a tracked network request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestInfo {
    /// Unique request identifier
    pub request_id: String,
    /// Request URL
    pub url: String,
    /// HTTP method (GET, POST, etc.)
    pub method: String,
    /// Response body (if available)
    pub response_body: Option<String>,
    /// Whether the response body is base64 encoded
    pub is_base64: bool,
}

/// Pattern for request interception
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterceptionPattern {
    /// URL pattern (supports wildcards)
    #[serde(rename = "urlPattern")]
    pub url_pattern: Option<String>,
    /// Resource type (Document, Image, Script, etc.)
    #[serde(rename = "resourceType")]
    pub resource_type: Option<String>,
    /// Interception stage (Request, HeadersReceived)
    #[serde(rename = "interceptionStage")]
    pub interception_stage: Option<String>,
}

/// Network domain implementation for Chrome DevTools Protocol
///
/// Provides network monitoring, request/response inspection, and interception capabilities.
#[derive(Debug)]
pub struct NetworkDomain {
    /// Map of tracked requests (RequestId → RequestInfo)
    request_map: Arc<DashMap<String, RequestInfo>>,
    /// Whether request interception is enabled
    interception_enabled: Arc<AtomicBool>,
    /// List of interception patterns
    interception_patterns: Arc<RwLock<Vec<InterceptionPattern>>>,
}

impl NetworkDomain {
    /// Create a new NetworkDomain instance
    ///
    /// # Example
    /// ```
    /// use network_domain::NetworkDomain;
    /// use protocol_handler::DomainHandler;
    ///
    /// let domain = NetworkDomain::new();
    /// assert_eq!(domain.name(), "Network");
    /// ```
    pub fn new() -> Self {
        Self {
            request_map: Arc::new(DashMap::new()),
            interception_enabled: Arc::new(AtomicBool::new(false)),
            interception_patterns: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Enable network monitoring
    ///
    /// # Arguments
    /// * `params` - Optional parameters including maxTotalBufferSize and maxResourceBufferSize
    ///
    /// # Returns
    /// Empty result on success, error on failure
    pub async fn enable(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("Network.enable called with params: {:?}", params);

        // In a real implementation, this would:
        // 1. Set up network observers
        // 2. Configure buffer sizes from params
        // 3. Start monitoring network activity

        // For now, just accept the parameters and return success
        if let Some(p) = params {
            if let Some(max_buffer) = p.get("maxTotalBufferSize") {
                debug!("Setting max total buffer size: {}", max_buffer);
            }
            if let Some(max_resource_buffer) = p.get("maxResourceBufferSize") {
                debug!("Setting max resource buffer size: {}", max_resource_buffer);
            }
        }

        Ok(json!({}))
    }

    /// Disable network monitoring
    ///
    /// # Returns
    /// Empty result on success
    pub async fn disable(&self) -> Result<Value, CdpError> {
        debug!("Network.disable called");

        // In a real implementation, this would:
        // 1. Stop network monitoring
        // 2. Clean up observers
        // 3. Clear tracked requests

        // For now, just return success
        Ok(json!({}))
    }

    /// Get response body for a given request
    ///
    /// # Arguments
    /// * `params` - Parameters containing the requestId
    ///
    /// # Returns
    /// Response body and base64Encoded flag, or error if request not found
    pub async fn get_response_body(&self, params: Option<Value>) -> Result<Value, CdpError> {
        let params = params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?;

        let request_id = params
            .get("requestId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CdpError::invalid_params("Missing requestId parameter"))?;

        debug!("Network.getResponseBody for request: {}", request_id);

        let request = self.request_map.get(request_id).ok_or_else(|| {
            CdpError::server_error(-32000, format!("Request not found: {}", request_id))
        })?;

        let body = request
            .response_body
            .clone()
            .ok_or_else(|| CdpError::server_error(-32000, "Response body not available"))?;

        Ok(json!({
            "body": body,
            "base64Encoded": request.is_base64
        }))
    }

    /// Enable request interception with specified patterns
    ///
    /// # Arguments
    /// * `params` - Parameters containing patterns array
    ///
    /// # Returns
    /// Empty result on success, error on failure
    pub async fn set_request_interception(&self, params: Option<Value>) -> Result<Value, CdpError> {
        let params = params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?;

        let patterns = params
            .get("patterns")
            .and_then(|v| v.as_array())
            .ok_or_else(|| CdpError::invalid_params("Missing patterns parameter"))?;

        debug!(
            "Network.setRequestInterception with {} patterns",
            patterns.len()
        );

        // Parse patterns
        let mut interception_patterns = Vec::new();
        for pattern_value in patterns {
            let pattern: InterceptionPattern = serde_json::from_value(pattern_value.clone())
                .map_err(|e| CdpError::invalid_params(format!("Invalid pattern: {}", e)))?;
            interception_patterns.push(pattern);
        }

        // Update interception state
        let enabled = !interception_patterns.is_empty();
        self.interception_enabled.store(enabled, Ordering::SeqCst);

        // Store patterns
        *self.interception_patterns.write() = interception_patterns;

        debug!("Request interception enabled: {}", enabled);

        Ok(json!({}))
    }

    /// Track a network request
    ///
    /// # Arguments
    /// * `request_id` - Unique identifier for the request
    /// * `url` - Request URL
    /// * `method` - HTTP method
    pub fn track_request(&self, request_id: String, url: String, method: String) {
        debug!("Tracking request: {} {} {}", method, url, request_id);

        let request_info = RequestInfo {
            request_id: request_id.clone(),
            url,
            method,
            response_body: None,
            is_base64: false,
        };

        self.request_map.insert(request_id, request_info);
    }

    /// Store response body for a tracked request
    ///
    /// # Arguments
    /// * `request_id` - Request identifier
    /// * `body` - Response body content
    /// * `is_base64` - Whether the body is base64 encoded
    pub fn store_response_body(&self, request_id: String, body: String, is_base64: bool) {
        debug!("Storing response body for request: {}", request_id);

        if let Some(mut request) = self.request_map.get_mut(&request_id) {
            request.response_body = Some(body);
            request.is_base64 = is_base64;
        } else {
            warn!(
                "Attempted to store response body for unknown request: {}",
                request_id
            );
        }
    }

    /// Check if a request is being tracked
    ///
    /// # Arguments
    /// * `request_id` - Request identifier to check
    ///
    /// # Returns
    /// true if the request is being tracked, false otherwise
    pub fn has_request(&self, request_id: &str) -> bool {
        self.request_map.contains_key(request_id)
    }

    /// Remove a tracked request
    ///
    /// # Arguments
    /// * `request_id` - Request identifier to remove
    pub fn untrack_request(&self, request_id: &str) {
        debug!("Untracking request: {}", request_id);
        self.request_map.remove(request_id);
    }

    /// Check if request interception is enabled
    ///
    /// # Returns
    /// true if interception is enabled, false otherwise
    pub fn is_interception_enabled(&self) -> bool {
        self.interception_enabled.load(Ordering::SeqCst)
    }
}

impl Default for NetworkDomain {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DomainHandler for NetworkDomain {
    /// Returns the name of this domain
    fn name(&self) -> &str {
        "Network"
    }

    /// Handle a method call for the Network domain
    ///
    /// # Arguments
    /// * `method` - Method name (without "Network." prefix)
    /// * `params` - Optional method parameters
    ///
    /// # Returns
    /// Method result or error
    async fn handle_method(&self, method: &str, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("Network domain handling method: {}", method);

        match method {
            "enable" => self.enable(params).await,
            "disable" => self.disable().await,
            "getResponseBody" => self.get_response_body(params).await,
            "setRequestInterception" => self.set_request_interception(params).await,
            _ => {
                warn!("Unknown Network method: {}", method);
                Err(CdpError::method_not_found(format!("Network.{}", method)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_new_domain() {
        let domain = NetworkDomain::new();
        assert_eq!(domain.name(), "Network");
    }

    #[tokio::test]
    async fn test_enable_basic() {
        let domain = NetworkDomain::new();
        let result = domain.enable(None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_track_and_retrieve() {
        let domain = NetworkDomain::new();

        domain.track_request(
            "test-123".to_string(),
            "https://example.com".to_string(),
            "GET".to_string(),
        );

        assert!(domain.has_request("test-123"));

        domain.store_response_body("test-123".to_string(), "Hello".to_string(), false);

        let params = json!({"requestId": "test-123"});
        let result = domain.get_response_body(Some(params)).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response["body"], "Hello");
    }

    #[tokio::test]
    async fn test_interception() {
        let domain = NetworkDomain::new();

        assert!(!domain.is_interception_enabled());

        let params = json!({
            "patterns": [{"urlPattern": "*"}]
        });

        domain.set_request_interception(Some(params)).await.unwrap();

        assert!(domain.is_interception_enabled());
    }
}
