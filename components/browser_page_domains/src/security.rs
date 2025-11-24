//! Security Domain Handler
//!
//! Implements the CDP Security domain for security monitoring, certificate handling,
//! and security state tracking.
//!
//! # Features
//! - **FEAT-013**: Security Domain - CDP Security domain for certificate and security state
//!   - Security state tracking (secure, neutral, insecure, unknown)
//!   - Certificate details and error handling
//!   - Mixed content status tracking
//!   - Security state change events

use async_trait::async_trait;
use cdp_types::CdpError;
use parking_lot::RwLock;
use protocol_handler::DomainHandler;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::debug;

// =============================================================================
// Security State Types
// =============================================================================

/// The security state of a page
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SecurityState {
    /// The page is secure (valid HTTPS)
    Secure,
    /// The page is neutral (HTTP on localhost, etc.)
    Neutral,
    /// The page is insecure (mixed content, invalid certificate)
    Insecure,
    /// The security state is unknown
    #[default]
    Unknown,
}

impl SecurityState {
    /// Returns the string representation of the security state
    pub fn as_str(&self) -> &'static str {
        match self {
            SecurityState::Secure => "secure",
            SecurityState::Neutral => "neutral",
            SecurityState::Insecure => "insecure",
            SecurityState::Unknown => "unknown",
        }
    }

    /// Creates a SecurityState from a string
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "secure" => SecurityState::Secure,
            "neutral" => SecurityState::Neutral,
            "insecure" => SecurityState::Insecure,
            _ => SecurityState::Unknown,
        }
    }
}

/// Mixed content type for a resource
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum MixedContentType {
    /// Blockable mixed content (scripts, iframes, etc.)
    Blockable,
    /// Optionally-blockable mixed content (images, audio, video)
    OptionallyBlockable,
    /// No mixed content
    #[default]
    None,
}

/// Enum for certificate error actions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CertificateErrorAction {
    /// Continue despite the certificate error
    Continue,
    /// Cancel the request due to certificate error
    Cancel,
}

// =============================================================================
// Certificate Types
// =============================================================================

/// Information about a certificate
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CertificateDetails {
    /// Subject name
    pub subject_name: String,
    /// Issuer name
    pub issuer_name: String,
    /// Certificate valid from (Unix timestamp in ms)
    pub valid_from: f64,
    /// Certificate valid to (Unix timestamp in ms)
    pub valid_to: f64,
    /// Certificate serial number (hex string)
    pub serial_number: String,
    /// Certificate fingerprint (SHA-256)
    pub fingerprint: String,
    /// Subject Alternative Names (SANs)
    #[serde(default)]
    pub san_list: Vec<String>,
    /// Key exchange algorithm
    pub key_exchange: Option<String>,
    /// Cipher suite
    pub cipher: Option<String>,
    /// Protocol (e.g., "TLS 1.3")
    pub protocol: Option<String>,
}

impl CertificateDetails {
    /// Create a new CertificateDetails with basic information
    pub fn new(subject_name: String, issuer_name: String) -> Self {
        Self {
            subject_name,
            issuer_name,
            valid_from: 0.0,
            valid_to: 0.0,
            serial_number: String::new(),
            fingerprint: String::new(),
            san_list: Vec::new(),
            key_exchange: None,
            cipher: None,
            protocol: None,
        }
    }

    /// Check if the certificate is currently valid
    pub fn is_valid(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs_f64() * 1000.0)
            .unwrap_or(0.0);
        now >= self.valid_from && now <= self.valid_to
    }
}

/// A pending certificate error awaiting user decision
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CertificateError {
    /// Unique identifier for this error event
    pub event_id: u64,
    /// The type of error (e.g., "CERT_AUTHORITY_INVALID")
    pub error_type: String,
    /// The URL that caused the error
    pub url: String,
    /// Request ID associated with this error
    pub request_id: String,
    /// Timestamp when the error occurred (ms since epoch)
    pub timestamp: f64,
}

impl CertificateError {
    /// Create a new certificate error
    pub fn new(event_id: u64, error_type: String, url: String, request_id: String) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs_f64() * 1000.0)
            .unwrap_or(0.0);

        Self {
            event_id,
            error_type,
            url,
            request_id,
            timestamp,
        }
    }
}

// =============================================================================
// Security State Explanation
// =============================================================================

