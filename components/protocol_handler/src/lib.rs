//! CDP message routing and domain registry
//!
//! This module provides the core protocol handling infrastructure for the Chrome DevTools Protocol.
//! It routes incoming CDP messages to registered domain handlers and manages responses.
//!
//! ## Features
//!
//! - **FEAT-041**: Message Batching - Batch CDP messages for efficiency

pub mod batching;
pub mod validation;

use async_trait::async_trait;
use cdp_types::{CdpError, CdpRequest, CdpResponse};
use dashmap::DashMap;
use serde_json::Value;
use std::sync::Arc;
use tracing::{debug, error, warn};

// Re-export validation types
pub use validation::{
    validate_cdp_request, validate_cdp_request_detailed, validate_method_name,
    MessageValidator, MessageValidatorConfig, ValidatedRequest, ValidationResult,
};

// Re-export batching types
pub use batching::{
    AsyncMessageBatcher, BatchConfig, BatchStats, BatchedEvent, EventBatch, MessageBatcher,
};

/// Trait that all domain handlers must implement
///
/// Domain handlers provide implementations for specific CDP domains (e.g., DOM, Network, Runtime).
/// Each handler is responsible for processing methods within its domain.
#[async_trait]
pub trait DomainHandler: Send + Sync {
    /// Returns the name of this domain (e.g., "DOM", "Network", "Runtime")
    fn name(&self) -> &str;

    /// Handle a method call for this domain
    ///
    /// # Arguments
    /// * `method` - The method name (without domain prefix, e.g., "getDocument")
    /// * `params` - Optional parameters for the method
    ///
    /// # Returns
    /// Result containing the method's return value or a CDP error
    async fn handle_method(&self, method: &str, params: Option<Value>) -> Result<Value, CdpError>;
}

/// Main protocol handler that routes CDP messages to appropriate domain handlers
///
/// The ProtocolHandler maintains a registry of domain handlers and routes incoming
/// CDP requests to the appropriate handler based on the method name.
pub struct ProtocolHandler {
    /// Registry of domain handlers, keyed by domain name
    domains: Arc<DashMap<String, Arc<dyn DomainHandler>>>,
}

impl ProtocolHandler {
    /// Create a new ProtocolHandler
    ///
    /// # Example
    /// ```
    /// use protocol_handler::ProtocolHandler;
    ///
    /// let handler = ProtocolHandler::new();
    /// ```
    pub fn new() -> Self {
        Self {
            domains: Arc::new(DashMap::new()),
        }
    }

    /// Register a domain handler
    ///
    /// # Arguments
    /// * `handler` - The domain handler to register
    ///
    /// # Example
    /// ```ignore
    /// let handler = ProtocolHandler::new();
    /// let dom_handler = Arc::new(DomDomainHandler::new());
    /// handler.register_domain(dom_handler);
    /// ```
    pub fn register_domain(&self, handler: Arc<dyn DomainHandler>) {
        let name = handler.name().to_string();
        debug!("Registering domain handler: {}", name);
        self.domains.insert(name, handler);
    }

    /// Unregister a domain handler
    ///
    /// # Arguments
    /// * `domain_name` - Name of the domain to unregister
    ///
    /// # Returns
    /// The removed handler, if it existed
    pub fn unregister_domain(&self, domain_name: &str) -> Option<Arc<dyn DomainHandler>> {
        debug!("Unregistering domain handler: {}", domain_name);
        self.domains.remove(domain_name).map(|(_, v)| v)
    }

    /// Handle an incoming CDP message
    ///
    /// Parses the message, validates it, routes it to the appropriate domain handler,
    /// and returns a JSON-formatted response.
    ///
    /// # Arguments
    /// * `message` - JSON string containing the CDP request
    ///
    /// # Returns
    /// JSON string containing the CDP response
    ///
    /// # Example
    /// ```ignore
    /// let response = handler.handle_message(r#"{"id": 1, "method": "DOM.getDocument"}"#).await;
    /// ```
    pub async fn handle_message(&self, message: &str) -> String {
        // Parse the message
        let request = match self.parse_request(message) {
            Ok(req) => req,
            Err(error) => {
                return self.create_error_response(None, error);
            }
        };

        let request_id = request.id;

        // Validate and route the request
        match self.route_request(&request).await {
            Ok(result) => self.create_success_response(request_id, result),
            Err(error) => self.create_error_response(Some(request_id), error),
        }
    }

