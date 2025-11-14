// Unit tests for EmulationDomain

use browser_page_domains::EmulationDomain;
use protocol_handler::DomainHandler;
use serde_json::json;

#[tokio::test]
async fn test_emulation_domain_name() {
    let domain = EmulationDomain::new();
    assert_eq!(domain.name(), "Emulation");
}

#[tokio::test]
async fn test_set_device_metrics_override() {
    let domain = EmulationDomain::new();
    let params = json!({
        "width": 1920,
        "height": 1080,
        "deviceScaleFactor": 1.0,
        "mobile": false
    });

    let result = domain
        .handle_method("setDeviceMetricsOverride", Some(params))
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_set_device_metrics_override_mobile() {
    let domain = EmulationDomain::new();
    let params = json!({
        "width": 375,
        "height": 667,
        "deviceScaleFactor": 2.0,
        "mobile": true,
        "screenOrientation": {
            "type": "portraitPrimary",
            "angle": 0
        }
    });

    let result = domain
        .handle_method("setDeviceMetricsOverride", Some(params))
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_set_device_metrics_override_missing_params() {
    let domain = EmulationDomain::new();
    let result = domain.handle_method("setDeviceMetricsOverride", None).await;

    // Should fail with invalid params
    assert!(result.is_err());
}

#[tokio::test]
async fn test_clear_device_metrics_override() {
    let domain = EmulationDomain::new();
    let result = domain
        .handle_method("clearDeviceMetricsOverride", None)
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_set_user_agent_override() {
    let domain = EmulationDomain::new();
    let params = json!({
        "userAgent": "Mozilla/5.0 (Custom Browser)"
    });

    let result = domain
        .handle_method("setUserAgentOverride", Some(params))
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_set_user_agent_override_with_metadata() {
    let domain = EmulationDomain::new();
    let params = json!({
        "userAgent": "Mozilla/5.0",
        "acceptLanguage": "en-US",
        "platform": "Linux"
    });

    let result = domain
        .handle_method("setUserAgentOverride", Some(params))
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_set_user_agent_override_missing_param() {
    let domain = EmulationDomain::new();
    let result = domain.handle_method("setUserAgentOverride", None).await;

    // Should fail with invalid params
    assert!(result.is_err());
}

#[tokio::test]
async fn test_set_geolocation_override() {
    let domain = EmulationDomain::new();
    let params = json!({
        "latitude": 37.7749,
        "longitude": -122.4194,
        "accuracy": 100.0
    });

    let result = domain
        .handle_method("setGeolocationOverride", Some(params))
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_clear_geolocation_override() {
    let domain = EmulationDomain::new();
    let result = domain.handle_method("clearGeolocationOverride", None).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_unknown_method() {
    let domain = EmulationDomain::new();
    let result = domain.handle_method("unknownMethod", None).await;

    assert!(result.is_err());
}
