// Unit tests for SecurityDomain
//
// FEAT-013: Security Domain - CDP Security domain for certificate and security state

use browser_page_domains::{
    CertificateDetails, InsecureContentStatus, SecurityDomain, SecurityState,
    SecurityStateExplanation,
};
use protocol_handler::DomainHandler;
use serde_json::json;

// =============================================================================
// Domain Handler Interface Tests
// =============================================================================

#[tokio::test]
async fn test_security_domain_name() {
    let domain = SecurityDomain::new();
    assert_eq!(domain.name(), "Security");
}

#[tokio::test]
async fn test_security_enable() {
    let domain = SecurityDomain::new();
    let result = domain.handle_method("enable", None).await;

    assert!(result.is_ok());
    assert!(domain.is_enabled());
}

#[tokio::test]
async fn test_security_disable() {
    let domain = SecurityDomain::new();
    domain.handle_method("enable", None).await.unwrap();

    let result = domain.handle_method("disable", None).await;
    assert!(result.is_ok());
    assert!(!domain.is_enabled());
}

// =============================================================================
// setIgnoreCertificateErrors Tests
// =============================================================================

#[tokio::test]
async fn test_set_ignore_certificate_errors() {
    let domain = SecurityDomain::new();
    let params = json!({
        "ignore": true
    });

    let result = domain
        .handle_method("setIgnoreCertificateErrors", Some(params))
        .await;
    assert!(result.is_ok());
    assert!(domain.ignores_certificate_errors());
}

#[tokio::test]
async fn test_set_ignore_certificate_errors_false() {
    let domain = SecurityDomain::new();

    // First enable ignoring
    domain
        .handle_method("setIgnoreCertificateErrors", Some(json!({"ignore": true})))
        .await
        .unwrap();
    assert!(domain.ignores_certificate_errors());

    // Then disable
    domain
        .handle_method(
            "setIgnoreCertificateErrors",
            Some(json!({"ignore": false})),
        )
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

    // Should fail with invalid params
    assert!(result.is_err());
}

#[tokio::test]
async fn test_set_ignore_certificate_errors_wrong_type() {
    let domain = SecurityDomain::new();
    let params = json!({
        "ignore": "not a boolean"
    });

    let result = domain
        .handle_method("setIgnoreCertificateErrors", Some(params))
        .await;
    assert!(result.is_err());
}

// =============================================================================
// handleCertificateError Tests
// =============================================================================