    /// Parse a JSON string into a CDP request
    fn parse_request(&self, message: &str) -> Result<CdpRequest, CdpError> {
        // First, try to parse as generic JSON to distinguish parse errors from invalid requests
        let _json_check: Value = serde_json::from_str(message).map_err(|e| {
            error!("Invalid JSON: {}", e);
            CdpError::parse_error()
        })?;

        // Then try to parse as CdpRequest
        // If this fails, it's because the JSON is valid but doesn't match the request schema
        serde_json::from_str::<CdpRequest>(message).map_err(|e| {
            error!("Invalid CDP request structure: {}", e);
            CdpError::invalid_request()
        })
    }

    /// Validate and route a request to the appropriate domain handler
    async fn route_request(&self, request: &CdpRequest) -> Result<Value, CdpError> {
        // Validate the request has required fields
        if request.method.is_empty() {
            warn!("Request missing method field");
            return Err(CdpError::invalid_request());
        }

        // Parse the method into domain and method name
        let (domain_name, method_name) = self.parse_method(&request.method)?;

        debug!(
            "Routing request {} to domain: {}, method: {}",
            request.id, domain_name, method_name
        );

        // Look up the domain handler
        let handler = self
            .domains
            .get(domain_name)
            .ok_or_else(|| {
                warn!("Domain not found: {}", domain_name);
                CdpError::method_not_found(&request.method)
            })?
            .clone();

        // Call the domain handler
        handler
            .handle_method(method_name, request.params.clone())
            .await
    }

    /// Parse a method string into domain name and method name
    ///
    /// CDP methods have the format "Domain.method" (e.g., "DOM.getDocument")
    fn parse_method<'a>(&self, method: &'a str) -> Result<(&'a str, &'a str), CdpError> {
        let parts: Vec<&str> = method.splitn(2, '.').collect();

        if parts.len() != 2 {
            warn!("Invalid method format (expected Domain.method): {}", method);
            return Err(CdpError::invalid_request());
        }

        Ok((parts[0], parts[1]))
    }

    /// Create a success response
    fn create_success_response(&self, id: u64, result: Value) -> String {
        let response = CdpResponse {
            id,
            result: Some(result),
            error: None,
        };

        serde_json::to_string(&response).unwrap_or_else(|e| {
            error!("Failed to serialize response: {}", e);
            self.create_error_response(
                Some(id),
                CdpError::internal_error("Failed to serialize response"),
            )
        })
    }

    /// Create an error response
    fn create_error_response(&self, id: Option<u64>, error: CdpError) -> String {
        let response = CdpResponse {
            id: id.unwrap_or(0),
            result: None,
            error: Some(error),
        };

        serde_json::to_string(&response).unwrap_or_else(|e| {
            error!("Failed to serialize error response: {}", e);
            // Fallback to a minimal error response
            format!(
                r#"{{"id":{},"error":{{"code":-32603,"message":"Internal error"}}}}"#,
                id.unwrap_or(0)
            )
        })
    }
}

