// Network domain types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unique request identifier
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct RequestId(pub String);

/// Timestamp (seconds since epoch with millisecond precision)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Timestamp(pub f64);

/// Resource type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ResourceType {
    Document,
    Stylesheet,
    Image,
    Media,
    Font,
    Script,
    TextTrack,
    XHR,
    Fetch,
    EventSource,
    WebSocket,
    Manifest,
    SignedExchange,
    Ping,
    CSPViolationReport,
    Other,
}

/// Resource priority
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ResourcePriority {
    VeryLow,
    Low,
    Medium,
    High,
    VeryHigh,
}

/// Referrer policy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ReferrerPolicy {
    UnsafeUrl,
    NoReferrerWhenDowngrade,
    NoReferrer,
    Origin,
    OriginWhenCrossOrigin,
    SameOrigin,
    StrictOrigin,
    StrictOriginWhenCrossOrigin,
}

/// Security state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SecurityState {
    Unknown,
    Neutral,
    Insecure,
    Secure,
    Info,
    InsecureBroken,
}

/// HTTP request
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    /// Request URL
    pub url: String,
    /// HTTP method
    pub method: String,
    /// HTTP request headers
    pub headers: HashMap<String, String>,
    /// POST data (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_data: Option<String>,
    /// Whether request has POST data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_post_data: Option<bool>,
    /// Mixed content type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mixed_content_type: Option<String>,
    /// Request priority
    pub initial_priority: ResourcePriority,
    /// Referrer policy
    pub referrer_policy: ReferrerPolicy,
}

/// HTTP response
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    /// Response URL
    pub url: String,
    /// HTTP status code
    pub status: u32,
    /// HTTP status text
    pub status_text: String,
    /// HTTP response headers
    pub headers: HashMap<String, String>,
    /// MIME type
    pub mime_type: String,
    /// Request headers (if captured)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_headers: Option<HashMap<String, String>>,
    /// Whether connection was reused
    pub connection_reused: bool,
    /// Connection ID
    pub connection_id: u64,
    /// Whether response came from disk cache
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_disk_cache: Option<bool>,
    /// Whether response came from service worker
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_service_worker: Option<bool>,
    /// Encoded data length
    pub encoded_data_length: f64,
    /// Timing information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timing: Option<ResourceTiming>,
    /// Protocol used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,
    /// Security state
    pub security_state: SecurityState,
}

/// Resource timing information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResourceTiming {
    /// Request start time
    pub request_time: f64,
    /// Proxy negotiation start
    pub proxy_start: f64,
    /// Proxy negotiation end
    pub proxy_end: f64,
    /// DNS lookup start
    pub dns_start: f64,
    /// DNS lookup end
    pub dns_end: f64,
    /// Connection start
    pub connect_start: f64,
    /// Connection end
    pub connect_end: f64,
    /// SSL handshake start
    pub ssl_start: f64,
    /// SSL handshake end
    pub ssl_end: f64,
    /// Send start
    pub send_start: f64,
    /// Send end
    pub send_end: f64,
    /// Push notification start
    pub push_start: f64,
    /// Push notification end
    pub push_end: f64,
    /// Response headers received
    pub receive_headers_end: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_id() {
        let id = RequestId("req-123".to_string());
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"req-123\"");
    }

    #[test]
    fn test_timestamp() {
        let ts = Timestamp(1234567890.123);
        let json = serde_json::to_string(&ts).unwrap();
        assert!(json.contains("1234567890.123"));
    }

    #[test]
    fn test_resource_type() {
        let rt = ResourceType::Document;
        let json = serde_json::to_string(&rt).unwrap();
        assert_eq!(json, "\"Document\"");
    }
}
