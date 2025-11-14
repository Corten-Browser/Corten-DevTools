// Unit tests for BrowserDomain

use browser_page_domains::BrowserDomain;
use protocol_handler::DomainHandler;
use serde_json::json;

#[tokio::test]
async fn test_browser_domain_name() {
    let domain = BrowserDomain::new();
    assert_eq!(domain.name(), "Browser");
}

#[tokio::test]
async fn test_get_version() {
    let domain = BrowserDomain::new();
    let result = domain.handle_method("getVersion", None).await;

    assert!(result.is_ok());
    let value = result.unwrap();

    // Should contain required fields
    assert!(value["protocolVersion"].is_string());
    assert!(value["product"].is_string());
    assert!(value["revision"].is_string());
    assert!(value["userAgent"].is_string());
    assert!(value["jsVersion"].is_string());
}

#[tokio::test]
async fn test_get_browser_command_line() {
    let domain = BrowserDomain::new();
    let result = domain.handle_method("getBrowserCommandLine", None).await;

    assert!(result.is_ok());
    let value = result.unwrap();

    // Should contain arguments array
    assert!(value["arguments"].is_array());
}

#[tokio::test]
async fn test_close_browser() {
    let domain = BrowserDomain::new();
    let result = domain.handle_method("close", None).await;

    // Should succeed (even if it's a no-op in mock)
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_unknown_method() {
    let domain = BrowserDomain::new();
    let result = domain.handle_method("unknownMethod", None).await;

    // Should return method not found error
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_version_serialization() {
    let domain = BrowserDomain::new();
    let result = domain.handle_method("getVersion", None).await;

    assert!(result.is_ok());
    let value = result.unwrap();

    // Should be serializable JSON
    let json_str = serde_json::to_string(&value).unwrap();
    assert!(json_str.contains("protocolVersion"));
    assert!(json_str.contains("product"));
}
