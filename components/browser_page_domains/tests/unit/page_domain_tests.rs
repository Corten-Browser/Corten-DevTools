// Unit tests for PageDomain

use browser_page_domains::PageDomain;
use protocol_handler::DomainHandler;
use serde_json::json;

#[tokio::test]
async fn test_page_domain_name() {
    let domain = PageDomain::new();
    assert_eq!(domain.name(), "Page");
}

#[tokio::test]
async fn test_page_enable() {
    let domain = PageDomain::new();
    let result = domain.handle_method("enable", None).await;

    assert!(result.is_ok());
    assert!(domain.is_enabled());
}

#[tokio::test]
async fn test_page_disable() {
    let mut domain = PageDomain::new();
    domain.handle_method("enable", None).await.unwrap();

    let result = domain.handle_method("disable", None).await;
    assert!(result.is_ok());
    assert!(!domain.is_enabled());
}

#[tokio::test]
async fn test_navigate() {
    let domain = PageDomain::new();
    let params = json!({
        "url": "https://example.com"
    });

    let result = domain.handle_method("navigate", Some(params)).await;
    assert!(result.is_ok());

    let value = result.unwrap();
    // Should return frameId
    assert!(value["frameId"].is_string());
}

#[tokio::test]
async fn test_navigate_missing_url() {
    let domain = PageDomain::new();
    let result = domain.handle_method("navigate", None).await;

    // Should fail with invalid params
    assert!(result.is_err());
}

#[tokio::test]
async fn test_reload() {
    let domain = PageDomain::new();
    let result = domain.handle_method("reload", None).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_reload_with_ignore_cache() {
    let domain = PageDomain::new();
    let params = json!({
        "ignoreCache": true
    });

    let result = domain.handle_method("reload", Some(params)).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_get_frame_tree() {
    let domain = PageDomain::new();
    let result = domain.handle_method("getFrameTree", None).await;

    assert!(result.is_ok());
    let value = result.unwrap();

    // Should have a frameTree
    assert!(value["frameTree"].is_object());
    assert!(value["frameTree"]["frame"].is_object());
}

#[tokio::test]
async fn test_capture_screenshot() {
    let domain = PageDomain::new();
    let result = domain.handle_method("captureScreenshot", None).await;

    assert!(result.is_ok());
    let value = result.unwrap();

    // Should return base64 encoded data
    assert!(value["data"].is_string());
}

#[tokio::test]
async fn test_capture_screenshot_with_format() {
    let domain = PageDomain::new();
    let params = json!({
        "format": "jpeg",
        "quality": 80
    });

    let result = domain
        .handle_method("captureScreenshot", Some(params))
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_unknown_method() {
    let domain = PageDomain::new();
    let result = domain.handle_method("unknownMethod", None).await;

    assert!(result.is_err());
}
