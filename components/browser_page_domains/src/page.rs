//! Page Domain Handler
//!
//! Implements the CDP Page domain for page navigation and control.

use async_trait::async_trait;
use cdp_types::CdpError;
use parking_lot::RwLock;
use protocol_handler::DomainHandler;
use serde_json::{json, Value};
use std::sync::Arc;

/// Page domain handler
///
/// Provides methods for page navigation, reloading, and screenshot capture.
#[derive(Debug, Clone)]
pub struct PageDomain {
    state: Arc<RwLock<PageState>>,
}

#[derive(Debug)]
struct PageState {
    enabled: bool,
    current_url: Option<String>,
    frame_id: String,
}

impl PageDomain {
    /// Create a new PageDomain instance
    ///
    /// # Example
    /// ```
    /// use browser_page_domains::PageDomain;
    ///
    /// let domain = PageDomain::new();
    /// ```
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(PageState {
                enabled: false,
                current_url: None,
                frame_id: "main_frame".to_string(),
            })),
        }
    }

    /// Check if the domain is enabled
    pub fn is_enabled(&self) -> bool {
        self.state.read().enabled
    }

    /// Enable page domain
    fn enable(&self) -> Result<Value, CdpError> {
        self.state.write().enabled = true;
        Ok(json!({}))
    }

    /// Disable page domain
    fn disable(&self) -> Result<Value, CdpError> {
        self.state.write().enabled = false;
        Ok(json!({}))
    }

    /// Navigate to a URL
    fn navigate(&self, params: Option<Value>) -> Result<Value, CdpError> {
        let params = params.ok_or_else(|| CdpError::invalid_params("Missing params"))?;

        let url = params["url"]
            .as_str()
            .ok_or_else(|| CdpError::invalid_params("Missing 'url' parameter"))?;

        // Update state
        self.state.write().current_url = Some(url.to_string());

        Ok(json!({
            "frameId": self.state.read().frame_id,
            "loaderId": "loader_1"
        }))
    }

    /// Reload the current page
    fn reload(&self, _params: Option<Value>) -> Result<Value, CdpError> {
        // In a real implementation, this would trigger a page reload
        // For now, just return success
        Ok(json!({}))
    }

    /// Get the frame tree
    fn get_frame_tree(&self) -> Result<Value, CdpError> {
        let state = self.state.read();
        Ok(json!({
            "frameTree": {
                "frame": {
                    "id": state.frame_id,
                    "url": state.current_url.as_deref().unwrap_or("about:blank"),
                    "securityOrigin": "null",
                    "mimeType": "text/html"
                },
                "childFrames": []
            }
        }))
    }

    /// Capture a screenshot
    fn capture_screenshot(&self, _params: Option<Value>) -> Result<Value, CdpError> {
        // Mock screenshot - returns a base64 encoded empty PNG
        // In a real implementation, this would capture the actual rendered page
        let mock_png_base64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==";

        Ok(json!({
            "data": mock_png_base64
        }))
    }
}

impl Default for PageDomain {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DomainHandler for PageDomain {
    fn name(&self) -> &str {
        "Page"
    }

    async fn handle_method(&self, method: &str, params: Option<Value>) -> Result<Value, CdpError> {
        match method {
            "enable" => self.enable(),
            "disable" => self.disable(),
            "navigate" => self.navigate(params),
            "reload" => self.reload(params),
            "getFrameTree" => self.get_frame_tree(),
            "captureScreenshot" => self.capture_screenshot(params),
            _ => Err(CdpError::method_not_found(format!("Page.{}", method))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let domain = PageDomain::new();
        assert!(!domain.is_enabled());
    }

    #[test]
    fn test_enable_disable() {
        let domain = PageDomain::new();
        assert!(!domain.is_enabled());

        domain.enable().unwrap();
        assert!(domain.is_enabled());

        domain.disable().unwrap();
        assert!(!domain.is_enabled());
    }

    #[tokio::test]
    async fn test_navigate() {
        let domain = PageDomain::new();
        let params = json!({ "url": "https://example.com" });

        let result = domain.handle_method("navigate", Some(params)).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        assert!(value["frameId"].is_string());
    }

    #[tokio::test]
    async fn test_get_frame_tree() {
        let domain = PageDomain::new();
        let result = domain.handle_method("getFrameTree", None).await;

        assert!(result.is_ok());
        let value = result.unwrap();
        assert!(value["frameTree"]["frame"].is_object());
    }

    #[tokio::test]
    async fn test_capture_screenshot() {
        let domain = PageDomain::new();
        let result = domain.handle_method("captureScreenshot", None).await;

        assert!(result.is_ok());
        let value = result.unwrap();
        assert!(value["data"].is_string());
    }
}
