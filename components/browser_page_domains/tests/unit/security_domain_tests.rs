// Unit tests for SecurityDomain

use browser_page_domains::SecurityDomain;
use protocol_handler::DomainHandler;
use serde_json::json;

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
    let mut domain = SecurityDomain::new();
    domain.handle_method("enable", None).await.unwrap();

    let result = domain.handle_method("disable", None).await;
    assert!(result.is_ok());
    assert!(!domain.is_enabled());
}

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
async fn test_set_ignore_certificate_errors_missing_param() {
    let domain = SecurityDomain::new();
    let result = domain
        .handle_method("setIgnoreCertificateErrors", None)
        .await;

    // Should fail with invalid params
    assert!(result.is_err());
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
async fn test_handle_certificate_error_cancel() {
    let domain = SecurityDomain::new();
    let params = json!({
        "eventId": 456,
        "action": "cancel"
    });

    let result = domain
        .handle_method("handleCertificateError", Some(params))
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_unknown_method() {
    let domain = SecurityDomain::new();
    let result = domain.handle_method("unknownMethod", None).await;

    assert!(result.is_err());
}
