//! Console REPL and storage inspection
//!
//! This module implements the Console and Storage domains for the Chrome DevTools Protocol.
//! It provides console message management, logging, and storage inspection capabilities.

pub mod storage_types;

// Re-export main types
pub use storage_types::{Cookie, CookieSameSite, StorageType};

use async_trait::async_trait;
use cdp_types::domains::console::ConsoleMessage;
use cdp_types::CdpError;
use parking_lot::RwLock;
use protocol_handler::DomainHandler;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::{debug, warn};

/// Console domain handler
///
/// Implements the Chrome DevTools Protocol Console domain for managing console messages,
/// logging, and console API calls.
pub struct ConsoleDomain {
    /// Whether console monitoring is enabled
    enabled: Arc<AtomicBool>,
    /// Stored console messages
    messages: Arc<RwLock<Vec<ConsoleMessage>>>,
}

impl ConsoleDomain {
    /// Create a new ConsoleDomain instance
    pub fn new() -> Self {
        Self {
            enabled: Arc::new(AtomicBool::new(false)),
            messages: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Add a console message
    fn add_message(&self, message: ConsoleMessage) {
        self.messages.write().push(message);
    }

    /// Clear all console messages
    fn clear(&self) {
        self.messages.write().clear();
    }

    /// Get all console messages
    fn get_messages(&self) -> Vec<ConsoleMessage> {
        self.messages.read().clone()
    }
}

impl Default for ConsoleDomain {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DomainHandler for ConsoleDomain {
    fn name(&self) -> &str {
        "Console"
    }

    async fn handle_method(&self, method: &str, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("Console.{} called", method);

        match method {
            "enable" => {
                self.enabled.store(true, Ordering::SeqCst);
                Ok(json!({}))
            }
            "disable" => {
                self.enabled.store(false, Ordering::SeqCst);
                Ok(json!({}))
            }
            "clearMessages" => {
                self.clear();
                Ok(json!({}))
            }
            "messageAdded" => {
                let params = params
                    .ok_or_else(|| CdpError::invalid_params("messageAdded requires params"))?;

                let message: ConsoleMessage = serde_json::from_value(
                    params
                        .get("message")
                        .ok_or_else(|| CdpError::invalid_params("Missing 'message' field"))?
                        .clone(),
                )
                .map_err(|e| CdpError::invalid_params(format!("Invalid message format: {}", e)))?;

                self.add_message(message);
                Ok(json!({}))
            }
            "getMessages" => {
                let messages = self.get_messages();
                Ok(json!({
                    "messages": messages
                }))
            }
            _ => {
                warn!("Unknown Console method: {}", method);
                Err(CdpError::method_not_found(format!("Console.{}", method)))
            }
        }
    }
}

/// Storage domain handler
///
/// Implements the Chrome DevTools Protocol Storage domain for inspecting cookies,
/// localStorage, sessionStorage, and other storage mechanisms.
pub struct StorageDomain {
    /// Mock cookie storage for testing
    cookies: Arc<RwLock<Vec<Cookie>>>,
}

impl StorageDomain {
    /// Create a new StorageDomain instance
    pub fn new() -> Self {
        Self {
            cookies: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Get all cookies
    fn get_cookies(&self) -> Vec<Cookie> {
        self.cookies.read().clone()
    }

    /// Add or update a cookie
    fn set_cookie(&self, cookie: Cookie) {
        let mut cookies = self.cookies.write();

        // Remove existing cookie with same name and domain
        cookies.retain(|c| !(c.name == cookie.name && c.domain == cookie.domain));

        // Add new cookie
        cookies.push(cookie);
    }

    /// Clear all cookies
    fn clear_cookies(&self) {
        self.cookies.write().clear();
    }

    /// Delete a specific cookie
    fn delete_cookie(&self, name: &str, domain: &str) {
        let mut cookies = self.cookies.write();
        cookies.retain(|c| !(c.name == name && c.domain == domain));
    }
}

impl Default for StorageDomain {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DomainHandler for StorageDomain {
    fn name(&self) -> &str {
        "Storage"
    }

    async fn handle_method(&self, method: &str, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("Storage.{} called", method);

        match method {
            "getCookies" => {
                let cookies = self.get_cookies();
                Ok(json!({
                    "cookies": cookies
                }))
            }
            "setCookie" => {
                let params =
                    params.ok_or_else(|| CdpError::invalid_params("setCookie requires params"))?;

                // Extract required fields
                let name = params
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| CdpError::invalid_params("Missing 'name' field"))?
                    .to_string();

                let value = params
                    .get("value")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| CdpError::invalid_params("Missing 'value' field"))?
                    .to_string();

                let domain = params
                    .get("domain")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| CdpError::invalid_params("Missing 'domain' field"))?
                    .to_string();

                let path = params
                    .get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("/")
                    .to_string();

                // Calculate size
                let size = (name.len() + value.len()) as u32;

                let cookie = Cookie {
                    name,
                    value,
                    domain,
                    path,
                    expires: params.get("expires").and_then(|v| v.as_f64()),
                    size,
                    http_only: params.get("httpOnly").and_then(|v| v.as_bool()),
                    secure: params.get("secure").and_then(|v| v.as_bool()),
                    session: params.get("session").and_then(|v| v.as_bool()),
                    same_site: None, // Simplified for now
                };

                self.set_cookie(cookie);
                Ok(json!({}))
            }
            "clearCookies" => {
                self.clear_cookies();
                Ok(json!({}))
            }
            "deleteCookie" => {
                let params = params
                    .ok_or_else(|| CdpError::invalid_params("deleteCookie requires params"))?;

                let name = params
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| CdpError::invalid_params("Missing 'name' field"))?;

                let domain = params
                    .get("domain")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| CdpError::invalid_params("Missing 'domain' field"))?;

                self.delete_cookie(name, domain);
                Ok(json!({}))
            }
            _ => {
                warn!("Unknown Storage method: {}", method);
                Err(CdpError::method_not_found(format!("Storage.{}", method)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cdp_types::domains::console::{ConsoleMessageLevel, ConsoleMessageSource};
    use serde_json::json;

    // ============================================================================
    // ConsoleDomain Tests (TDD: RED Phase)
    // ============================================================================

    #[tokio::test]
    async fn test_console_domain_name() {
        let console = ConsoleDomain::new();
        assert_eq!(console.name(), "Console");
    }

    #[tokio::test]
    async fn test_console_enable() {
        let console = ConsoleDomain::new();

        let result = console.handle_method("enable", None).await;

        assert!(result.is_ok());
        assert!(console.enabled.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_console_disable() {
        let console = ConsoleDomain::new();

        // Enable first
        console.handle_method("enable", None).await.unwrap();
        assert!(console.enabled.load(Ordering::SeqCst));

        // Then disable
        let result = console.handle_method("disable", None).await;

        assert!(result.is_ok());
        assert!(!console.enabled.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_console_clear_messages() {
        let console = ConsoleDomain::new();

        // Add some messages first
        console.messages.write().push(ConsoleMessage {
            source: ConsoleMessageSource::Console,
            level: ConsoleMessageLevel::Log,
            text: "Test message".to_string(),
            url: None,
            line: None,
            column: None,
        });

        assert_eq!(console.messages.read().len(), 1);

        // Clear messages
        let result = console.handle_method("clearMessages", None).await;

        assert!(result.is_ok());
        assert_eq!(console.messages.read().len(), 0);
    }

    #[tokio::test]
    async fn test_console_message_added() {
        let console = ConsoleDomain::new();

        let params = json!({
            "message": {
                "source": "console",
                "level": "log",
                "text": "Hello, world!",
                "url": "http://example.com",
                "line": 10,
                "column": 5
            }
        });

        let result = console.handle_method("messageAdded", Some(params)).await;

        assert!(result.is_ok());
        assert_eq!(console.messages.read().len(), 1);

        let msg = &console.messages.read()[0];
        assert_eq!(msg.text, "Hello, world!");
        assert_eq!(msg.level, ConsoleMessageLevel::Log);
    }

    #[tokio::test]
    async fn test_console_get_messages() {
        let console = ConsoleDomain::new();

        // Add test messages
        console.messages.write().push(ConsoleMessage {
            source: ConsoleMessageSource::Console,
            level: ConsoleMessageLevel::Log,
            text: "Message 1".to_string(),
            url: None,
            line: None,
            column: None,
        });

        console.messages.write().push(ConsoleMessage {
            source: ConsoleMessageSource::Console,
            level: ConsoleMessageLevel::Error,
            text: "Message 2".to_string(),
            url: None,
            line: None,
            column: None,
        });

        let result = console.handle_method("getMessages", None).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.is_object());
        assert!(response["messages"].is_array());
        assert_eq!(response["messages"].as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_console_unknown_method() {
        let console = ConsoleDomain::new();

        let result = console.handle_method("unknownMethod", None).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, -32601); // Method not found
    }

    // ============================================================================
    // StorageDomain Tests (TDD: RED Phase)
    // ============================================================================

    #[tokio::test]
    async fn test_storage_domain_name() {
        let storage = StorageDomain::new();
        assert_eq!(storage.name(), "Storage");
    }

    #[tokio::test]
    async fn test_storage_get_cookies_empty() {
        let storage = StorageDomain::new();

        let result = storage.handle_method("getCookies", None).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response["cookies"].is_array());
        assert_eq!(response["cookies"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_storage_set_cookie() {
        let storage = StorageDomain::new();

        let params = json!({
            "name": "session",
            "value": "abc123",
            "domain": "example.com",
            "path": "/"
        });

        let result = storage.handle_method("setCookie", Some(params)).await;

        assert!(result.is_ok());
        assert_eq!(storage.cookies.read().len(), 1);

        let cookie = &storage.cookies.read()[0];
        assert_eq!(cookie.name, "session");
        assert_eq!(cookie.value, "abc123");
        assert_eq!(cookie.domain, "example.com");
    }

    #[tokio::test]
    async fn test_storage_get_cookies_after_set() {
        let storage = StorageDomain::new();

        // Set a cookie
        let params = json!({
            "name": "test",
            "value": "value",
            "domain": "example.com",
            "path": "/"
        });

        storage
            .handle_method("setCookie", Some(params))
            .await
            .unwrap();

        // Get cookies
        let result = storage.handle_method("getCookies", None).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response["cookies"].as_array().unwrap().len(), 1);
        assert_eq!(response["cookies"][0]["name"], "test");
    }

    #[tokio::test]
    async fn test_storage_clear_cookies() {
        let storage = StorageDomain::new();

        // Set some cookies
        let params1 = json!({
            "name": "cookie1",
            "value": "value1",
            "domain": "example.com",
            "path": "/"
        });

        let params2 = json!({
            "name": "cookie2",
            "value": "value2",
            "domain": "example.com",
            "path": "/"
        });

        storage
            .handle_method("setCookie", Some(params1))
            .await
            .unwrap();
        storage
            .handle_method("setCookie", Some(params2))
            .await
            .unwrap();

        assert_eq!(storage.cookies.read().len(), 2);

        // Clear cookies
        let result = storage.handle_method("clearCookies", None).await;

        assert!(result.is_ok());
        assert_eq!(storage.cookies.read().len(), 0);
    }

    #[tokio::test]
    async fn test_storage_delete_cookie() {
        let storage = StorageDomain::new();

        // Set cookies
        let params1 = json!({
            "name": "keep",
            "value": "value1",
            "domain": "example.com",
            "path": "/"
        });

        let params2 = json!({
            "name": "delete",
            "value": "value2",
            "domain": "example.com",
            "path": "/"
        });

        storage
            .handle_method("setCookie", Some(params1))
            .await
            .unwrap();
        storage
            .handle_method("setCookie", Some(params2))
            .await
            .unwrap();

        assert_eq!(storage.cookies.read().len(), 2);

        // Delete one cookie
        let delete_params = json!({
            "name": "delete",
            "domain": "example.com"
        });

        let result = storage
            .handle_method("deleteCookie", Some(delete_params))
            .await;

        assert!(result.is_ok());
        assert_eq!(storage.cookies.read().len(), 1);
        assert_eq!(storage.cookies.read()[0].name, "keep");
    }

    #[tokio::test]
    async fn test_storage_unknown_method() {
        let storage = StorageDomain::new();

        let result = storage.handle_method("unknownMethod", None).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, -32601); // Method not found
    }

    #[tokio::test]
    async fn test_storage_set_cookie_missing_params() {
        let storage = StorageDomain::new();

        let params = json!({
            "name": "test"
            // Missing required fields
        });

        let result = storage.handle_method("setCookie", Some(params)).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, -32602); // Invalid params
    }
}
