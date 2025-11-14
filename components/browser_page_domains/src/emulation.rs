//! Emulation Domain Handler
//!
//! Implements the CDP Emulation domain for device emulation and override capabilities.

use async_trait::async_trait;
use cdp_types::CdpError;
use parking_lot::RwLock;
use protocol_handler::DomainHandler;
use serde_json::{json, Value};
use std::sync::Arc;

/// Emulation domain handler
///
/// Provides methods for device metrics emulation, user agent override, and geolocation override.
#[derive(Debug, Clone)]
pub struct EmulationDomain {
    state: Arc<RwLock<EmulationState>>,
}

#[derive(Debug, Default)]
struct EmulationState {
    device_metrics: Option<DeviceMetrics>,
    user_agent: Option<String>,
    geolocation: Option<Geolocation>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields used for state management in real implementation
struct DeviceMetrics {
    width: u32,
    height: u32,
    device_scale_factor: f64,
    mobile: bool,
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields used for state management in real implementation
struct Geolocation {
    latitude: f64,
    longitude: f64,
    accuracy: Option<f64>,
}

impl EmulationDomain {
    /// Create a new EmulationDomain instance
    ///
    /// # Example
    /// ```
    /// use browser_page_domains::EmulationDomain;
    ///
    /// let domain = EmulationDomain::new();
    /// ```
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(EmulationState::default())),
        }
    }

    /// Set device metrics override
    fn set_device_metrics_override(&self, params: Option<Value>) -> Result<Value, CdpError> {
        let params = params.ok_or_else(|| CdpError::invalid_params("Missing params"))?;

        let width = params["width"]
            .as_u64()
            .ok_or_else(|| CdpError::invalid_params("Missing 'width' parameter"))?
            as u32;

        let height = params["height"]
            .as_u64()
            .ok_or_else(|| CdpError::invalid_params("Missing 'height' parameter"))?
            as u32;

        let device_scale_factor = params["deviceScaleFactor"]
            .as_f64()
            .ok_or_else(|| CdpError::invalid_params("Missing 'deviceScaleFactor' parameter"))?;

        let mobile = params["mobile"]
            .as_bool()
            .ok_or_else(|| CdpError::invalid_params("Missing 'mobile' parameter"))?;

        self.state.write().device_metrics = Some(DeviceMetrics {
            width,
            height,
            device_scale_factor,
            mobile,
        });

        Ok(json!({}))
    }

    /// Clear device metrics override
    fn clear_device_metrics_override(&self) -> Result<Value, CdpError> {
        self.state.write().device_metrics = None;
        Ok(json!({}))
    }

    /// Set user agent override
    fn set_user_agent_override(&self, params: Option<Value>) -> Result<Value, CdpError> {
        let params = params.ok_or_else(|| CdpError::invalid_params("Missing params"))?;

        let user_agent = params["userAgent"]
            .as_str()
            .ok_or_else(|| CdpError::invalid_params("Missing 'userAgent' parameter"))?
            .to_string();

        self.state.write().user_agent = Some(user_agent);

        Ok(json!({}))
    }

    /// Set geolocation override
    fn set_geolocation_override(&self, params: Option<Value>) -> Result<Value, CdpError> {
        let params = params.ok_or_else(|| CdpError::invalid_params("Missing params"))?;

        let latitude = params["latitude"]
            .as_f64()
            .ok_or_else(|| CdpError::invalid_params("Missing 'latitude' parameter"))?;

        let longitude = params["longitude"]
            .as_f64()
            .ok_or_else(|| CdpError::invalid_params("Missing 'longitude' parameter"))?;

        let accuracy = params["accuracy"].as_f64();

        self.state.write().geolocation = Some(Geolocation {
            latitude,
            longitude,
            accuracy,
        });

        Ok(json!({}))
    }

    /// Clear geolocation override
    fn clear_geolocation_override(&self) -> Result<Value, CdpError> {
        self.state.write().geolocation = None;
        Ok(json!({}))
    }
}

impl Default for EmulationDomain {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DomainHandler for EmulationDomain {
    fn name(&self) -> &str {
        "Emulation"
    }

    async fn handle_method(&self, method: &str, params: Option<Value>) -> Result<Value, CdpError> {
        match method {
            "setDeviceMetricsOverride" => self.set_device_metrics_override(params),
            "clearDeviceMetricsOverride" => self.clear_device_metrics_override(),
            "setUserAgentOverride" => self.set_user_agent_override(params),
            "setGeolocationOverride" => self.set_geolocation_override(params),
            "clearGeolocationOverride" => self.clear_geolocation_override(),
            _ => Err(CdpError::method_not_found(format!("Emulation.{}", method))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let domain = EmulationDomain::new();
        assert!(domain.state.read().device_metrics.is_none());
        assert!(domain.state.read().user_agent.is_none());
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

        let state = domain.state.read();
        assert!(state.device_metrics.is_some());
        let metrics = state.device_metrics.as_ref().unwrap();
        assert_eq!(metrics.width, 1920);
        assert_eq!(metrics.height, 1080);
    }

    #[tokio::test]
    async fn test_clear_device_metrics_override() {
        let domain = EmulationDomain::new();

        // Set first
        let params = json!({
            "width": 1920,
            "height": 1080,
            "deviceScaleFactor": 1.0,
            "mobile": false
        });
        domain
            .handle_method("setDeviceMetricsOverride", Some(params))
            .await
            .unwrap();

        // Then clear
        let result = domain
            .handle_method("clearDeviceMetricsOverride", None)
            .await;
        assert!(result.is_ok());
        assert!(domain.state.read().device_metrics.is_none());
    }

    #[tokio::test]
    async fn test_set_user_agent_override() {
        let domain = EmulationDomain::new();
        let params = json!({
            "userAgent": "Custom UA"
        });

        let result = domain
            .handle_method("setUserAgentOverride", Some(params))
            .await;
        assert!(result.is_ok());

        let state = domain.state.read();
        assert_eq!(state.user_agent.as_deref(), Some("Custom UA"));
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

        let state = domain.state.read();
        assert!(state.geolocation.is_some());
        let geo = state.geolocation.as_ref().unwrap();
        assert_eq!(geo.latitude, 37.7749);
    }
}