/// Explanation for a security state
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityStateExplanation {
    /// The security state this explanation refers to
    pub security_state: SecurityState,
    /// Title of the explanation
    pub title: String,
    /// Summary description
    pub summary: String,
    /// Detailed description
    pub description: String,
    /// Mixed content type (if applicable)
    pub mixed_content_type: MixedContentType,
    /// Certificate details (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub certificate: Option<CertificateDetails>,
    /// Recommendations for fixing issues
    #[serde(default)]
    pub recommendations: Vec<String>,
}

impl SecurityStateExplanation {
    /// Create a new security state explanation
    pub fn new(
        security_state: SecurityState,
        title: String,
        summary: String,
        description: String,
    ) -> Self {
        Self {
            security_state,
            title,
            summary,
            description,
            mixed_content_type: MixedContentType::None,
            certificate: None,
            recommendations: Vec::new(),
        }
    }
}

// =============================================================================
// Insecure Content Status
// =============================================================================

/// Status of insecure content on the page
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct InsecureContentStatus {
    /// Whether the page ran insecure content (e.g., scripts over HTTP)
    pub ran_insecure_content: bool,
    /// Whether the page displayed insecure content (e.g., images over HTTP)
    pub displayed_insecure_content: bool,
    /// Whether the page contained insecure forms
    pub contained_mixed_form: bool,
    /// Whether the page ran content with certificate errors
    pub ran_content_with_cert_errors: bool,
    /// Whether the page displayed content with certificate errors
    pub displayed_content_with_cert_errors: bool,
    /// URLs of insecure origins
    #[serde(default)]
    pub insecure_origins: Vec<String>,
}

// =============================================================================
// Visible Security State
// =============================================================================

/// The overall visible security state of the page
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisibleSecurityState {
    /// The overall security state
    pub security_state: SecurityState,
    /// Security state of the certificate
    pub certificate_security_state: Option<CertificateSecurityState>,
    /// Security state as it pertains to safe browsing
    pub safe_browsing_state: Option<SafeBrowsingState>,
    /// Explanations for the security state
    #[serde(default)]
    pub security_state_issue_ids: Vec<String>,
}

/// Certificate-specific security state
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CertificateSecurityState {
    /// Protocol (e.g., "TLS 1.3")
    pub protocol: String,
    /// Key exchange algorithm
    pub key_exchange: String,
    /// Cipher suite
    pub cipher: String,
    /// Certificate chain
    #[serde(default)]
    pub certificate: Vec<String>,
    /// Subject name
    pub subject_name: String,
    /// Issuer name
    pub issuer: String,
    /// Valid from (Unix timestamp)
    pub valid_from: f64,
    /// Valid to (Unix timestamp)
    pub valid_to: f64,
    /// Whether the certificate has Certificate Transparency compliance
    pub certificate_has_ct_compliance: bool,
    /// Whether the certificate has a weak signature
    pub certificate_has_weak_signature: bool,
    /// Whether the certificate uses an obsolete cipher suite
    pub obsolete_ssl_cipher: bool,
    /// Whether the certificate uses an obsolete key exchange
    pub obsolete_ssl_key_exchange: bool,
    /// Whether the certificate uses obsolete SSL signature
    pub obsolete_ssl_signature: bool,
    /// Whether the certificate uses obsolete SSL version
    pub obsolete_ssl_version: bool,
}

/// Safe browsing state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SafeBrowsingState {
    /// Safe - no threats detected
    #[default]
    Safe,
    /// Malware detected
    Malware,
    /// Social engineering (phishing) detected
    SocialEngineering,
    /// Unwanted software detected
    UnwantedSoftware,
    /// Potentially harmful application
    PotentiallyHarmfulApplication,
}

// =============================================================================
// Security Domain State
// =============================================================================

#[derive(Debug)]
struct SecurityDomainState {
    /// Whether the domain is enabled
    enabled: bool,
    /// Whether to ignore certificate errors
    ignore_certificate_errors: bool,
    /// Whether certificate error override mode is enabled
    override_certificate_errors: bool,
    /// Current security state
    security_state: SecurityState,
    /// Certificate details for the current page
    certificate: Option<CertificateDetails>,
    /// Insecure content status
    insecure_content: InsecureContentStatus,
    /// Security state explanations
    explanations: Vec<SecurityStateExplanation>,
    /// Pending certificate errors (event_id -> error)
    pending_certificate_errors: HashMap<u64, CertificateError>,
    /// Queued events to be emitted
    event_queue: Vec<Value>,
}

