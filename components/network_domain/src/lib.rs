//! Network monitoring and interception
//!
//! This module implements the Chrome DevTools Protocol (CDP) Network domain,
//! providing request/response monitoring, interception, body retrieval capabilities,
//! cache inspection, and WebSocket frame inspection.
//!
//! # Features
//! - **Network Inspector Bridge**: Full request/response inspection with body capture
//! - **Cache Inspection**: Cache.requestCacheNames, requestEntries, deleteCache, deleteEntry
//! - **WebSocket Frame Inspection**: Track WebSocket connections and frame traffic

use async_trait::async_trait;
use cdp_types::CdpError;
use dashmap::DashMap;
use parking_lot::RwLock;
use protocol_handler::DomainHandler;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn};

/// Default maximum response body size (10MB)
pub const DEFAULT_MAX_RESPONSE_BODY_SIZE: usize = 10 * 1024 * 1024;

/// Default maximum request body size (5MB)
pub const DEFAULT_MAX_REQUEST_BODY_SIZE: usize = 5 * 1024 * 1024;

/// HTTP Headers map type
pub type HttpHeaders = std::collections::HashMap<String, String>;

/// Information about a tracked network request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestInfo {
    /// Unique request identifier
    pub request_id: String,
    /// Request URL
    pub url: String,
    /// HTTP method (GET, POST, etc.)
    pub method: String,
    /// Request headers
    #[serde(default)]
    pub request_headers: HttpHeaders,
    /// Request body (if available)
    pub request_body: Option<String>,
    /// Whether the request body is base64 encoded
    pub request_body_base64: bool,
    /// Response status code
    pub status_code: Option<u16>,
    /// Response headers
    #[serde(default)]
    pub response_headers: HttpHeaders,
    /// Response body (if available)
    pub response_body: Option<String>,
    /// Whether the response body is base64 encoded
    pub is_base64: bool,
    /// Response body size in bytes
    pub response_size: Option<usize>,
    /// Timestamp when request started (ms since epoch)
    pub timestamp: f64,
    /// Time taken for response (ms)
    pub response_time: Option<f64>,
    /// Resource type (Document, Script, Image, etc.)
    pub resource_type: Option<String>,
    /// Whether the response was from cache
    pub from_cache: bool,
}

impl RequestInfo {
    /// Create a new RequestInfo with basic fields
    pub fn new(request_id: String, url: String, method: String) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs_f64() * 1000.0)
            .unwrap_or(0.0);

        Self {
            request_id,
            url,
            method,
            request_headers: HttpHeaders::new(),
            request_body: None,
            request_body_base64: false,
            status_code: None,
            response_headers: HttpHeaders::new(),
            response_body: None,
            is_base64: false,
            response_size: None,
            timestamp,
            response_time: None,
            resource_type: None,
            from_cache: false,
        }
    }
}

// =============================================================================
// Cache Inspection Types
// =============================================================================

/// Cache entry information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheEntry {
    /// Request URL
    pub request_url: String,
    /// Request method
    pub request_method: String,
    /// Request headers
    #[serde(default)]
    pub request_headers: Vec<CacheHeader>,
    /// Response time
    pub response_time: f64,
    /// Response status
    pub response_status: u16,
    /// Response status text
    pub response_status_text: String,
    /// Response type
    pub response_type: String,
    /// Response headers
    #[serde(default)]
    pub response_headers: Vec<CacheHeader>,
}

/// Cache header representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheHeader {
    /// Header name
    pub name: String,
    /// Header value
    pub value: String,
}

/// Cache information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheInfo {
    /// Cache name/ID
    pub cache_id: String,
    /// Security origin
    pub security_origin: String,
    /// Cache name
    pub cache_name: String,
}

// =============================================================================
// WebSocket Frame Inspection Types
// =============================================================================

/// WebSocket connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WebSocketState {
    /// Connection is being established
    Connecting,
    /// Connection is open and ready
    Open,
    /// Connection is closing
    Closing,
    /// Connection is closed
    Closed,
}

/// WebSocket frame opcode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WebSocketOpcode {
    /// Continuation frame
    Continuation,
    /// Text frame
    Text,
    /// Binary frame
    Binary,
    /// Connection close
    Close,
    /// Ping frame
    Ping,
    /// Pong frame
    Pong,
}

impl WebSocketOpcode {
    /// Convert from raw opcode value
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => WebSocketOpcode::Continuation,
            1 => WebSocketOpcode::Text,
            2 => WebSocketOpcode::Binary,
            8 => WebSocketOpcode::Close,
            9 => WebSocketOpcode::Ping,
            10 => WebSocketOpcode::Pong,
            _ => WebSocketOpcode::Binary, // Default to binary for unknown
        }
    }
}

/// WebSocket frame information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebSocketFrame {
    /// Frame opcode
    pub opcode: WebSocketOpcode,
    /// Whether this is a masked frame
    pub mask: bool,
    /// Frame payload data (text or base64 encoded binary)
    pub payload_data: String,
    /// Whether payload_data is base64 encoded
    pub is_base64: bool,
    /// Timestamp when frame was received/sent (ms since epoch)
    pub timestamp: f64,
    /// Whether this frame was sent (true) or received (false)
    pub is_outgoing: bool,
}

impl WebSocketFrame {
    /// Create a new text frame
    pub fn text(data: String, is_outgoing: bool) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs_f64() * 1000.0)
            .unwrap_or(0.0);

        Self {
            opcode: WebSocketOpcode::Text,
            mask: is_outgoing,
            payload_data: data,
            is_base64: false,
            timestamp,
            is_outgoing,
        }
    }

    /// Create a new binary frame
    pub fn binary(data: String, is_outgoing: bool) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs_f64() * 1000.0)
            .unwrap_or(0.0);

        Self {
            opcode: WebSocketOpcode::Binary,
            mask: is_outgoing,
            payload_data: data,
            is_base64: true,
            timestamp,
            is_outgoing,
        }
    }
}