impl Default for ProtocolHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Mock domain handler for testing
    struct TestDomainHandler {
        name: String,
    }

    impl TestDomainHandler {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
            }
        }
    }

    #[async_trait]
    impl DomainHandler for TestDomainHandler {
        fn name(&self) -> &str {
            &self.name
        }

        async fn handle_method(
            &self,
            method: &str,
            params: Option<Value>,
        ) -> Result<Value, CdpError> {
            match method {
                "test" => Ok(json!({"success": true})),
                "echo" => Ok(params.unwrap_or(json!(null))),
                _ => Err(CdpError::method_not_found(format!(
                    "{}.{}",
                    self.name, method
                ))),
            }
        }
    }

    #[tokio::test]
    async fn test_protocol_handler_new() {
        let handler = ProtocolHandler::new();
        assert_eq!(handler.domains.len(), 0);
    }

    #[tokio::test]
    async fn test_register_domain() {
        let handler = ProtocolHandler::new();
        let test_domain = Arc::new(TestDomainHandler::new("Test"));

        handler.register_domain(test_domain);
        assert_eq!(handler.domains.len(), 1);
        assert!(handler.domains.contains_key("Test"));
    }

    #[tokio::test]
    async fn test_unregister_domain() {
        let handler = ProtocolHandler::new();
        let test_domain = Arc::new(TestDomainHandler::new("Test"));

        handler.register_domain(test_domain);
        assert_eq!(handler.domains.len(), 1);

        let removed = handler.unregister_domain("Test");
        assert!(removed.is_some());
        assert_eq!(handler.domains.len(), 0);
    }

    #[tokio::test]
    async fn test_parse_method() {
        let handler = ProtocolHandler::new();

        let result = handler.parse_method("DOM.getDocument");
        assert!(result.is_ok());
        let (domain, method) = result.unwrap();
        assert_eq!(domain, "DOM");
        assert_eq!(method, "getDocument");
    }

    #[tokio::test]
    async fn test_parse_method_invalid() {
        let handler = ProtocolHandler::new();

        let result = handler.parse_method("InvalidMethod");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_handle_message_success() {
        let handler = ProtocolHandler::new();
        let test_domain = Arc::new(TestDomainHandler::new("Test"));
        handler.register_domain(test_domain);

        let request = json!({
            "id": 1,
            "method": "Test.test"
        });

        let response = handler.handle_message(&request.to_string()).await;
        let response_json: Value = serde_json::from_str(&response).unwrap();

        assert_eq!(response_json["id"], 1);
        assert!(response_json["result"].is_object());
        assert_eq!(response_json["result"]["success"], true);
    }

    #[tokio::test]
    async fn test_handle_message_with_params() {
        let handler = ProtocolHandler::new();
        let test_domain = Arc::new(TestDomainHandler::new("Test"));
        handler.register_domain(test_domain);

        let request = json!({
            "id": 2,
            "method": "Test.echo",
            "params": {"test": "data"}
        });

        let response = handler.handle_message(&request.to_string()).await;
        let response_json: Value = serde_json::from_str(&response).unwrap();

        assert_eq!(response_json["id"], 2);
        assert_eq!(response_json["result"]["test"], "data");
    }

    #[tokio::test]
    async fn test_handle_message_unknown_domain() {
        let handler = ProtocolHandler::new();

        let request = json!({
            "id": 3,
            "method": "UnknownDomain.method"
        });

        let response = handler.handle_message(&request.to_string()).await;
        let response_json: Value = serde_json::from_str(&response).unwrap();

        assert_eq!(response_json["id"], 3);
        assert!(response_json["error"].is_object());
        assert_eq!(response_json["error"]["code"], -32601);
    }

    #[tokio::test]
    async fn test_handle_message_parse_error() {
        let handler = ProtocolHandler::new();

        let response = handler.handle_message("invalid json {{{").await;
        let response_json: Value = serde_json::from_str(&response).unwrap();

        assert!(response_json["error"].is_object());
        assert_eq!(response_json["error"]["code"], -32700);
    }

    #[tokio::test]
    async fn test_handle_message_invalid_method_format() {
        let handler = ProtocolHandler::new();

        let request = json!({
            "id": 4,
            "method": "InvalidMethodFormat"
        });

        let response = handler.handle_message(&request.to_string()).await;
        let response_json: Value = serde_json::from_str(&response).unwrap();

        assert_eq!(response_json["id"], 4);
        assert!(response_json["error"].is_object());
        assert_eq!(response_json["error"]["code"], -32600);
    }
}