impl Default for SecurityDomainState {
    fn default() -> Self {
        Self {
            enabled: false,
            ignore_certificate_errors: false,
            override_certificate_errors: false,
            security_state: SecurityState::Unknown,
            certificate: None,
            insecure_content: InsecureContentStatus::default(),
            explanations: Vec::new(),
            pending_certificate_errors: HashMap::new(),
            event_queue: Vec::new(),
        }
    }
}

// =============================================================================
// Security Domain Handler
// =============================================================================

/// Security domain handler
///
/// Provides methods for security monitoring, certificate error handling,
/// and security state tracking.
///
/// # CDP Methods
/// - `Security.enable` - Enable security domain notifications
/// - `Security.disable` - Disable security domain notifications
/// - `Security.setIgnoreCertificateErrors` - Ignore certificate errors globally
/// - `Security.handleCertificateError` - Handle a specific certificate error
/// - `Security.setOverrideCertificateErrors` - Enable override mode for certificate errors
/// - `Security.getSecurityState` - Get current security state (non-standard)
///
/// # CDP Events
/// - `Security.securityStateChanged` - Fired when security state changes
/// - `Security.certificateError` - Fired when a certificate error occurs
#[derive(Debug)]
pub struct SecurityDomain {
    state: Arc<RwLock<SecurityDomainState>>,
    /// Counter for generating unique event IDs
    event_id_counter: Arc<AtomicU64>,
}