/// WebSocket connection information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebSocketConnection {
    /// Unique connection identifier
    pub request_id: String,
    /// WebSocket URL
    pub url: String,
    /// Connection state
    pub state: WebSocketState,
    /// Request headers used during handshake
    #[serde(default)]
    pub request_headers: HttpHeaders,
    /// Response headers from handshake
    #[serde(default)]
    pub response_headers: HttpHeaders,
    /// Connection timestamp (ms since epoch)
    pub timestamp: f64,
    /// Frames sent/received on this connection
    #[serde(default)]
    pub frames: Vec<WebSocketFrame>,
}

impl WebSocketConnection {
    /// Create a new WebSocket connection
    pub fn new(request_id: String, url: String) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs_f64() * 1000.0)
            .unwrap_or(0.0);

        Self {
            request_id,
            url,
            state: WebSocketState::Connecting,
            request_headers: HttpHeaders::new(),
            response_headers: HttpHeaders::new(),
            timestamp,
            frames: Vec::new(),
        }
    }

    /// Add a frame to this connection
    pub fn add_frame(&mut self, frame: WebSocketFrame) {
        self.frames.push(frame);
    }
}

/// Pattern for request interception
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterceptionPattern {
    /// URL pattern (supports wildcards)
    #[serde(rename = "urlPattern")]
    pub url_pattern: Option<String>,
    /// Resource type (Document, Image, Script, etc.)
    #[serde(rename = "resourceType")]
    pub resource_type: Option<String>,
    /// Interception stage (Request, HeadersReceived)
    #[serde(rename = "interceptionStage")]
    pub interception_stage: Option<String>,
}

/// Network domain implementation for Chrome DevTools Protocol
///
/// Provides network monitoring, request/response inspection, interception capabilities,
/// cache inspection, and WebSocket frame inspection.
///
/// # Features
/// - **Network Inspector Bridge**: Full request/response inspection
/// - **Request Body Capture**: Capture POST/PUT request bodies
/// - **Response Body Capture**: Capture response bodies with size limits
/// - **Cache Inspection**: Inspect browser cache contents
/// - **WebSocket Frame Inspection**: Track WebSocket connections and frames
#[derive(Debug)]
pub struct NetworkDomain {
    /// Map of tracked requests (RequestId → RequestInfo)
    request_map: Arc<DashMap<String, RequestInfo>>,
    /// Whether request interception is enabled
    interception_enabled: Arc<AtomicBool>,
    /// List of interception patterns
    interception_patterns: Arc<RwLock<Vec<InterceptionPattern>>>,
    /// Maximum response body size to capture (bytes)
    max_response_body_size: Arc<AtomicU64>,
    /// Maximum request body size to capture (bytes)
    max_request_body_size: Arc<AtomicU64>,
    /// Cache storage (CacheId → Vec<CacheEntry>)
    cache_storage: Arc<DashMap<String, Vec<CacheEntry>>>,
    /// Cache metadata (CacheId → CacheInfo)
    cache_info: Arc<DashMap<String, CacheInfo>>,
    /// WebSocket connections (RequestId → WebSocketConnection)
    websocket_connections: Arc<DashMap<String, WebSocketConnection>>,
}

impl NetworkDomain {
    /// Create a new NetworkDomain instance
    ///
    /// # Example
    /// ```
    /// use network_domain::NetworkDomain;
    /// use protocol_handler::DomainHandler;
    ///
    /// let domain = NetworkDomain::new();
    /// assert_eq!(domain.name(), "Network");
    /// ```
    pub fn new() -> Self {
        Self {
            request_map: Arc::new(DashMap::new()),
            interception_enabled: Arc::new(AtomicBool::new(false)),
            interception_patterns: Arc::new(RwLock::new(Vec::new())),
            max_response_body_size: Arc::new(AtomicU64::new(DEFAULT_MAX_RESPONSE_BODY_SIZE as u64)),
            max_request_body_size: Arc::new(AtomicU64::new(DEFAULT_MAX_REQUEST_BODY_SIZE as u64)),
            cache_storage: Arc::new(DashMap::new()),
            cache_info: Arc::new(DashMap::new()),
            websocket_connections: Arc::new(DashMap::new()),
        }
    }

    /// Create a NetworkDomain with custom body size limits
    ///
    /// # Arguments
    /// * `max_response_size` - Maximum response body size in bytes
    /// * `max_request_size` - Maximum request body size in bytes
    pub fn with_limits(max_response_size: usize, max_request_size: usize) -> Self {
        Self {
            request_map: Arc::new(DashMap::new()),
            interception_enabled: Arc::new(AtomicBool::new(false)),
            interception_patterns: Arc::new(RwLock::new(Vec::new())),
            max_response_body_size: Arc::new(AtomicU64::new(max_response_size as u64)),
            max_request_body_size: Arc::new(AtomicU64::new(max_request_size as u64)),
            cache_storage: Arc::new(DashMap::new()),
            cache_info: Arc::new(DashMap::new()),
            websocket_connections: Arc::new(DashMap::new()),
        }
    }

    /// Enable network monitoring
    ///
    /// # Arguments
    /// * `params` - Optional parameters including maxTotalBufferSize and maxResourceBufferSize
    ///
    /// # Returns
    /// Empty result on success, error on failure
    pub async fn enable(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("Network.enable called with params: {:?}", params);

        // In a real implementation, this would:
        // 1. Set up network observers
        // 2. Configure buffer sizes from params
        // 3. Start monitoring network activity

        // For now, just accept the parameters and return success
        if let Some(p) = params {
            if let Some(max_buffer) = p.get("maxTotalBufferSize") {
                debug!("Setting max total buffer size: {}", max_buffer);
            }
            if let Some(max_resource_buffer) = p.get("maxResourceBufferSize") {
                debug!("Setting max resource buffer size: {}", max_resource_buffer);
            }
        }

        Ok(json!({}))
    }

    /// Disable network monitoring
    ///
    /// # Returns
    /// Empty result on success
    pub async fn disable(&self) -> Result<Value, CdpError> {
        debug!("Network.disable called");

        // In a real implementation, this would:
        // 1. Stop network monitoring
        // 2. Clean up observers
        // 3. Clear tracked requests

        // For now, just return success
        Ok(json!({}))
    }

