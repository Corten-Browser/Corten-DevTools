//! Security Domain Handler
//!
//! Implements the CDP Security domain for security monitoring and certificate handling.

use async_trait::async_trait;
use cdp_types::CdpError;
use parking_lot::RwLock;
use protocol_handler::DomainHandler;
use serde_json::{json, Value};
use std::sync::Arc;

/// Security domain handler
///
/// Provides methods for security monitoring and certificate error handling.
#[derive(Debug, Clone)]
pub struct SecurityDomain {
    state: Arc<RwLock<SecurityState>>,
}

#[derive(Debug)]
struct SecurityState {
    enabled: bool,
    ignore_certificate_errors: bool,
}

impl SecurityDomain {
    /// Create a new SecurityDomain instance
    ///
    /// # Example
    /// ```
    /// use browser_page_domains::SecurityDomain;
    ///
    /// let domain = SecurityDomain::new();
    /// ```
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(SecurityState {
                enabled: false,
                ignore_certificate_errors: false,
            })),
        }
    }

    /// Check if the domain is enabled
    pub fn is_enabled(&self) -> bool {
        self.state.read().enabled
    }

    /// Check if certificate errors are being ignored
    pub fn ignores_certificate_errors(&self) -> bool {
        self.state.read().ignore_certificate_errors
    }

    /// Enable security domain
    fn enable(&self) -> Result<Value, CdpError> {
        self.state.write().enabled = true;
        Ok(json!({}))
    }

    /// Disable security domain
    fn disable(&self) -> Result<Value, CdpError> {
        self.state.write().enabled = false;
        Ok(json!({}))
    }

    /// Set whether to ignore certificate errors
    fn set_ignore_certificate_errors(&self, params: Option<Value>) -> Result<Value, CdpError> {
        let params = params.ok_or_else(|| CdpError::invalid_params("Missing params"))?;

        let ignore = params["ignore"]
            .as_bool()
            .ok_or_else(|| CdpError::invalid_params("Missing 'ignore' parameter"))?;

        self.state.write().ignore_certificate_errors = ignore;

        Ok(json!({}))
    }

    /// Handle a certificate error
    fn handle_certificate_error(&self, params: Option<Value>) -> Result<Value, CdpError> {
        let params = params.ok_or_else(|| CdpError::invalid_params("Missing params"))?;

        let _event_id = params["eventId"]
            .as_i64()
            .ok_or_else(|| CdpError::invalid_params("Missing 'eventId' parameter"))?;

        let action = params["action"]
            .as_str()
            .ok_or_else(|| CdpError::invalid_params("Missing 'action' parameter"))?;

        // Validate action
        match action {
            "continue" | "cancel" => {}
            _ => {
                return Err(CdpError::invalid_params(
                    "Invalid action, must be 'continue' or 'cancel'",
                ))
            }
        }

        // In a real implementation, this would handle the certificate error
        // based on the action (continue or cancel)
        Ok(json!({}))
    }
}

impl Default for SecurityDomain {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DomainHandler for SecurityDomain {
    fn name(&self) -> &str {
        "Security"
    }

    async fn handle_method(&self, method: &str, params: Option<Value>) -> Result<Value, CdpError> {
        match method {
            "enable" => self.enable(),
            "disable" => self.disable(),
            "setIgnoreCertificateErrors" => self.set_ignore_certificate_errors(params),
            "handleCertificateError" => self.handle_certificate_error(params),
            _ => Err(CdpError::method_not_found(format!("Security.{}", method))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let domain = SecurityDomain::new();
        assert!(!domain.is_enabled());
        assert!(!domain.ignores_certificate_errors());
    }

    #[test]
    fn test_enable_disable() {
        let domain = SecurityDomain::new();
        assert!(!domain.is_enabled());

        domain.enable().unwrap();
        assert!(domain.is_enabled());

        domain.disable().unwrap();
        assert!(!domain.is_enabled());
    }

    #[tokio::test]
    async fn test_set_ignore_certificate_errors() {
        let domain = SecurityDomain::new();
        let params = json!({ "ignore": true });

        let result = domain
            .handle_method("setIgnoreCertificateErrors", Some(params))
            .await;
        assert!(result.is_ok());
        assert!(domain.ignores_certificate_errors());
    }

    #[tokio::test]
    async fn test_handle_certificate_error() {
        let domain = SecurityDomain::new();
        let params = json!({
            "eventId": 123,
            "action": "continue"
        });

        let result = domain
            .handle_method("handleCertificateError", Some(params))
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_certificate_error_invalid_action() {
        let domain = SecurityDomain::new();
        let params = json!({
            "eventId": 123,
            "action": "invalid"
        });

        let result = domain
            .handle_method("handleCertificateError", Some(params))
            .await;
        assert!(result.is_err());
    }
}