impl Clone for SecurityDomain {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
            event_id_counter: Arc::clone(&self.event_id_counter),
        }
    }
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
            state: Arc::new(RwLock::new(SecurityDomainState::default())),
            event_id_counter: Arc::new(AtomicU64::new(1)),
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

    /// Check if certificate error override mode is enabled
    pub fn is_override_mode_enabled(&self) -> bool {
        self.state.read().override_certificate_errors
    }

    /// Get the current security state
    pub fn get_security_state(&self) -> SecurityState {
        self.state.read().security_state
    }

    /// Get certificate details for the current page
    pub fn get_certificate_details(&self) -> Option<CertificateDetails> {
        self.state.read().certificate.clone()
    }

    /// Get insecure content status
    pub fn get_insecure_content_status(&self) -> InsecureContentStatus {
        self.state.read().insecure_content.clone()
    }

    /// Take all queued events (returns and clears the queue)
    pub fn take_events(&self) -> Vec<Value> {
        std::mem::take(&mut self.state.write().event_queue)
    }

    /// Check if there are pending events
    pub fn has_pending_events(&self) -> bool {
        !self.state.read().event_queue.is_empty()
    }

    /// Get count of pending certificate errors
    pub fn pending_certificate_error_count(&self) -> usize {
        self.state.read().pending_certificate_errors.len()
    }

    // =========================================================================
    // CDP Methods
    // =========================================================================

    /// Enable security domain
    fn enable(&self) -> Result<Value, CdpError> {
        debug!("Security.enable called");
        self.state.write().enabled = true;

        // Emit initial security state
        self.emit_security_state_changed();

        Ok(json!({}))
    }

    /// Disable security domain
    fn disable(&self) -> Result<Value, CdpError> {
        debug!("Security.disable called");
        let mut state = self.state.write();
        state.enabled = false;
        // Clear pending errors and events when disabled
        state.pending_certificate_errors.clear();
        state.event_queue.clear();
        Ok(json!({}))
    }

    /// Set whether to ignore certificate errors
    fn set_ignore_certificate_errors(&self, params: Option<Value>) -> Result<Value, CdpError> {
        let params = params.ok_or_else(|| CdpError::invalid_params("Missing params"))?;

        let ignore = params["ignore"]
            .as_bool()
            .ok_or_else(|| CdpError::invalid_params("Missing 'ignore' parameter"))?;

        debug!("Security.setIgnoreCertificateErrors: {}", ignore);
        self.state.write().ignore_certificate_errors = ignore;

        Ok(json!({}))
    }

    /// Handle a certificate error
    fn handle_certificate_error(&self, params: Option<Value>) -> Result<Value, CdpError> {
        let params = params.ok_or_else(|| CdpError::invalid_params("Missing params"))?;

        let event_id = params["eventId"]
            .as_u64()
            .ok_or_else(|| CdpError::invalid_params("Missing 'eventId' parameter"))?;

        let action = params["action"]
            .as_str()
            .ok_or_else(|| CdpError::invalid_params("Missing 'action' parameter"))?;

        debug!(
            "Security.handleCertificateError: eventId={}, action={}",
            event_id, action
        );

        // Validate action
        let _action = match action {
            "continue" => CertificateErrorAction::Continue,
            "cancel" => CertificateErrorAction::Cancel,
            _ => {
                return Err(CdpError::invalid_params(
                    "Invalid action, must be 'continue' or 'cancel'",
                ))
            }
        };

        // Remove the pending error
        let mut state = self.state.write();
        if state.pending_certificate_errors.remove(&event_id).is_none() {
            return Err(CdpError::invalid_params(format!(
                "No pending certificate error with eventId: {}",
                event_id
            )));
        }

        Ok(json!({}))
    }

    /// Set override mode for certificate errors
    fn set_override_certificate_errors(&self, params: Option<Value>) -> Result<Value, CdpError> {
        let params = params.ok_or_else(|| CdpError::invalid_params("Missing params"))?;

        let override_enabled = params["override"]
            .as_bool()
            .ok_or_else(|| CdpError::invalid_params("Missing 'override' parameter"))?;

        debug!(
            "Security.setOverrideCertificateErrors: {}",
            override_enabled
        );
        self.state.write().override_certificate_errors = override_enabled;

        Ok(json!({}))
    }

    /// Get current security state (non-standard method for inspection)
    fn get_current_security_state(&self) -> Result<Value, CdpError> {
        let state = self.state.read();

        let mut result = json!({
            "securityState": state.security_state.as_str(),
            "schemeIsCryptographic": matches!(state.security_state, SecurityState::Secure | SecurityState::Neutral),
            "explanations": state.explanations.iter().map(|e| json!({
                "securityState": e.security_state.as_str(),
                "title": e.title,
                "summary": e.summary,
                "description": e.description,
                "mixedContentType": e.mixed_content_type,
                "recommendations": e.recommendations
            })).collect::<Vec<_>>(),
            "insecureContentStatus": {
                "ranInsecureContent": state.insecure_content.ran_insecure_content,
                "displayedInsecureContent": state.insecure_content.displayed_insecure_content,
                "containedMixedForm": state.insecure_content.contained_mixed_form,
                "ranContentWithCertErrors": state.insecure_content.ran_content_with_cert_errors,
                "displayedContentWithCertErrors": state.insecure_content.displayed_content_with_cert_errors
            }
        });

        if let Some(cert) = &state.certificate {
            result["certificate"] = json!({
                "subjectName": cert.subject_name,
                "issuerName": cert.issuer_name,
                "validFrom": cert.valid_from,
                "validTo": cert.valid_to,
                "serialNumber": cert.serial_number,
                "fingerprint": cert.fingerprint,
                "sanList": cert.san_list,
                "keyExchange": cert.key_exchange,
                "cipher": cert.cipher,
                "protocol": cert.protocol
            });
        }

        Ok(result)
    }

    // =========================================================================
    // Event Emission
    // =========================================================================

    /// Emit a security state changed event
    fn emit_security_state_changed(&self) {
        let state = self.state.read();
        if !state.enabled {
            return;
        }

        let event = json!({
            "method": "Security.securityStateChanged",
            "params": {
                "securityState": state.security_state.as_str(),
                "schemeIsCryptographic": matches!(state.security_state, SecurityState::Secure | SecurityState::Neutral),
                "explanations": state.explanations.iter().map(|e| json!({
                    "securityState": e.security_state.as_str(),
                    "title": e.title,
                    "summary": e.summary,
                    "description": e.description,
                    "mixedContentType": e.mixed_content_type
                })).collect::<Vec<_>>(),
                "insecureContentStatus": {
                    "ranInsecureContent": state.insecure_content.ran_insecure_content,
                    "displayedInsecureContent": state.insecure_content.displayed_insecure_content,
                    "containedMixedForm": state.insecure_content.contained_mixed_form,
                    "ranContentWithCertErrors": state.insecure_content.ran_content_with_cert_errors,
                    "displayedContentWithCertErrors": state.insecure_content.displayed_content_with_cert_errors
                },
                "summary": format!("Security state: {}", state.security_state.as_str())
            }
        });

        drop(state);
        self.state.write().event_queue.push(event);
    }

    /// Emit a certificate error event
    fn emit_certificate_error(&self, error: &CertificateError) {
        let state = self.state.read();
        if !state.enabled || !state.override_certificate_errors {
            return;
        }

        let event = json!({
            "method": "Security.certificateError",
            "params": {
                "eventId": error.event_id,
                "errorType": error.error_type,
                "requestUrl": error.url
            }
        });

        drop(state);
        self.state.write().event_queue.push(event);
    }

    // =========================================================================
    // Public API for Browser Integration
    // =========================================================================

    /// Update the security state (called by browser integration)
    pub fn update_security_state(&self, new_state: SecurityState) {
        let mut state = self.state.write();
        let old_state = state.security_state;
        state.security_state = new_state;
        drop(state);

        if old_state != new_state {
            self.emit_security_state_changed();
        }
    }

    /// Update certificate details (called by browser integration)
    pub fn update_certificate(&self, certificate: CertificateDetails) {
        let mut state = self.state.write();
        state.certificate = Some(certificate);
        drop(state);

        self.emit_security_state_changed();
    }

    /// Update insecure content status (called by browser integration)
    pub fn update_insecure_content(&self, status: InsecureContentStatus) {
        let mut state = self.state.write();
        state.insecure_content = status;

        // Update security state based on insecure content
        if state.insecure_content.ran_insecure_content
            || state.insecure_content.ran_content_with_cert_errors
        {
            state.security_state = SecurityState::Insecure;
        } else if state.insecure_content.displayed_insecure_content
            || state.insecure_content.displayed_content_with_cert_errors
        {
            if state.security_state == SecurityState::Secure {
                state.security_state = SecurityState::Neutral;
            }
        }

        drop(state);
        self.emit_security_state_changed();
    }

    /// Add a security state explanation
    pub fn add_explanation(&self, explanation: SecurityStateExplanation) {
        self.state.write().explanations.push(explanation);
        self.emit_security_state_changed();
    }

    /// Clear all explanations
    pub fn clear_explanations(&self) {
        self.state.write().explanations.clear();
    }

    /// Report a certificate error (called by browser integration)
    pub fn report_certificate_error(
        &self,
        error_type: String,
        url: String,
        request_id: String,
    ) -> Option<u64> {
        let state = self.state.read();

        // If ignoring certificate errors, don't report
        if state.ignore_certificate_errors {
            return None;
        }

        // If override mode is not enabled, don't queue for user decision
        if !state.override_certificate_errors {
            return None;
        }

        drop(state);

        let event_id = self.event_id_counter.fetch_add(1, Ordering::SeqCst);
        let error = CertificateError::new(event_id, error_type, url, request_id);

        // Queue the error and emit event
        self.state
            .write()
            .pending_certificate_errors
            .insert(event_id, error.clone());
        self.emit_certificate_error(&error);

        Some(event_id)
    }

    /// Check if a certificate error should be ignored
    pub fn should_ignore_certificate_error(&self, _error_type: &str) -> bool {
        self.state.read().ignore_certificate_errors
    }

    /// Set security state for a new navigation
    pub fn on_navigation(&self, url: &str) {
        let mut state = self.state.write();

        // Reset state on navigation
        state.explanations.clear();
        state.insecure_content = InsecureContentStatus::default();

        // Determine initial security state based on URL scheme
        state.security_state = if url.starts_with("https://") {
            SecurityState::Secure
        } else if url.starts_with("http://localhost")
            || url.starts_with("http://127.0.0.1")
            || url.starts_with("file://")
            || url.starts_with("about:")
            || url.starts_with("chrome://")
            || url.starts_with("data:")
        {
            SecurityState::Neutral
        } else if url.starts_with("http://") {
            SecurityState::Insecure
        } else {
            SecurityState::Unknown
        };

        drop(state);
        self.emit_security_state_changed();
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
        debug!("Security domain handling method: {}", method);

        match method {
            "enable" => self.enable(),
            "disable" => self.disable(),
            "setIgnoreCertificateErrors" => self.set_ignore_certificate_errors(params),
            "handleCertificateError" => self.handle_certificate_error(params),
            "setOverrideCertificateErrors" => self.set_override_certificate_errors(params),
            // Non-standard method for getting current state
            "getSecurityState" => self.get_current_security_state(),
            _ => Err(CdpError::method_not_found(format!("Security.{}", method))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // SecurityState Tests
    // =========================================================================

    #[test]
    fn test_security_state_as_str() {
        assert_eq!(SecurityState::Secure.as_str(), "secure");
        assert_eq!(SecurityState::Neutral.as_str(), "neutral");
        assert_eq!(SecurityState::Insecure.as_str(), "insecure");
        assert_eq!(SecurityState::Unknown.as_str(), "unknown");
    }

    #[test]
    fn test_security_state_from_str() {
        assert_eq!(SecurityState::from_str("secure"), SecurityState::Secure);
        assert_eq!(SecurityState::from_str("SECURE"), SecurityState::Secure);
        assert_eq!(SecurityState::from_str("neutral"), SecurityState::Neutral);
        assert_eq!(SecurityState::from_str("insecure"), SecurityState::Insecure);
        assert_eq!(SecurityState::from_str("unknown"), SecurityState::Unknown);
        assert_eq!(SecurityState::from_str("invalid"), SecurityState::Unknown);
    }

    #[test]
    fn test_security_state_default() {
        let state: SecurityState = Default::default();
        assert_eq!(state, SecurityState::Unknown);
    }

    // =========================================================================
    // CertificateDetails Tests
    // =========================================================================

    #[test]
    fn test_certificate_details_new() {
        let cert = CertificateDetails::new("example.com".to_string(), "CA".to_string());
        assert_eq!(cert.subject_name, "example.com");
        assert_eq!(cert.issuer_name, "CA");
        assert!(cert.san_list.is_empty());
    }

    #[test]
    fn test_certificate_is_valid() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs_f64()
            * 1000.0;

        let mut cert = CertificateDetails::new("example.com".to_string(), "CA".to_string());
        cert.valid_from = now - 1000.0;
        cert.valid_to = now + 1000.0;
        assert!(cert.is_valid());

        // Expired certificate
        let mut expired_cert = CertificateDetails::new("example.com".to_string(), "CA".to_string());
        expired_cert.valid_from = now - 2000.0;
        expired_cert.valid_to = now - 1000.0;
        assert!(!expired_cert.is_valid());
    }

    // =========================================================================
    // SecurityDomain Basic Tests
    // =========================================================================

    #[test]
    fn test_new() {
        let domain = SecurityDomain::new();
        assert!(!domain.is_enabled());
        assert!(!domain.ignores_certificate_errors());
        assert!(!domain.is_override_mode_enabled());
        assert_eq!(domain.get_security_state(), SecurityState::Unknown);
    }

    #[test]
    fn test_default() {
        let domain = SecurityDomain::default();
        assert!(!domain.is_enabled());
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

        let params = json!({ "ignore": false });
        domain
            .handle_method("setIgnoreCertificateErrors", Some(params))
            .await
            .unwrap();
        assert!(!domain.ignores_certificate_errors());
    }

    #[tokio::test]
    async fn test_set_ignore_certificate_errors_missing_param() {
        let domain = SecurityDomain::new();
        let result = domain
            .handle_method("setIgnoreCertificateErrors", None)
            .await;
        assert!(result.is_err());

        let result = domain
            .handle_method("setIgnoreCertificateErrors", Some(json!({})))
            .await;
        assert!(result.is_err());
    }

    // =========================================================================
    // Certificate Error Handling Tests
    // =========================================================================

    #[tokio::test]
    async fn test_handle_certificate_error() {
        let domain = SecurityDomain::new();
        domain.enable().unwrap();

        // Enable override mode and report an error first
        domain
            .handle_method("setOverrideCertificateErrors", Some(json!({ "override": true })))
            .await
            .unwrap();

        let event_id = domain
            .report_certificate_error(
                "CERT_AUTHORITY_INVALID".to_string(),
                "https://example.com".to_string(),
                "req-1".to_string(),
            )
            .unwrap();

        assert_eq!(domain.pending_certificate_error_count(), 1);

        let params = json!({
            "eventId": event_id,
            "action": "continue"
        });

        let result = domain
            .handle_method("handleCertificateError", Some(params))
            .await;
        assert!(result.is_ok());
        assert_eq!(domain.pending_certificate_error_count(), 0);
    }

    #[tokio::test]
    async fn test_handle_certificate_error_cancel() {
        let domain = SecurityDomain::new();
        domain.enable().unwrap();

        domain
            .handle_method("setOverrideCertificateErrors", Some(json!({ "override": true })))
            .await
            .unwrap();

        let event_id = domain
            .report_certificate_error(
                "CERT_EXPIRED".to_string(),
                "https://expired.com".to_string(),
                "req-2".to_string(),
            )
            .unwrap();

        let params = json!({
            "eventId": event_id,
            "action": "cancel"
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

    #[tokio::test]
    async fn test_handle_certificate_error_invalid_event_id() {
        let domain = SecurityDomain::new();
        let params = json!({
            "eventId": 999999,
            "action": "continue"
        });

        let result = domain
            .handle_method("handleCertificateError", Some(params))
            .await;
        assert!(result.is_err());
    }

    // =========================================================================
    // Override Certificate Errors Tests
    // =========================================================================

    #[tokio::test]
    async fn test_set_override_certificate_errors() {
        let domain = SecurityDomain::new();

        let params = json!({ "override": true });
        let result = domain
            .handle_method("setOverrideCertificateErrors", Some(params))
            .await;
        assert!(result.is_ok());
        assert!(domain.is_override_mode_enabled());

        let params = json!({ "override": false });
        domain
            .handle_method("setOverrideCertificateErrors", Some(params))
            .await
            .unwrap();
        assert!(!domain.is_override_mode_enabled());
    }

    #[tokio::test]
    async fn test_set_override_certificate_errors_missing_param() {
        let domain = SecurityDomain::new();
        let result = domain
            .handle_method("setOverrideCertificateErrors", None)
            .await;
        assert!(result.is_err());
    }

    // =========================================================================
    // Security State Update Tests
    // =========================================================================

    #[test]
    fn test_update_security_state() {
        let domain = SecurityDomain::new();
        domain.enable().unwrap();

        domain.update_security_state(SecurityState::Secure);
        assert_eq!(domain.get_security_state(), SecurityState::Secure);

        domain.update_security_state(SecurityState::Insecure);
        assert_eq!(domain.get_security_state(), SecurityState::Insecure);

        // Events should be queued
        assert!(domain.has_pending_events());
    }

    #[test]
    fn test_update_certificate() {
        let domain = SecurityDomain::new();

        let mut cert = CertificateDetails::new("example.com".to_string(), "Let's Encrypt".to_string());
        cert.serial_number = "ABC123".to_string();
        cert.protocol = Some("TLS 1.3".to_string());

        domain.update_certificate(cert);

        let retrieved = domain.get_certificate_details().unwrap();
        assert_eq!(retrieved.subject_name, "example.com");
        assert_eq!(retrieved.issuer_name, "Let's Encrypt");
        assert_eq!(retrieved.protocol, Some("TLS 1.3".to_string()));
    }

    #[test]
    fn test_update_insecure_content() {
        let domain = SecurityDomain::new();
        domain.enable().unwrap();
        domain.update_security_state(SecurityState::Secure);

        let status = InsecureContentStatus {
            ran_insecure_content: true,
            displayed_insecure_content: false,
            contained_mixed_form: false,
            ran_content_with_cert_errors: false,
            displayed_content_with_cert_errors: false,
            insecure_origins: vec!["http://insecure.com".to_string()],
        };

        domain.update_insecure_content(status);
        assert_eq!(domain.get_security_state(), SecurityState::Insecure);

        let retrieved = domain.get_insecure_content_status();
        assert!(retrieved.ran_insecure_content);
    }

    // =========================================================================
    // Navigation Tests
    // =========================================================================

    #[test]
    fn test_on_navigation_https() {
        let domain = SecurityDomain::new();
        domain.enable().unwrap();

        domain.on_navigation("https://secure.example.com");
        assert_eq!(domain.get_security_state(), SecurityState::Secure);
    }

    #[test]
    fn test_on_navigation_http() {
        let domain = SecurityDomain::new();
        domain.enable().unwrap();

        domain.on_navigation("http://insecure.example.com");
        assert_eq!(domain.get_security_state(), SecurityState::Insecure);
    }

    #[test]
    fn test_on_navigation_localhost() {
        let domain = SecurityDomain::new();
        domain.enable().unwrap();

        domain.on_navigation("http://localhost:8080");
        assert_eq!(domain.get_security_state(), SecurityState::Neutral);

        domain.on_navigation("http://127.0.0.1:8080");
        assert_eq!(domain.get_security_state(), SecurityState::Neutral);
    }

    #[test]
    fn test_on_navigation_special_schemes() {
        let domain = SecurityDomain::new();
        domain.enable().unwrap();

        domain.on_navigation("file:///home/user/doc.html");
        assert_eq!(domain.get_security_state(), SecurityState::Neutral);

        domain.on_navigation("about:blank");
        assert_eq!(domain.get_security_state(), SecurityState::Neutral);

        domain.on_navigation("chrome://settings");
        assert_eq!(domain.get_security_state(), SecurityState::Neutral);

        domain.on_navigation("data:text/html,<h1>Test</h1>");
        assert_eq!(domain.get_security_state(), SecurityState::Neutral);
    }

    // =========================================================================
    // Event Tests
    // =========================================================================

    #[test]
    fn test_event_queue() {
        let domain = SecurityDomain::new();
        domain.enable().unwrap();

        // Enable should emit an event
        let events = domain.take_events();
        assert!(!events.is_empty());

        // After taking, queue should be empty
        assert!(!domain.has_pending_events());
    }

    #[test]
    fn test_certificate_error_event() {
        let domain = SecurityDomain::new();
        domain.enable().unwrap();

        // Clear initial events
        domain.take_events();

        // Enable override mode
        domain.state.write().override_certificate_errors = true;

        // Report an error
        let _event_id = domain.report_certificate_error(
            "CERT_AUTHORITY_INVALID".to_string(),
            "https://untrusted.com".to_string(),
            "req-1".to_string(),
        );

        let events = domain.take_events();
        assert!(!events.is_empty());

        let event = &events[0];
        assert_eq!(event["method"], "Security.certificateError");
        assert_eq!(event["params"]["errorType"], "CERT_AUTHORITY_INVALID");
    }

    #[test]
    fn test_no_event_when_disabled() {
        let domain = SecurityDomain::new();
        // Domain is not enabled

        domain.update_security_state(SecurityState::Secure);

        // No events should be queued when disabled
        assert!(!domain.has_pending_events());
    }

    // =========================================================================
    // Security State Explanation Tests
    // =========================================================================

    #[test]
    fn test_add_explanation() {
        let domain = SecurityDomain::new();
        domain.enable().unwrap();

        let explanation = SecurityStateExplanation::new(
            SecurityState::Insecure,
            "Certificate Error".to_string(),
            "The certificate is not trusted".to_string(),
            "The certificate was issued by an unknown authority".to_string(),
        );

        domain.add_explanation(explanation);

        // Verify via get_current_security_state
        let state = domain.get_current_security_state().unwrap();
        let explanations = state["explanations"].as_array().unwrap();
        assert_eq!(explanations.len(), 1);
        assert_eq!(explanations[0]["title"], "Certificate Error");
    }

    #[test]
    fn test_clear_explanations() {
        let domain = SecurityDomain::new();

        domain.add_explanation(SecurityStateExplanation::new(
            SecurityState::Insecure,
            "Test".to_string(),
            "Summary".to_string(),
            "Description".to_string(),
        ));

        domain.clear_explanations();

        let state = domain.get_current_security_state().unwrap();
        let explanations = state["explanations"].as_array().unwrap();
        assert!(explanations.is_empty());
    }

    // =========================================================================
    // Get Security State Tests
    // =========================================================================

    #[tokio::test]
    async fn test_get_security_state_method() {
        let domain = SecurityDomain::new();
        domain.update_security_state(SecurityState::Secure);

        let result = domain.handle_method("getSecurityState", None).await;
        assert!(result.is_ok());

        let state = result.unwrap();
        assert_eq!(state["securityState"], "secure");
        assert_eq!(state["schemeIsCryptographic"], true);
    }

    // =========================================================================
    // Unknown Method Test
    // =========================================================================

    #[tokio::test]
    async fn test_unknown_method() {
        let domain = SecurityDomain::new();
        let result = domain.handle_method("unknownMethod", None).await;

        assert!(result.is_err());
    }

    // =========================================================================
    // Report Certificate Error Tests
    // =========================================================================

    #[test]
    fn test_report_certificate_error_when_ignoring() {
        let domain = SecurityDomain::new();
        domain.state.write().ignore_certificate_errors = true;

        let result = domain.report_certificate_error(
            "CERT_EXPIRED".to_string(),
            "https://expired.com".to_string(),
            "req-1".to_string(),
        );

        assert!(result.is_none());
    }

    #[test]
    fn test_report_certificate_error_without_override_mode() {
        let domain = SecurityDomain::new();

        let result = domain.report_certificate_error(
            "CERT_EXPIRED".to_string(),
            "https://expired.com".to_string(),
            "req-1".to_string(),
        );

        assert!(result.is_none());
    }

    #[test]
    fn test_should_ignore_certificate_error() {
        let domain = SecurityDomain::new();
        assert!(!domain.should_ignore_certificate_error("CERT_EXPIRED"));

        domain.state.write().ignore_certificate_errors = true;
        assert!(domain.should_ignore_certificate_error("CERT_EXPIRED"));
    }

    // =========================================================================
    // Clone Tests
    // =========================================================================

    #[test]
    fn test_clone_shares_state() {
        let domain1 = SecurityDomain::new();
        let domain2 = domain1.clone();

        domain1.enable().unwrap();
        assert!(domain2.is_enabled());

        domain2.update_security_state(SecurityState::Secure);
        assert_eq!(domain1.get_security_state(), SecurityState::Secure);
    }
}