    /// Get response body for a given request
    ///
    /// # Arguments
    /// * `params` - Parameters containing the requestId
    ///
    /// # Returns
    /// Response body and base64Encoded flag, or error if request not found
    pub async fn get_response_body(&self, params: Option<Value>) -> Result<Value, CdpError> {
        let params = params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?;

        let request_id = params
            .get("requestId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CdpError::invalid_params("Missing requestId parameter"))?;

        debug!("Network.getResponseBody for request: {}", request_id);

        let request = self.request_map.get(request_id).ok_or_else(|| {
            CdpError::server_error(-32000, format!("Request not found: {}", request_id))
        })?;

        let body = request
            .response_body
            .clone()
            .ok_or_else(|| CdpError::server_error(-32000, "Response body not available"))?;

        Ok(json!({
            "body": body,
            "base64Encoded": request.is_base64
        }))
    }

    /// Enable request interception with specified patterns
    ///
    /// # Arguments
    /// * `params` - Parameters containing patterns array
    ///
    /// # Returns
    /// Empty result on success, error on failure
    pub async fn set_request_interception(&self, params: Option<Value>) -> Result<Value, CdpError> {
        let params = params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?;

        let patterns = params
            .get("patterns")
            .and_then(|v| v.as_array())
            .ok_or_else(|| CdpError::invalid_params("Missing patterns parameter"))?;

        debug!(
            "Network.setRequestInterception with {} patterns",
            patterns.len()
        );

        // Parse patterns
        let mut interception_patterns = Vec::new();
        for pattern_value in patterns {
            let pattern: InterceptionPattern = serde_json::from_value(pattern_value.clone())
                .map_err(|e| CdpError::invalid_params(format!("Invalid pattern: {}", e)))?;
            interception_patterns.push(pattern);
        }

        // Update interception state
        let enabled = !interception_patterns.is_empty();
        self.interception_enabled.store(enabled, Ordering::SeqCst);

        // Store patterns
        *self.interception_patterns.write() = interception_patterns;

        debug!("Request interception enabled: {}", enabled);

        Ok(json!({}))
    }

    /// Track a network request
    ///
    /// # Arguments
    /// * `request_id` - Unique identifier for the request
    /// * `url` - Request URL
    /// * `method` - HTTP method
    pub fn track_request(&self, request_id: String, url: String, method: String) {
        debug!("Tracking request: {} {} {}", method, url, request_id);
        let request_info = RequestInfo::new(request_id.clone(), url, method);
        self.request_map.insert(request_id, request_info);
    }

    /// Track a request with full details (Network Inspector Bridge)
    ///
    /// # Arguments
    /// * `request_id` - Unique identifier for the request
    /// * `url` - Request URL
    /// * `method` - HTTP method
    /// * `headers` - Request headers
    /// * `body` - Request body (if any)
    /// * `resource_type` - Type of resource being requested
    pub fn track_request_full(
        &self,
        request_id: String,
        url: String,
        method: String,
        headers: HttpHeaders,
        body: Option<String>,
        resource_type: Option<String>,
    ) {
        debug!("Tracking full request: {} {} {}", method, url, request_id);

        let max_size = self.max_request_body_size.load(Ordering::Relaxed) as usize;

        let mut request_info = RequestInfo::new(request_id.clone(), url, method);
        request_info.request_headers = headers;
        request_info.resource_type = resource_type;

        // Capture request body if within size limit
        if let Some(b) = body {
            if b.len() <= max_size {
                request_info.request_body = Some(b);
                request_info.request_body_base64 = false;
            } else {
                debug!(
                    "Request body exceeds size limit ({} > {}), truncating",
                    b.len(),
                    max_size
                );
                request_info.request_body = Some(b[..max_size].to_string());
                request_info.request_body_base64 = false;
            }
        }

        self.request_map.insert(request_id, request_info);
    }

    /// Get request body for a given request
    ///
    /// # Arguments
    /// * `params` - Parameters containing the requestId
    ///
    /// # Returns
    /// Request body and base64Encoded flag, or error if request not found
    pub async fn get_request_post_data(&self, params: Option<Value>) -> Result<Value, CdpError> {
        let params = params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?;

        let request_id = params
            .get("requestId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CdpError::invalid_params("Missing requestId parameter"))?;

        debug!("Network.getRequestPostData for request: {}", request_id);

        let request = self.request_map.get(request_id).ok_or_else(|| {
            CdpError::server_error(-32000, format!("Request not found: {}", request_id))
        })?;

        let body = request
            .request_body
            .clone()
            .ok_or_else(|| CdpError::server_error(-32000, "Request body not available"))?;

        Ok(json!({
            "postData": body
        }))
    }

    /// Store response body for a tracked request
    ///
    /// # Arguments
    /// * `request_id` - Request identifier
    /// * `body` - Response body content
    /// * `is_base64` - Whether the body is base64 encoded
    pub fn store_response_body(&self, request_id: String, body: String, is_base64: bool) {
        debug!("Storing response body for request: {}", request_id);

        let max_size = self.max_response_body_size.load(Ordering::Relaxed) as usize;

        if let Some(mut request) = self.request_map.get_mut(&request_id) {
            // Apply size limit
            if body.len() <= max_size {
                request.response_body = Some(body.clone());
                request.response_size = Some(body.len());
            } else {
                debug!(
                    "Response body exceeds size limit ({} > {}), truncating",
                    body.len(),
                    max_size
                );
                request.response_body = Some(body[..max_size].to_string());
                request.response_size = Some(body.len());
            }
            request.is_base64 = is_base64;
        } else {
            warn!(
                "Attempted to store response body for unknown request: {}",
                request_id
            );
        }
    }

    /// Store full response details for a tracked request (Network Inspector Bridge)
    ///
    /// # Arguments
    /// * `request_id` - Request identifier
    /// * `status_code` - HTTP status code
    /// * `headers` - Response headers
    /// * `body` - Response body content
    /// * `is_base64` - Whether the body is base64 encoded
    /// * `from_cache` - Whether response was served from cache
    pub fn store_response_full(
        &self,
        request_id: String,
        status_code: u16,
        headers: HttpHeaders,
        body: String,
        is_base64: bool,
        from_cache: bool,
    ) {
        debug!(
            "Storing full response for request: {} (status: {})",
            request_id, status_code
        );

        let max_size = self.max_response_body_size.load(Ordering::Relaxed) as usize;

        if let Some(mut request) = self.request_map.get_mut(&request_id) {
            request.status_code = Some(status_code);
            request.response_headers = headers;
            request.from_cache = from_cache;

            // Calculate response time
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs_f64() * 1000.0)
                .unwrap_or(0.0);
            request.response_time = Some(now - request.timestamp);

            // Apply size limit
            let body_len = body.len();
            if body_len <= max_size {
                request.response_body = Some(body);
            } else {
                info!(
                    "Response body exceeds size limit ({} > {}), truncating",
                    body_len, max_size
                );
                request.response_body = Some(body[..max_size].to_string());
            }
            request.response_size = Some(body_len);
            request.is_base64 = is_base64;
        } else {
            warn!(
                "Attempted to store response for unknown request: {}",
                request_id
            );
        }
    }

    /// Get all tracked requests (for Network Inspector)
    pub fn get_all_requests(&self) -> Vec<RequestInfo> {
        self.request_map.iter().map(|r| r.value().clone()).collect()
    }

    /// Clear all tracked requests
    pub fn clear_requests(&self) {
        debug!("Clearing all tracked requests");
        self.request_map.clear();
    }

    /// Check if a request is being tracked
    ///
    /// # Arguments
    /// * `request_id` - Request identifier to check
    ///
    /// # Returns
    /// true if the request is being tracked, false otherwise
    pub fn has_request(&self, request_id: &str) -> bool {
        self.request_map.contains_key(request_id)
    }

    /// Remove a tracked request
    ///
    /// # Arguments
    /// * `request_id` - Request identifier to remove
    pub fn untrack_request(&self, request_id: &str) {
        debug!("Untracking request: {}", request_id);
        self.request_map.remove(request_id);
    }

    /// Check if request interception is enabled
    ///
    /// # Returns
    /// true if interception is enabled, false otherwise
    pub fn is_interception_enabled(&self) -> bool {
        self.interception_enabled.load(Ordering::SeqCst)
    }

    // =========================================================================
    // Cache Inspection Methods (FEAT-030)
    // =========================================================================

    /// Request cache names for a security origin
    ///
    /// Implements CacheStorage.requestCacheNames
    pub async fn request_cache_names(&self, params: Option<Value>) -> Result<Value, CdpError> {
        let params = params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?;

        let security_origin = params
            .get("securityOrigin")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CdpError::invalid_params("Missing securityOrigin parameter"))?;

        debug!(
            "CacheStorage.requestCacheNames for origin: {}",
            security_origin
        );

        // Collect caches for this origin
        let caches: Vec<Value> = self
            .cache_info
            .iter()
            .filter(|entry| entry.value().security_origin == security_origin)
            .map(|entry| {
                json!({
                    "cacheId": entry.value().cache_id,
                    "securityOrigin": entry.value().security_origin,
                    "cacheName": entry.value().cache_name
                })
            })
            .collect();

        Ok(json!({
            "caches": caches
        }))
    }

