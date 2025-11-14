//! Browser Domain Handler
//!
//! Implements the CDP Browser domain for browser information and control.

use async_trait::async_trait;
use cdp_types::CdpError;
use protocol_handler::DomainHandler;
use serde_json::{json, Value};

/// Browser domain handler
///
/// Provides methods for retrieving browser information and controlling the browser process.
#[derive(Debug, Clone)]
pub struct BrowserDomain {
    // Browser metadata
    protocol_version: String,
    product: String,
    revision: String,
    user_agent: String,
    js_version: String,
}

impl BrowserDomain {
    /// Create a new BrowserDomain instance
    ///
    /// # Example
    /// ```
    /// use browser_page_domains::BrowserDomain;
    ///
    /// let domain = BrowserDomain::new();
    /// ```
    pub fn new() -> Self {
        Self {
            protocol_version: "1.3".to_string(),
            product: "CortenBrowser/1.0".to_string(),
            revision: env!("CARGO_PKG_VERSION").to_string(),
            user_agent: "Mozilla/5.0 (X11; Linux x86_64) CortenBrowser/1.0".to_string(),
            js_version: "Rust/1.0".to_string(),
        }
    }

    /// Get browser version information
    fn get_version(&self) -> Value {
        json!({
            "protocolVersion": self.protocol_version,
            "product": self.product,
            "revision": self.revision,
            "userAgent": self.user_agent,
            "jsVersion": self.js_version,
        })
    }

    /// Get browser command line arguments
    fn get_browser_command_line(&self) -> Value {
        let args: Vec<String> = std::env::args().collect();
        json!({
            "arguments": args
        })
    }

    /// Close the browser
    ///
    /// Note: This is a no-op in the current implementation as we don't
    /// have direct browser lifecycle control yet.
    fn close(&self) -> Value {
        json!({})
    }
}

impl Default for BrowserDomain {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DomainHandler for BrowserDomain {
    fn name(&self) -> &str {
        "Browser"
    }

    async fn handle_method(&self, method: &str, _params: Option<Value>) -> Result<Value, CdpError> {
        match method {
            "getVersion" => Ok(self.get_version()),
            "getBrowserCommandLine" => Ok(self.get_browser_command_line()),
            "close" => Ok(self.close()),
            _ => Err(CdpError::method_not_found(format!("Browser.{}", method))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let domain = BrowserDomain::new();
        assert_eq!(domain.protocol_version, "1.3");
        assert!(domain.product.contains("CortenBrowser"));
    }

    #[test]
    fn test_default() {
        let domain = BrowserDomain::default();
        assert_eq!(domain.name(), "Browser");
    }

    #[test]
    fn test_get_version() {
        let domain = BrowserDomain::new();
        let version = domain.get_version();

        assert_eq!(version["protocolVersion"], "1.3");
        assert!(version["product"].is_string());
        assert!(version["userAgent"].is_string());
    }

    #[test]
    fn test_get_browser_command_line() {
        let domain = BrowserDomain::new();
        let cmd_line = domain.get_browser_command_line();

        assert!(cmd_line["arguments"].is_array());
    }

    #[tokio::test]
    async fn test_handle_get_version() {
        let domain = BrowserDomain::new();
        let result = domain.handle_method("getVersion", None).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_unknown_method() {
        let domain = BrowserDomain::new();
        let result = domain.handle_method("unknownMethod", None).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, -32601); // Method not found
    }
}