#[tokio::test]
async fn test_handle_certificate_error() {
    let domain = SecurityDomain::new();
    domain.handle_method("enable", None).await.unwrap();

    // Enable override mode
    domain
        .handle_method(
            "setOverrideCertificateErrors",
            Some(json!({"override": true})),
        )
        .await
        .unwrap();

    // Report an error to get a valid event ID
    let event_id = domain
        .report_certificate_error(
            "CERT_AUTHORITY_INVALID".to_string(),
            "https://example.com".to_string(),
            "req-1".to_string(),
        )
        .unwrap();

    let params = json!({
        "eventId": event_id,
        "action": "continue"
    });

    let result = domain
        .handle_method("handleCertificateError", Some(params))
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_handle_certificate_error_cancel() {
    let domain = SecurityDomain::new();
    domain.handle_method("enable", None).await.unwrap();

    domain
        .handle_method(
            "setOverrideCertificateErrors",
            Some(json!({"override": true})),
        )
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

#[tokio::test]
async fn test_handle_certificate_error_missing_params() {
    let domain = SecurityDomain::new();

    let result = domain.handle_method("handleCertificateError", None).await;
    assert!(result.is_err());

    let result = domain
        .handle_method("handleCertificateError", Some(json!({})))
        .await;
    assert!(result.is_err());
}

// =============================================================================
// setOverrideCertificateErrors Tests
// =============================================================================

#[tokio::test]
async fn test_set_override_certificate_errors() {
    let domain = SecurityDomain::new();

    let params = json!({ "override": true });
    let result = domain
        .handle_method("setOverrideCertificateErrors", Some(params))
        .await;
    assert!(result.is_ok());
    assert!(domain.is_override_mode_enabled());
}

#[tokio::test]
async fn test_set_override_certificate_errors_false() {
    let domain = SecurityDomain::new();

    domain
        .handle_method(
            "setOverrideCertificateErrors",
            Some(json!({"override": true})),
        )
        .await
        .unwrap();
    assert!(domain.is_override_mode_enabled());

    domain
        .handle_method(
            "setOverrideCertificateErrors",
            Some(json!({"override": false})),
        )
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

// =============================================================================
// getSecurityState Tests
// =============================================================================

#[tokio::test]
async fn test_get_security_state() {
    let domain = SecurityDomain::new();
    domain.update_security_state(SecurityState::Secure);

    let result = domain.handle_method("getSecurityState", None).await;
    assert!(result.is_ok());

    let state = result.unwrap();
    assert_eq!(state["securityState"], "secure");
    assert_eq!(state["schemeIsCryptographic"], true);
}

#[tokio::test]
async fn test_get_security_state_insecure() {
    let domain = SecurityDomain::new();
    domain.update_security_state(SecurityState::Insecure);

    let result = domain.handle_method("getSecurityState", None).await;
    assert!(result.is_ok());

    let state = result.unwrap();
    assert_eq!(state["securityState"], "insecure");
    assert_eq!(state["schemeIsCryptographic"], false);
}

#[tokio::test]
async fn test_get_security_state_with_explanations() {
    let domain = SecurityDomain::new();
    domain.update_security_state(SecurityState::Insecure);

    domain.add_explanation(SecurityStateExplanation::new(
        SecurityState::Insecure,
        "Certificate Error".to_string(),
        "Invalid certificate".to_string(),
        "The certificate is not trusted by this browser".to_string(),
    ));

    let result = domain.handle_method("getSecurityState", None).await;
    assert!(result.is_ok());

    let state = result.unwrap();
    let explanations = state["explanations"].as_array().unwrap();
    assert_eq!(explanations.len(), 1);
    assert_eq!(explanations[0]["title"], "Certificate Error");
}

#[tokio::test]
async fn test_get_security_state_with_certificate() {
    let domain = SecurityDomain::new();

    let cert = CertificateDetails::new("example.com".to_string(), "Let's Encrypt".to_string());
    domain.update_certificate(cert);

    let result = domain.handle_method("getSecurityState", None).await;
    assert!(result.is_ok());

    let state = result.unwrap();
    assert!(state["certificate"].is_object());
    assert_eq!(state["certificate"]["subjectName"], "example.com");
}

// =============================================================================
// Security State Tracking Tests
// =============================================================================

#[test]
fn test_security_state_enum() {
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

#[tokio::test]
async fn test_update_security_state() {
    let domain = SecurityDomain::new();
    domain.handle_method("enable", None).await.unwrap();

    domain.update_security_state(SecurityState::Secure);
    assert_eq!(domain.get_security_state(), SecurityState::Secure);

    domain.update_security_state(SecurityState::Insecure);
    assert_eq!(domain.get_security_state(), SecurityState::Insecure);
}

// =============================================================================
// Certificate Details Tests
// =============================================================================

#[test]
fn test_certificate_details_new() {
    let cert = CertificateDetails::new("example.com".to_string(), "CA".to_string());
    assert_eq!(cert.subject_name, "example.com");
    assert_eq!(cert.issuer_name, "CA");
    assert!(cert.san_list.is_empty());
}

#[test]
fn test_update_certificate() {
    let domain = SecurityDomain::new();

    let mut cert = CertificateDetails::new("secure.example.com".to_string(), "DigiCert".to_string());
    cert.protocol = Some("TLS 1.3".to_string());
    cert.cipher = Some("AES_256_GCM".to_string());

    domain.update_certificate(cert);

    let retrieved = domain.get_certificate_details().unwrap();
    assert_eq!(retrieved.subject_name, "secure.example.com");
    assert_eq!(retrieved.issuer_name, "DigiCert");
    assert_eq!(retrieved.protocol, Some("TLS 1.3".to_string()));
}

// =============================================================================
// Insecure Content Status Tests
// =============================================================================

#[test]
fn test_insecure_content_status_default() {
    let status = InsecureContentStatus::default();
    assert!(!status.ran_insecure_content);
    assert!(!status.displayed_insecure_content);
    assert!(!status.contained_mixed_form);
    assert!(!status.ran_content_with_cert_errors);
    assert!(!status.displayed_content_with_cert_errors);
    assert!(status.insecure_origins.is_empty());
}

#[test]
fn test_update_insecure_content_downgrades_state() {
    let domain = SecurityDomain::new();
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
}

#[test]
fn test_update_insecure_content_displayed_only() {
    let domain = SecurityDomain::new();
    domain.update_security_state(SecurityState::Secure);

    let status = InsecureContentStatus {
        ran_insecure_content: false,
        displayed_insecure_content: true,
        contained_mixed_form: false,
        ran_content_with_cert_errors: false,
        displayed_content_with_cert_errors: false,
        insecure_origins: vec![],
    };

    domain.update_insecure_content(status);
    // Displayed content only downgrades to neutral, not insecure
    assert_eq!(domain.get_security_state(), SecurityState::Neutral);
}

// =============================================================================
// Navigation Tests
// =============================================================================

#[test]
fn test_on_navigation_https() {
    let domain = SecurityDomain::new();
    domain.on_navigation("https://secure.example.com/path");
    assert_eq!(domain.get_security_state(), SecurityState::Secure);
}

#[test]
fn test_on_navigation_http() {
    let domain = SecurityDomain::new();
    domain.on_navigation("http://insecure.example.com");
    assert_eq!(domain.get_security_state(), SecurityState::Insecure);
}

#[test]
fn test_on_navigation_localhost() {
    let domain = SecurityDomain::new();

    domain.on_navigation("http://localhost:3000");
    assert_eq!(domain.get_security_state(), SecurityState::Neutral);

    domain.on_navigation("http://127.0.0.1:8080/api");
    assert_eq!(domain.get_security_state(), SecurityState::Neutral);
}

#[test]
fn test_on_navigation_special_schemes() {
    let domain = SecurityDomain::new();

    domain.on_navigation("file:///home/user/document.html");
    assert_eq!(domain.get_security_state(), SecurityState::Neutral);

    domain.on_navigation("about:blank");
    assert_eq!(domain.get_security_state(), SecurityState::Neutral);

    domain.on_navigation("chrome://settings");
    assert_eq!(domain.get_security_state(), SecurityState::Neutral);

    domain.on_navigation("data:text/html,<h1>Test</h1>");
    assert_eq!(domain.get_security_state(), SecurityState::Neutral);
}

#[test]
fn test_on_navigation_unknown_scheme() {
    let domain = SecurityDomain::new();
    domain.on_navigation("custom://something");
    assert_eq!(domain.get_security_state(), SecurityState::Unknown);
}

// =============================================================================
// Event Emission Tests
// =============================================================================

#[tokio::test]
async fn test_events_queued_when_enabled() {
    let domain = SecurityDomain::new();
    domain.handle_method("enable", None).await.unwrap();

    // Enable emits initial state
    let events = domain.take_events();
    assert!(!events.is_empty());

    // State change emits event
    domain.update_security_state(SecurityState::Secure);
    let events = domain.take_events();
    assert!(!events.is_empty());

    let event = &events[0];
    assert_eq!(event["method"], "Security.securityStateChanged");
}

#[test]
fn test_no_events_when_disabled() {
    let domain = SecurityDomain::new();
    // Domain not enabled

    domain.update_security_state(SecurityState::Secure);
    assert!(!domain.has_pending_events());
}

#[tokio::test]
async fn test_certificate_error_event_emitted() {
    let domain = SecurityDomain::new();
    domain.handle_method("enable", None).await.unwrap();
    domain.take_events(); // Clear initial events

    // Enable override mode
    domain
        .handle_method(
            "setOverrideCertificateErrors",
            Some(json!({"override": true})),
        )
        .await
        .unwrap();

    // Report error
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
    assert_eq!(event["params"]["requestUrl"], "https://untrusted.com");
}

// =============================================================================
// Certificate Error Reporting Tests
// =============================================================================

#[tokio::test]
async fn test_report_certificate_error_when_ignoring() {
    let domain = SecurityDomain::new();
    domain
        .handle_method("setIgnoreCertificateErrors", Some(json!({"ignore": true})))
        .await
        .unwrap();

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

#[tokio::test]
async fn test_should_ignore_certificate_error() {
    let domain = SecurityDomain::new();
    assert!(!domain.should_ignore_certificate_error("CERT_EXPIRED"));

    domain
        .handle_method("setIgnoreCertificateErrors", Some(json!({"ignore": true})))
        .await
        .unwrap();
    assert!(domain.should_ignore_certificate_error("CERT_EXPIRED"));
}

// =============================================================================
// Unknown Method Test
// =============================================================================

#[tokio::test]
async fn test_unknown_method() {
    let domain = SecurityDomain::new();
    let result = domain.handle_method("unknownMethod", None).await;

    assert!(result.is_err());
}

// =============================================================================
// Clone and State Sharing Tests
// =============================================================================

#[tokio::test]
async fn test_clone_shares_state() {
    let domain1 = SecurityDomain::new();
    let domain2 = domain1.clone();

    domain1.handle_method("enable", None).await.unwrap();
    assert!(domain2.is_enabled());

    domain2.update_security_state(SecurityState::Secure);
    assert_eq!(domain1.get_security_state(), SecurityState::Secure);
}

// =============================================================================
// Integration Tests
// =============================================================================

#[tokio::test]
async fn test_full_certificate_error_flow() {
    let domain = SecurityDomain::new();

    // 1. Enable the domain
    domain.handle_method("enable", None).await.unwrap();
    assert!(domain.is_enabled());

    // 2. Enable override mode for certificate errors
    domain
        .handle_method(
            "setOverrideCertificateErrors",
            Some(json!({"override": true})),
        )
        .await
        .unwrap();

    // Clear events so far
    domain.take_events();

    // 3. Report a certificate error (simulating browser encountering one)
    let event_id = domain
        .report_certificate_error(
            "NET::ERR_CERT_AUTHORITY_INVALID".to_string(),
            "https://self-signed.badssl.com".to_string(),
            "request-123".to_string(),
        )
        .expect("Should return event ID when override mode is enabled");

    // 4. Verify event was queued
    let events = domain.take_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["method"], "Security.certificateError");
    assert_eq!(events[0]["params"]["eventId"], event_id);

    // 5. Handle the certificate error (user decision to continue)
    let result = domain
        .handle_method(
            "handleCertificateError",
            Some(json!({
                "eventId": event_id,
                "action": "continue"
            })),
        )
        .await;
    assert!(result.is_ok());

    // 6. Verify pending error count is 0
    assert_eq!(domain.pending_certificate_error_count(), 0);
}

#[tokio::test]
async fn test_navigation_and_security_state_flow() {
    let domain = SecurityDomain::new();
    domain.handle_method("enable", None).await.unwrap();
    domain.take_events(); // Clear initial events

    // Navigate to HTTPS page
    domain.on_navigation("https://secure.example.com");
    assert_eq!(domain.get_security_state(), SecurityState::Secure);

    let events = domain.take_events();
    assert!(!events.is_empty());
    assert_eq!(events[0]["params"]["securityState"], "secure");

    // Update with insecure content
    domain.update_insecure_content(InsecureContentStatus {
        ran_insecure_content: true,
        displayed_insecure_content: false,
        contained_mixed_form: false,
        ran_content_with_cert_errors: false,
        displayed_content_with_cert_errors: false,
        insecure_origins: vec!["http://tracker.com".to_string()],
    });

    assert_eq!(domain.get_security_state(), SecurityState::Insecure);

    let events = domain.take_events();
    assert!(!events.is_empty());
    assert_eq!(events[0]["params"]["securityState"], "insecure");
}