    /// Request entries from a cache
    ///
    /// Implements CacheStorage.requestEntries
    pub async fn request_cache_entries(&self, params: Option<Value>) -> Result<Value, CdpError> {
        let params = params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?;

        let cache_id = params
            .get("cacheId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CdpError::invalid_params("Missing cacheId parameter"))?;

        let skip_count = params
            .get("skipCount")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;

        let page_size = params
            .get("pageSize")
            .and_then(|v| v.as_u64())
            .unwrap_or(100) as usize;

        debug!(
            "CacheStorage.requestEntries for cache: {} (skip: {}, page: {})",
            cache_id, skip_count, page_size
        );

        let entries: Vec<Value> = self
            .cache_storage
            .get(cache_id)
            .map(|cache| {
                cache
                    .value()
                    .iter()
                    .skip(skip_count)
                    .take(page_size)
                    .map(|entry| {
                        json!({
                            "requestURL": entry.request_url,
                            "requestMethod": entry.request_method,
                            "requestHeaders": entry.request_headers,
                            "responseTime": entry.response_time,
                            "responseStatus": entry.response_status,
                            "responseStatusText": entry.response_status_text,
                            "responseType": entry.response_type,
                            "responseHeaders": entry.response_headers
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        let total_count = self
            .cache_storage
            .get(cache_id)
            .map(|c| c.value().len())
            .unwrap_or(0);

        let return_count = entries.len();

        Ok(json!({
            "cacheDataEntries": entries,
            "returnCount": return_count,
            "hasMore": skip_count + return_count < total_count
        }))
    }

    /// Delete a cache
    ///
    /// Implements CacheStorage.deleteCache
    pub async fn delete_cache(&self, params: Option<Value>) -> Result<Value, CdpError> {
        let params = params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?;

        let cache_id = params
            .get("cacheId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CdpError::invalid_params("Missing cacheId parameter"))?;

        debug!("CacheStorage.deleteCache: {}", cache_id);

        self.cache_storage.remove(cache_id);
        self.cache_info.remove(cache_id);

        Ok(json!({}))
    }

    /// Delete a specific cache entry
    ///
    /// Implements CacheStorage.deleteEntry
    pub async fn delete_cache_entry(&self, params: Option<Value>) -> Result<Value, CdpError> {
        let params = params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?;

        let cache_id = params
            .get("cacheId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CdpError::invalid_params("Missing cacheId parameter"))?;

        let request_url = params
            .get("request")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CdpError::invalid_params("Missing request parameter"))?;

        debug!(
            "CacheStorage.deleteEntry: {} from cache {}",
            request_url, cache_id
        );

        if let Some(mut cache) = self.cache_storage.get_mut(cache_id) {
            cache.retain(|entry| entry.request_url != request_url);
        }

        Ok(json!({}))
    }

    /// Add a cache (for testing/internal use)
    pub fn add_cache(&self, cache_id: String, security_origin: String, cache_name: String) {
        debug!(
            "Adding cache: {} (origin: {}, name: {})",
            cache_id, security_origin, cache_name
        );

        self.cache_info.insert(
            cache_id.clone(),
            CacheInfo {
                cache_id: cache_id.clone(),
                security_origin,
                cache_name,
            },
        );
        self.cache_storage.insert(cache_id, Vec::new());
    }

    /// Add an entry to a cache (for testing/internal use)
    pub fn add_cache_entry(&self, cache_id: &str, entry: CacheEntry) {
        debug!(
            "Adding cache entry: {} to cache {}",
            entry.request_url, cache_id
        );

        if let Some(mut cache) = self.cache_storage.get_mut(cache_id) {
            cache.push(entry);
        } else {
            warn!(
                "Attempted to add entry to non-existent cache: {}",
                cache_id
            );
        }
    }

    // =========================================================================
    // WebSocket Frame Inspection Methods (FEAT-031)
    // =========================================================================

    /// Track a new WebSocket connection
    pub fn track_websocket(&self, request_id: String, url: String) {
        debug!("Tracking WebSocket connection: {} ({})", request_id, url);
        let connection = WebSocketConnection::new(request_id.clone(), url);
        self.websocket_connections.insert(request_id, connection);
    }

    /// Track WebSocket connection with full details
    pub fn track_websocket_full(
        &self,
        request_id: String,
        url: String,
        request_headers: HttpHeaders,
    ) {
        debug!(
            "Tracking WebSocket connection (full): {} ({})",
            request_id, url
        );
        let mut connection = WebSocketConnection::new(request_id.clone(), url);
        connection.request_headers = request_headers;
        self.websocket_connections.insert(request_id, connection);
    }

    /// Update WebSocket connection state
    pub fn update_websocket_state(&self, request_id: &str, state: WebSocketState) {
        debug!(
            "Updating WebSocket state for {}: {:?}",
            request_id, state
        );

        if let Some(mut conn) = self.websocket_connections.get_mut(request_id) {
            conn.state = state;
        } else {
            warn!(
                "Attempted to update state for unknown WebSocket: {}",
                request_id
            );
        }
    }

    /// Set WebSocket handshake response headers
    pub fn set_websocket_response_headers(&self, request_id: &str, headers: HttpHeaders) {
        if let Some(mut conn) = self.websocket_connections.get_mut(request_id) {
            conn.response_headers = headers;
            conn.state = WebSocketState::Open;
        }
    }

    /// Add a WebSocket frame to a connection
    pub fn add_websocket_frame(&self, request_id: &str, frame: WebSocketFrame) {
        debug!(
            "Adding WebSocket frame to {}: {:?} (outgoing: {})",
            request_id, frame.opcode, frame.is_outgoing
        );

        if let Some(mut conn) = self.websocket_connections.get_mut(request_id) {
            conn.add_frame(frame);
        } else {
            warn!(
                "Attempted to add frame to unknown WebSocket: {}",
                request_id
            );
        }
    }

    /// Add a text frame to a WebSocket connection
    pub fn add_websocket_text_frame(&self, request_id: &str, data: String, is_outgoing: bool) {
        let frame = WebSocketFrame::text(data, is_outgoing);
        self.add_websocket_frame(request_id, frame);
    }

    /// Add a binary frame to a WebSocket connection (base64 encoded)
    pub fn add_websocket_binary_frame(&self, request_id: &str, data: String, is_outgoing: bool) {
        let frame = WebSocketFrame::binary(data, is_outgoing);
        self.add_websocket_frame(request_id, frame);
    }

    /// Get all frames for a WebSocket connection
    pub fn get_websocket_frames(&self, request_id: &str) -> Option<Vec<WebSocketFrame>> {
        self.websocket_connections
            .get(request_id)
            .map(|conn| conn.frames.clone())
    }

    /// Get WebSocket connection info
    pub fn get_websocket_connection(&self, request_id: &str) -> Option<WebSocketConnection> {
        self.websocket_connections.get(request_id).map(|r| r.clone())
    }

    /// Get all WebSocket connections
    pub fn get_all_websocket_connections(&self) -> Vec<WebSocketConnection> {
        self.websocket_connections
            .iter()
            .map(|r| r.value().clone())
            .collect()
    }

    /// Close a WebSocket connection
    pub fn close_websocket(&self, request_id: &str) {
        debug!("Closing WebSocket connection: {}", request_id);
        self.update_websocket_state(request_id, WebSocketState::Closed);
    }

    /// Remove a WebSocket connection from tracking
    pub fn untrack_websocket(&self, request_id: &str) {
        debug!("Untracking WebSocket connection: {}", request_id);
        self.websocket_connections.remove(request_id);
    }

    /// Check if a WebSocket connection is being tracked
    pub fn has_websocket(&self, request_id: &str) -> bool {
        self.websocket_connections.contains_key(request_id)
    }

    /// CDP method: Get WebSocket frame payload (Network.webSocketFrameReceived data)
    pub async fn get_websocket_frame_data(
        &self,
        params: Option<Value>,
    ) -> Result<Value, CdpError> {
        let params = params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?;

        let request_id = params
            .get("requestId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CdpError::invalid_params("Missing requestId parameter"))?;

        debug!("Getting WebSocket frames for: {}", request_id);

        let connection = self.websocket_connections.get(request_id).ok_or_else(|| {
            CdpError::server_error(-32000, format!("WebSocket not found: {}", request_id))
        })?;

        let frames: Vec<Value> = connection
            .frames
            .iter()
            .map(|frame| {
                json!({
                    "opcode": frame.opcode,
                    "mask": frame.mask,
                    "payloadData": frame.payload_data,
                    "isBase64": frame.is_base64,
                    "timestamp": frame.timestamp,
                    "isOutgoing": frame.is_outgoing
                })
            })
            .collect();

        Ok(json!({
            "requestId": request_id,
            "url": connection.url,
            "state": connection.state,
            "timestamp": connection.timestamp,
            "frames": frames
        }))
    }
}

impl Default for NetworkDomain {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DomainHandler for NetworkDomain {
    /// Returns the name of this domain
    fn name(&self) -> &str {
        "Network"
    }

    /// Handle a method call for the Network domain
    ///
    /// # Arguments
    /// * `method` - Method name (without "Network." prefix)
    /// * `params` - Optional method parameters
    ///
    /// # Returns
    /// Method result or error
    async fn handle_method(&self, method: &str, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("Network domain handling method: {}", method);

        match method {
            // Core Network methods
            "enable" => self.enable(params).await,
            "disable" => self.disable().await,
            "getResponseBody" => self.get_response_body(params).await,
            "getRequestPostData" => self.get_request_post_data(params).await,
            "setRequestInterception" => self.set_request_interception(params).await,

            // Cache Storage methods (CacheStorage domain, often routed through Network)
            "requestCacheNames" => self.request_cache_names(params).await,
            "requestEntries" => self.request_cache_entries(params).await,
            "deleteCache" => self.delete_cache(params).await,
            "deleteEntry" => self.delete_cache_entry(params).await,

            // WebSocket methods
            "getWebSocketFrames" => self.get_websocket_frame_data(params).await,

            _ => {
                warn!("Unknown Network method: {}", method);
                Err(CdpError::method_not_found(format!("Network.{}", method)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Core Network Domain Tests
    // =========================================================================

    #[tokio::test]
    async fn test_new_domain() {
        let domain = NetworkDomain::new();
        assert_eq!(domain.name(), "Network");
    }

    #[tokio::test]
    async fn test_enable_basic() {
        let domain = NetworkDomain::new();
        let result = domain.enable(None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_track_and_retrieve() {
        let domain = NetworkDomain::new();

        domain.track_request(
            "test-123".to_string(),
            "https://example.com".to_string(),
            "GET".to_string(),
        );

        assert!(domain.has_request("test-123"));

        domain.store_response_body("test-123".to_string(), "Hello".to_string(), false);

        let params = json!({"requestId": "test-123"});
        let result = domain.get_response_body(Some(params)).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response["body"], "Hello");
    }

    #[tokio::test]
    async fn test_interception() {
        let domain = NetworkDomain::new();

        assert!(!domain.is_interception_enabled());

        let params = json!({
            "patterns": [{"urlPattern": "*"}]
        });

        domain.set_request_interception(Some(params)).await.unwrap();

        assert!(domain.is_interception_enabled());
    }

    // =========================================================================
    // Network Inspector Bridge Tests (FEAT-018)
    // =========================================================================

    #[tokio::test]
    async fn test_track_request_full() {
        let domain = NetworkDomain::new();

        let mut headers = HttpHeaders::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers.insert("Authorization".to_string(), "Bearer token".to_string());

        domain.track_request_full(
            "req-001".to_string(),
            "https://api.example.com/users".to_string(),
            "POST".to_string(),
            headers,
            Some(r#"{"name": "John"}"#.to_string()),
            Some("XHR".to_string()),
        );

        assert!(domain.has_request("req-001"));

        let params = json!({"requestId": "req-001"});
        let result = domain.get_request_post_data(Some(params)).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response["postData"], r#"{"name": "John"}"#);
    }

    #[tokio::test]
    async fn test_store_response_full() {
        let domain = NetworkDomain::new();

        domain.track_request(
            "req-002".to_string(),
            "https://example.com".to_string(),
            "GET".to_string(),
        );

        let mut headers = HttpHeaders::new();
        headers.insert("Content-Type".to_string(), "text/html".to_string());

        domain.store_response_full(
            "req-002".to_string(),
            200,
            headers,
            "<html>Hello</html>".to_string(),
            false,
            false,
        );

        let params = json!({"requestId": "req-002"});
        let result = domain.get_response_body(Some(params)).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response["body"], "<html>Hello</html>");
    }

    #[tokio::test]
    async fn test_response_body_size_limit() {
        let domain = NetworkDomain::with_limits(100, 50); // 100 bytes max response

        domain.track_request(
            "req-003".to_string(),
            "https://example.com".to_string(),
            "GET".to_string(),
        );

        // Create a body larger than the limit
        let large_body = "x".repeat(200);
        domain.store_response_body("req-003".to_string(), large_body, false);

        let params = json!({"requestId": "req-003"});
        let result = domain.get_response_body(Some(params)).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        // Body should be truncated to 100 bytes
        assert_eq!(response["body"].as_str().unwrap().len(), 100);
    }

    #[tokio::test]
    async fn test_get_all_requests() {
        let domain = NetworkDomain::new();

        domain.track_request("req-a".to_string(), "https://a.com".to_string(), "GET".to_string());
        domain.track_request("req-b".to_string(), "https://b.com".to_string(), "POST".to_string());

        let requests = domain.get_all_requests();
        assert_eq!(requests.len(), 2);
    }

    #[tokio::test]
    async fn test_clear_requests() {
        let domain = NetworkDomain::new();

        domain.track_request("req-x".to_string(), "https://x.com".to_string(), "GET".to_string());
        assert!(domain.has_request("req-x"));

        domain.clear_requests();
        assert!(!domain.has_request("req-x"));
    }

    // =========================================================================
    // Cache Inspection Tests (FEAT-030)
    // =========================================================================

    #[tokio::test]
    async fn test_request_cache_names() {
        let domain = NetworkDomain::new();

        // Add test caches
        domain.add_cache(
            "cache-1".to_string(),
            "https://example.com".to_string(),
            "v1-cache".to_string(),
        );
        domain.add_cache(
            "cache-2".to_string(),
            "https://example.com".to_string(),
            "v2-cache".to_string(),
        );
        domain.add_cache(
            "cache-3".to_string(),
            "https://other.com".to_string(),
            "other-cache".to_string(),
        );

        let params = json!({"securityOrigin": "https://example.com"});
        let result = domain.request_cache_names(Some(params)).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        let caches = response["caches"].as_array().unwrap();
        assert_eq!(caches.len(), 2);
    }

    #[tokio::test]
    async fn test_request_cache_entries() {
        let domain = NetworkDomain::new();

        domain.add_cache(
            "test-cache".to_string(),
            "https://example.com".to_string(),
            "my-cache".to_string(),
        );

        // Add entries
        let entry1 = CacheEntry {
            request_url: "https://example.com/api/data".to_string(),
            request_method: "GET".to_string(),
            request_headers: vec![],
            response_time: 1234567890.0,
            response_status: 200,
            response_status_text: "OK".to_string(),
            response_type: "basic".to_string(),
            response_headers: vec![],
        };

        let entry2 = CacheEntry {
            request_url: "https://example.com/api/users".to_string(),
            request_method: "GET".to_string(),
            request_headers: vec![],
            response_time: 1234567891.0,
            response_status: 200,
            response_status_text: "OK".to_string(),
            response_type: "basic".to_string(),
            response_headers: vec![],
        };

        domain.add_cache_entry("test-cache", entry1);
        domain.add_cache_entry("test-cache", entry2);

        let params = json!({"cacheId": "test-cache"});
        let result = domain.request_cache_entries(Some(params)).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        let entries = response["cacheDataEntries"].as_array().unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(response["returnCount"], 2);
        assert_eq!(response["hasMore"], false);
    }

    #[tokio::test]
    async fn test_cache_pagination() {
        let domain = NetworkDomain::new();

        domain.add_cache(
            "paginated-cache".to_string(),
            "https://example.com".to_string(),
            "big-cache".to_string(),
        );

        // Add 5 entries
        for i in 0..5 {
            let entry = CacheEntry {
                request_url: format!("https://example.com/item/{}", i),
                request_method: "GET".to_string(),
                request_headers: vec![],
                response_time: 1234567890.0 + i as f64,
                response_status: 200,
                response_status_text: "OK".to_string(),
                response_type: "basic".to_string(),
                response_headers: vec![],
            };
            domain.add_cache_entry("paginated-cache", entry);
        }

        // Request with pagination
        let params = json!({
            "cacheId": "paginated-cache",
            "skipCount": 2,
            "pageSize": 2
        });
        let result = domain.request_cache_entries(Some(params)).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        let entries = response["cacheDataEntries"].as_array().unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(response["hasMore"], true);
    }

    #[tokio::test]
    async fn test_delete_cache() {
        let domain = NetworkDomain::new();

        domain.add_cache(
            "delete-me".to_string(),
            "https://example.com".to_string(),
            "temp-cache".to_string(),
        );

        // Verify cache exists
        let params = json!({"securityOrigin": "https://example.com"});
        let result = domain.request_cache_names(Some(params.clone())).await.unwrap();
        assert_eq!(result["caches"].as_array().unwrap().len(), 1);

        // Delete cache
        let delete_params = json!({"cacheId": "delete-me"});
        domain.delete_cache(Some(delete_params)).await.unwrap();

        // Verify cache is gone
        let result = domain.request_cache_names(Some(params)).await.unwrap();
        assert_eq!(result["caches"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_delete_cache_entry() {
        let domain = NetworkDomain::new();

        domain.add_cache(
            "entry-cache".to_string(),
            "https://example.com".to_string(),
            "entry-test".to_string(),
        );

        let entry1 = CacheEntry {
            request_url: "https://example.com/keep".to_string(),
            request_method: "GET".to_string(),
            request_headers: vec![],
            response_time: 1234567890.0,
            response_status: 200,
            response_status_text: "OK".to_string(),
            response_type: "basic".to_string(),
            response_headers: vec![],
        };

        let entry2 = CacheEntry {
            request_url: "https://example.com/delete".to_string(),
            request_method: "GET".to_string(),
            request_headers: vec![],
            response_time: 1234567891.0,
            response_status: 200,
            response_status_text: "OK".to_string(),
            response_type: "basic".to_string(),
            response_headers: vec![],
        };

        domain.add_cache_entry("entry-cache", entry1);
        domain.add_cache_entry("entry-cache", entry2);

        // Delete one entry
        let delete_params = json!({
            "cacheId": "entry-cache",
            "request": "https://example.com/delete"
        });
        domain.delete_cache_entry(Some(delete_params)).await.unwrap();

        // Verify only one entry remains
        let params = json!({"cacheId": "entry-cache"});
        let result = domain.request_cache_entries(Some(params)).await.unwrap();
        let entries = result["cacheDataEntries"].as_array().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0]["requestURL"], "https://example.com/keep");
    }

    // =========================================================================
    // WebSocket Frame Inspection Tests (FEAT-031)
    // =========================================================================

    #[tokio::test]
    async fn test_track_websocket() {
        let domain = NetworkDomain::new();

        domain.track_websocket(
            "ws-001".to_string(),
            "wss://example.com/socket".to_string(),
        );

        assert!(domain.has_websocket("ws-001"));

        let conn = domain.get_websocket_connection("ws-001").unwrap();
        assert_eq!(conn.url, "wss://example.com/socket");
        assert_eq!(conn.state, WebSocketState::Connecting);
    }

    #[tokio::test]
    async fn test_websocket_state_transitions() {
        let domain = NetworkDomain::new();

        domain.track_websocket("ws-002".to_string(), "wss://example.com/ws".to_string());

        // Initial state
        let conn = domain.get_websocket_connection("ws-002").unwrap();
        assert_eq!(conn.state, WebSocketState::Connecting);

        // Transition to Open
        domain.update_websocket_state("ws-002", WebSocketState::Open);
        let conn = domain.get_websocket_connection("ws-002").unwrap();
        assert_eq!(conn.state, WebSocketState::Open);

        // Transition to Closing
        domain.update_websocket_state("ws-002", WebSocketState::Closing);
        let conn = domain.get_websocket_connection("ws-002").unwrap();
        assert_eq!(conn.state, WebSocketState::Closing);

        // Transition to Closed
        domain.close_websocket("ws-002");
        let conn = domain.get_websocket_connection("ws-002").unwrap();
        assert_eq!(conn.state, WebSocketState::Closed);
    }

    #[tokio::test]
    async fn test_websocket_text_frames() {
        let domain = NetworkDomain::new();

        domain.track_websocket("ws-003".to_string(), "wss://chat.example.com".to_string());
        domain.update_websocket_state("ws-003", WebSocketState::Open);

        // Add text frames
        domain.add_websocket_text_frame("ws-003", "Hello, server!".to_string(), true);
        domain.add_websocket_text_frame("ws-003", "Hello, client!".to_string(), false);

        let frames = domain.get_websocket_frames("ws-003").unwrap();
        assert_eq!(frames.len(), 2);

        assert_eq!(frames[0].opcode, WebSocketOpcode::Text);
        assert_eq!(frames[0].payload_data, "Hello, server!");
        assert!(frames[0].is_outgoing);
        assert!(!frames[0].is_base64);

        assert_eq!(frames[1].opcode, WebSocketOpcode::Text);
        assert_eq!(frames[1].payload_data, "Hello, client!");
        assert!(!frames[1].is_outgoing);
    }

    #[tokio::test]
    async fn test_websocket_binary_frames() {
        let domain = NetworkDomain::new();

        domain.track_websocket("ws-004".to_string(), "wss://binary.example.com".to_string());
        domain.update_websocket_state("ws-004", WebSocketState::Open);

        // Add binary frame (base64 encoded)
        domain.add_websocket_binary_frame(
            "ws-004",
            "SGVsbG8gV29ybGQ=".to_string(), // "Hello World" in base64
            true,
        );

        let frames = domain.get_websocket_frames("ws-004").unwrap();
        assert_eq!(frames.len(), 1);

        assert_eq!(frames[0].opcode, WebSocketOpcode::Binary);
        assert!(frames[0].is_base64);
        assert!(frames[0].is_outgoing);
    }

    #[tokio::test]
    async fn test_get_websocket_frame_data() {
        let domain = NetworkDomain::new();

        domain.track_websocket("ws-005".to_string(), "wss://test.example.com".to_string());
        domain.update_websocket_state("ws-005", WebSocketState::Open);

        domain.add_websocket_text_frame("ws-005", "test message".to_string(), true);

        let params = json!({"requestId": "ws-005"});
        let result = domain.get_websocket_frame_data(Some(params)).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response["requestId"], "ws-005");
        assert_eq!(response["url"], "wss://test.example.com");
        assert_eq!(response["state"], "open");

        let frames = response["frames"].as_array().unwrap();
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0]["payloadData"], "test message");
    }

    #[tokio::test]
    async fn test_websocket_frame_timing() {
        let domain = NetworkDomain::new();

        domain.track_websocket("ws-006".to_string(), "wss://time.example.com".to_string());
        domain.update_websocket_state("ws-006", WebSocketState::Open);

        let frame = WebSocketFrame::text("timed message".to_string(), true);
        let frame_timestamp = frame.timestamp;
        domain.add_websocket_frame("ws-006", frame);

        let frames = domain.get_websocket_frames("ws-006").unwrap();
        assert!(frames[0].timestamp > 0.0);
        assert_eq!(frames[0].timestamp, frame_timestamp);
    }

    #[tokio::test]
    async fn test_get_all_websocket_connections() {
        let domain = NetworkDomain::new();

        domain.track_websocket("ws-a".to_string(), "wss://a.example.com".to_string());
        domain.track_websocket("ws-b".to_string(), "wss://b.example.com".to_string());

        let connections = domain.get_all_websocket_connections();
        assert_eq!(connections.len(), 2);
    }

    #[tokio::test]
    async fn test_untrack_websocket() {
        let domain = NetworkDomain::new();

        domain.track_websocket("ws-remove".to_string(), "wss://remove.example.com".to_string());
        assert!(domain.has_websocket("ws-remove"));

        domain.untrack_websocket("ws-remove");
        assert!(!domain.has_websocket("ws-remove"));
    }

    #[tokio::test]
    async fn test_websocket_opcode_from_u8() {
        assert_eq!(WebSocketOpcode::from_u8(0), WebSocketOpcode::Continuation);
        assert_eq!(WebSocketOpcode::from_u8(1), WebSocketOpcode::Text);
        assert_eq!(WebSocketOpcode::from_u8(2), WebSocketOpcode::Binary);
        assert_eq!(WebSocketOpcode::from_u8(8), WebSocketOpcode::Close);
        assert_eq!(WebSocketOpcode::from_u8(9), WebSocketOpcode::Ping);
        assert_eq!(WebSocketOpcode::from_u8(10), WebSocketOpcode::Pong);
        assert_eq!(WebSocketOpcode::from_u8(99), WebSocketOpcode::Binary); // Unknown defaults to Binary
    }

    // =========================================================================
    // Error Handling Tests
    // =========================================================================

    #[tokio::test]
    async fn test_cache_missing_params() {
        let domain = NetworkDomain::new();

        let result = domain.request_cache_names(None).await;
        assert!(result.is_err());

        let result = domain.request_cache_entries(None).await;
        assert!(result.is_err());

        let result = domain.delete_cache(None).await;
        assert!(result.is_err());

        let result = domain.delete_cache_entry(None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_websocket_not_found() {
        let domain = NetworkDomain::new();

        let params = json!({"requestId": "non-existent"});
        let result = domain.get_websocket_frame_data(Some(params)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_request_post_data_not_found() {
        let domain = NetworkDomain::new();

        let params = json!({"requestId": "missing-req"});
        let result = domain.get_request_post_data(Some(params)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_handle_method_routing() {
        let domain = NetworkDomain::new();

        // Test cache methods routing
        domain.add_cache(
            "route-test".to_string(),
            "https://test.com".to_string(),
            "test".to_string(),
        );

        let params = json!({"securityOrigin": "https://test.com"});
        let result = domain.handle_method("requestCacheNames", Some(params)).await;
        assert!(result.is_ok());

        let params = json!({"cacheId": "route-test"});
        let result = domain.handle_method("requestEntries", Some(params)).await;
        assert!(result.is_ok());

        // Test WebSocket method routing
        domain.track_websocket("ws-route".to_string(), "wss://ws.test.com".to_string());
        let params = json!({"requestId": "ws-route"});
        let result = domain.handle_method("getWebSocketFrames", Some(params)).await;
        assert!(result.is_ok());
    }
}
