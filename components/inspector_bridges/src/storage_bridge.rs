//! Storage Bridge implementation
//!
//! Provides bridge for browser storage access via Chrome DevTools Protocol.
//! Implements FEAT-021: Storage Bridge.
//!
//! Features:
//! - Local/Session storage access
//! - IndexedDB inspection
//! - Cookie management

use async_trait::async_trait;
use cdp_types::CdpError;
use protocol_handler::DomainHandler;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn};

/// Storage item
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StorageItem {
    /// Item key
    pub key: String,
    /// Item value
    pub value: String,
}

/// Storage area type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum StorageAreaType {
    /// Local storage (persistent)
    LocalStorage,
    /// Session storage (per-tab, cleared on close)
    SessionStorage,
}

/// Cookie with full properties
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Cookie {
    /// Cookie name
    pub name: String,
    /// Cookie value
    pub value: String,
    /// Cookie domain
    pub domain: String,
    /// Cookie path
    pub path: String,
    /// Cookie expiration date as the number of seconds since the UNIX epoch
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires: Option<f64>,
    /// Cookie size
    pub size: u32,
    /// True if cookie is http-only
    #[serde(default)]
    pub http_only: bool,
    /// True if cookie is secure
    #[serde(default)]
    pub secure: bool,
    /// True in case of session cookie
    #[serde(default)]
    pub session: bool,
    /// Cookie SameSite type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub same_site: Option<CookieSameSite>,
    /// Cookie priority
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<CookiePriority>,
    /// Whether the cookie is same-party
    #[serde(default)]
    pub same_party: bool,
    /// Source scheme
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_scheme: Option<CookieSourceScheme>,
    /// Source port
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_port: Option<u16>,
}

/// Cookie SameSite type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CookieSameSite {
    Strict,
    Lax,
    None,
}

/// Cookie priority
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CookiePriority {
    Low,
    Medium,
    High,
}

/// Cookie source scheme
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CookieSourceScheme {
    Unset,
    NonSecure,
    Secure,
}

/// IndexedDB database info
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseInfo {
    /// Database name
    pub name: String,
    /// Database version
    pub version: u64,
    /// Object stores in the database
    pub object_stores: Vec<ObjectStoreInfo>,
}

/// IndexedDB object store info
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ObjectStoreInfo {
    /// Object store name
    pub name: String,
    /// Key path
    pub key_path: KeyPath,
    /// Auto increment
    #[serde(default)]
    pub auto_increment: bool,
    /// Indexes in the object store
    pub indexes: Vec<IndexInfo>,
}

/// IndexedDB key path
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct KeyPath {
    /// Key path type
    #[serde(rename = "type")]
    pub key_path_type: KeyPathType,
    /// String key path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub string: Option<String>,
    /// Array key path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub array: Option<Vec<String>>,
}

/// Key path type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum KeyPathType {
    Null,
    String,
    Array,
}

/// IndexedDB index info
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IndexInfo {
    /// Index name
    pub name: String,
    /// Key path
    pub key_path: KeyPath,
    /// Unique
    #[serde(default)]
    pub unique: bool,
    /// Multi-entry
    #[serde(default)]
    pub multi_entry: bool,
}

/// IndexedDB data entry
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DataEntry {
    /// Key
    pub key: serde_json::Value,
    /// Primary key
    pub primary_key: serde_json::Value,
    /// Value
    pub value: serde_json::Value,
}

/// Cache storage info
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CacheInfo {
    /// Cache name
    pub cache_name: String,
    /// Security origin
    pub security_origin: String,
    /// Cache storage ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_id: Option<String>,
}

/// Storage origin info
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StorageOrigin {
    /// Security origin
    pub security_origin: String,
    /// Storage types available
    pub storage_types: Vec<String>,
    /// Total quota (bytes)
    pub quota: u64,
    /// Usage (bytes)
    pub usage: u64,
}

/// Storage bridge state
#[derive(Debug, Clone, Default)]
pub struct StorageBridgeState {
    /// Local storage per origin
    pub local_storage: HashMap<String, HashMap<String, String>>,
    /// Session storage per origin
    pub session_storage: HashMap<String, HashMap<String, String>>,
    /// Cookies
    pub cookies: Vec<Cookie>,
    /// IndexedDB databases per origin
    pub indexed_db: HashMap<String, Vec<DatabaseInfo>>,
    /// Cache storage per origin
    pub cache_storage: HashMap<String, Vec<CacheInfo>>,
    /// Whether tracking is enabled
    pub enabled: bool,
}

impl StorageBridgeState {
    /// Create with mock data
    pub fn with_mock_data() -> Self {
        let mut state = Self::default();

        // Add mock local storage
        let mut local = HashMap::new();
        local.insert("theme".to_string(), "dark".to_string());
        local.insert("language".to_string(), "en-US".to_string());
        local.insert("user_id".to_string(), "12345".to_string());
        state
            .local_storage
            .insert("https://example.com".to_string(), local);

        // Add mock session storage
        let mut session = HashMap::new();
        session.insert("cart_items".to_string(), "[]".to_string());
        session.insert("session_token".to_string(), "abc123".to_string());
        state
            .session_storage
            .insert("https://example.com".to_string(), session);

        // Add mock cookies
        state.cookies.push(Cookie {
            name: "session".to_string(),
            value: "abc123def456".to_string(),
            domain: "example.com".to_string(),
            path: "/".to_string(),
            expires: Some(1735689600.0),
            size: 24,
            http_only: true,
            secure: true,
            session: false,
            same_site: Some(CookieSameSite::Lax),
            priority: Some(CookiePriority::Medium),
            same_party: false,
            source_scheme: Some(CookieSourceScheme::Secure),
            source_port: Some(443),
        });

        // Add mock IndexedDB
        let db = DatabaseInfo {
            name: "myApp".to_string(),
            version: 1,
            object_stores: vec![ObjectStoreInfo {
                name: "users".to_string(),
                key_path: KeyPath {
                    key_path_type: KeyPathType::String,
                    string: Some("id".to_string()),
                    array: None,
                },
                auto_increment: true,
                indexes: vec![IndexInfo {
                    name: "by_email".to_string(),
                    key_path: KeyPath {
                        key_path_type: KeyPathType::String,
                        string: Some("email".to_string()),
                        array: None,
                    },
                    unique: true,
                    multi_entry: false,
                }],
            }],
        };
        state
            .indexed_db
            .insert("https://example.com".to_string(), vec![db]);

        state
    }
}

/// Storage Bridge
///
/// Provides comprehensive browser storage access including localStorage,
/// sessionStorage, cookies, IndexedDB, and cache storage.
pub struct StorageBridge {
    /// Storage state
    state: Arc<RwLock<StorageBridgeState>>,
}

impl StorageBridge {
    /// Create a new Storage Bridge
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(StorageBridgeState::with_mock_data())),
        }
    }

    /// Create with empty state (for testing)
    pub fn empty() -> Self {
        Self {
            state: Arc::new(RwLock::new(StorageBridgeState::default())),
        }
    }

    /// Enable storage tracking
    async fn enable(&self, _params: Option<Value>) -> Result<Value, CdpError> {
        debug!("StorageBridge.enable called");

        let mut state = self.state.write().await;
        state.enabled = true;

        Ok(serde_json::json!({}))
    }

    /// Disable storage tracking
    async fn disable(&self, _params: Option<Value>) -> Result<Value, CdpError> {
        debug!("StorageBridge.disable called");

        let mut state = self.state.write().await;
        state.enabled = false;

        Ok(serde_json::json!({}))
    }

    // ==================== Local/Session Storage ====================

    /// Get storage items
    async fn get_storage_items(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("StorageBridge.getStorageItems called");

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Params {
            security_origin: String,
            storage_area: StorageAreaType,
        }

        let params: Params = serde_json::from_value(
            params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?,
        )
        .map_err(|e| CdpError::invalid_params(format!("Invalid parameters: {}", e)))?;

        let state = self.state.read().await;

        let storage = match params.storage_area {
            StorageAreaType::LocalStorage => &state.local_storage,
            StorageAreaType::SessionStorage => &state.session_storage,
        };

        let items: Vec<StorageItem> = storage
            .get(&params.security_origin)
            .map(|s| {
                s.iter()
                    .map(|(k, v)| StorageItem {
                        key: k.clone(),
                        value: v.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(serde_json::json!({
            "items": items
        }))
    }

    /// Set storage item
    async fn set_storage_item(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("StorageBridge.setStorageItem called");

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Params {
            security_origin: String,
            storage_area: StorageAreaType,
            key: String,
            value: String,
        }

        let params: Params = serde_json::from_value(
            params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?,
        )
        .map_err(|e| CdpError::invalid_params(format!("Invalid parameters: {}", e)))?;

        let mut state = self.state.write().await;

        let storage = match params.storage_area {
            StorageAreaType::LocalStorage => &mut state.local_storage,
            StorageAreaType::SessionStorage => &mut state.session_storage,
        };

        storage
            .entry(params.security_origin)
            .or_default()
            .insert(params.key, params.value);

        Ok(serde_json::json!({}))
    }

    /// Remove storage item
    async fn remove_storage_item(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("StorageBridge.removeStorageItem called");

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Params {
            security_origin: String,
            storage_area: StorageAreaType,
            key: String,
        }

        let params: Params = serde_json::from_value(
            params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?,
        )
        .map_err(|e| CdpError::invalid_params(format!("Invalid parameters: {}", e)))?;

        let mut state = self.state.write().await;

        let storage = match params.storage_area {
            StorageAreaType::LocalStorage => &mut state.local_storage,
            StorageAreaType::SessionStorage => &mut state.session_storage,
        };

        if let Some(origin_storage) = storage.get_mut(&params.security_origin) {
            origin_storage.remove(&params.key);
        }

        Ok(serde_json::json!({}))
    }

    /// Clear storage
    async fn clear_storage(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("StorageBridge.clearStorage called");

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Params {
            security_origin: String,
            storage_area: StorageAreaType,
        }

        let params: Params = serde_json::from_value(
            params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?,
        )
        .map_err(|e| CdpError::invalid_params(format!("Invalid parameters: {}", e)))?;

        let mut state = self.state.write().await;

        let storage = match params.storage_area {
            StorageAreaType::LocalStorage => &mut state.local_storage,
            StorageAreaType::SessionStorage => &mut state.session_storage,
        };

        if let Some(origin_storage) = storage.get_mut(&params.security_origin) {
            origin_storage.clear();
        }

        Ok(serde_json::json!({}))
    }

    // ==================== Cookies ====================

    /// Get cookies
    async fn get_cookies(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("StorageBridge.getCookies called");

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Params {
            #[serde(default)]
            urls: Option<Vec<String>>,
        }

        let params: Params = params
            .map(|p| serde_json::from_value(p).ok())
            .flatten()
            .unwrap_or(Params { urls: None });

        let state = self.state.read().await;

        let cookies: Vec<&Cookie> = if let Some(urls) = params.urls {
            state
                .cookies
                .iter()
                .filter(|c| urls.iter().any(|u| u.contains(&c.domain)))
                .collect()
        } else {
            state.cookies.iter().collect()
        };

        Ok(serde_json::json!({
            "cookies": cookies
        }))
    }

    /// Set cookie
    async fn set_cookie(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("StorageBridge.setCookie called");

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Params {
            name: String,
            value: String,
            domain: String,
            #[serde(default = "default_path")]
            path: String,
            #[serde(default)]
            expires: Option<f64>,
            #[serde(default)]
            http_only: bool,
            #[serde(default)]
            secure: bool,
            #[serde(default)]
            same_site: Option<CookieSameSite>,
        }

        fn default_path() -> String {
            "/".to_string()
        }

        let params: Params = serde_json::from_value(
            params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?,
        )
        .map_err(|e| CdpError::invalid_params(format!("Invalid parameters: {}", e)))?;

        let size = (params.name.len() + params.value.len()) as u32;
        let session = params.expires.is_none();

        let cookie = Cookie {
            name: params.name,
            value: params.value,
            domain: params.domain,
            path: params.path,
            expires: params.expires,
            size,
            http_only: params.http_only,
            secure: params.secure,
            session,
            same_site: params.same_site,
            priority: Some(CookiePriority::Medium),
            same_party: false,
            source_scheme: if params.secure {
                Some(CookieSourceScheme::Secure)
            } else {
                Some(CookieSourceScheme::NonSecure)
            },
            source_port: None,
        };

        let mut state = self.state.write().await;

        // Remove existing cookie with same name and domain
        state
            .cookies
            .retain(|c| !(c.name == cookie.name && c.domain == cookie.domain));

        state.cookies.push(cookie);

        Ok(serde_json::json!({
            "success": true
        }))
    }

    /// Delete cookie
    async fn delete_cookie(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("StorageBridge.deleteCookie called");

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Params {
            name: String,
            domain: String,
        }

        let params: Params = serde_json::from_value(
            params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?,
        )
        .map_err(|e| CdpError::invalid_params(format!("Invalid parameters: {}", e)))?;

        let mut state = self.state.write().await;
        state
            .cookies
            .retain(|c| !(c.name == params.name && c.domain == params.domain));

        Ok(serde_json::json!({}))
    }

    /// Clear all cookies
    async fn clear_cookies(&self, _params: Option<Value>) -> Result<Value, CdpError> {
        debug!("StorageBridge.clearCookies called");

        let mut state = self.state.write().await;
        state.cookies.clear();

        Ok(serde_json::json!({}))
    }

    // ==================== IndexedDB ====================

    /// Get IndexedDB databases for an origin
    async fn get_indexed_db_databases(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("StorageBridge.getIndexedDBDatabases called");

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Params {
            security_origin: String,
        }

        let params: Params = serde_json::from_value(
            params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?,
        )
        .map_err(|e| CdpError::invalid_params(format!("Invalid parameters: {}", e)))?;

        let state = self.state.read().await;

        let databases = state
            .indexed_db
            .get(&params.security_origin)
            .cloned()
            .unwrap_or_default();

        Ok(serde_json::json!({
            "databases": databases
        }))
    }

    /// Get IndexedDB database info
    async fn get_indexed_db_database(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("StorageBridge.getIndexedDBDatabase called");

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Params {
            security_origin: String,
            database_name: String,
        }

        let params: Params = serde_json::from_value(
            params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?,
        )
        .map_err(|e| CdpError::invalid_params(format!("Invalid parameters: {}", e)))?;

        let state = self.state.read().await;

        let database = state
            .indexed_db
            .get(&params.security_origin)
            .and_then(|dbs| dbs.iter().find(|db| db.name == params.database_name))
            .ok_or_else(|| CdpError::server_error(-32000, "Database not found"))?;

        Ok(serde_json::json!({
            "database": database
        }))
    }

    /// Delete IndexedDB database
    async fn delete_indexed_db_database(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("StorageBridge.deleteIndexedDBDatabase called");

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Params {
            security_origin: String,
            database_name: String,
        }

        let params: Params = serde_json::from_value(
            params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?,
        )
        .map_err(|e| CdpError::invalid_params(format!("Invalid parameters: {}", e)))?;

        let mut state = self.state.write().await;

        if let Some(dbs) = state.indexed_db.get_mut(&params.security_origin) {
            dbs.retain(|db| db.name != params.database_name);
        }

        Ok(serde_json::json!({}))
    }

    /// Clear IndexedDB object store
    async fn clear_object_store(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("StorageBridge.clearObjectStore called");

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Params {
            security_origin: String,
            database_name: String,
            object_store_name: String,
        }

        let _params: Params = serde_json::from_value(
            params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?,
        )
        .map_err(|e| CdpError::invalid_params(format!("Invalid parameters: {}", e)))?;

        // In a real implementation, this would clear the object store data
        Ok(serde_json::json!({}))
    }

    // ==================== Storage Usage ====================

    /// Get storage usage and quota for an origin
    async fn get_usage_and_quota(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("StorageBridge.getUsageAndQuota called");

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Params {
            security_origin: String,
        }

        let params: Params = serde_json::from_value(
            params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?,
        )
        .map_err(|e| CdpError::invalid_params(format!("Invalid parameters: {}", e)))?;

        let state = self.state.read().await;

        // Calculate mock usage
        let local_usage = state
            .local_storage
            .get(&params.security_origin)
            .map(|s| s.iter().map(|(k, v)| k.len() + v.len()).sum::<usize>())
            .unwrap_or(0);

        let session_usage = state
            .session_storage
            .get(&params.security_origin)
            .map(|s| s.iter().map(|(k, v)| k.len() + v.len()).sum::<usize>())
            .unwrap_or(0);

        let usage = (local_usage + session_usage) as u64;
        let quota = 10 * 1024 * 1024; // 10MB mock quota

        Ok(serde_json::json!({
            "usage": usage,
            "quota": quota,
            "overrideActive": false,
            "usageBreakdown": [
                {
                    "storageType": "local_storage",
                    "usage": local_usage
                },
                {
                    "storageType": "session_storage",
                    "usage": session_usage
                }
            ]
        }))
    }

    /// Get state (for testing)
    pub async fn get_state(&self) -> StorageBridgeState {
        self.state.read().await.clone()
    }
}

impl Default for StorageBridge {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DomainHandler for StorageBridge {
    fn name(&self) -> &str {
        "StorageBridge"
    }

    async fn handle_method(&self, method: &str, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("StorageBridge domain handling method: {}", method);

        match method {
            // General
            "enable" => self.enable(params).await,
            "disable" => self.disable(params).await,

            // Local/Session Storage
            "getStorageItems" => self.get_storage_items(params).await,
            "setStorageItem" => self.set_storage_item(params).await,
            "removeStorageItem" => self.remove_storage_item(params).await,
            "clearStorage" => self.clear_storage(params).await,

            // Cookies
            "getCookies" => self.get_cookies(params).await,
            "setCookie" => self.set_cookie(params).await,
            "deleteCookie" => self.delete_cookie(params).await,
            "clearCookies" => self.clear_cookies(params).await,

            // IndexedDB
            "getIndexedDBDatabases" => self.get_indexed_db_databases(params).await,
            "getIndexedDBDatabase" => self.get_indexed_db_database(params).await,
            "deleteIndexedDBDatabase" => self.delete_indexed_db_database(params).await,
            "clearObjectStore" => self.clear_object_store(params).await,

            // Usage
            "getUsageAndQuota" => self.get_usage_and_quota(params).await,

            _ => {
                warn!("Unknown StorageBridge method: {}", method);
                Err(CdpError::method_not_found(format!(
                    "StorageBridge.{}",
                    method
                )))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_domain_name() {
        let bridge = StorageBridge::new();
        assert_eq!(bridge.name(), "StorageBridge");
    }

    #[tokio::test]
    async fn test_enable_disable() {
        let bridge = StorageBridge::new();

        let result = bridge.enable(None).await;
        assert!(result.is_ok());

        let state = bridge.get_state().await;
        assert!(state.enabled);

        let result = bridge.disable(None).await;
        assert!(result.is_ok());

        let state = bridge.get_state().await;
        assert!(!state.enabled);
    }

    // ==================== Local/Session Storage Tests ====================

    #[tokio::test]
    async fn test_get_storage_items() {
        let bridge = StorageBridge::new();
        let params = json!({
            "securityOrigin": "https://example.com",
            "storageArea": "localStorage"
        });

        let result = bridge.get_storage_items(Some(params)).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        assert!(value["items"].is_array());
    }

    #[tokio::test]
    async fn test_set_storage_item() {
        let bridge = StorageBridge::empty();
        let params = json!({
            "securityOrigin": "https://example.com",
            "storageArea": "localStorage",
            "key": "test_key",
            "value": "test_value"
        });

        let result = bridge.set_storage_item(Some(params)).await;
        assert!(result.is_ok());

        // Verify item was set
        let get_params = json!({
            "securityOrigin": "https://example.com",
            "storageArea": "localStorage"
        });
        let get_result = bridge.get_storage_items(Some(get_params)).await.unwrap();
        let items = get_result["items"].as_array().unwrap();
        assert!(items.iter().any(|i| i["key"] == "test_key"));
    }

    #[tokio::test]
    async fn test_remove_storage_item() {
        let bridge = StorageBridge::new();

        // First set an item
        let set_params = json!({
            "securityOrigin": "https://example.com",
            "storageArea": "localStorage",
            "key": "to_remove",
            "value": "value"
        });
        bridge.set_storage_item(Some(set_params)).await.unwrap();

        // Then remove it
        let remove_params = json!({
            "securityOrigin": "https://example.com",
            "storageArea": "localStorage",
            "key": "to_remove"
        });
        let result = bridge.remove_storage_item(Some(remove_params)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_clear_storage() {
        let bridge = StorageBridge::new();
        let params = json!({
            "securityOrigin": "https://example.com",
            "storageArea": "localStorage"
        });

        let result = bridge.clear_storage(Some(params)).await;
        assert!(result.is_ok());

        // Verify storage was cleared
        let get_params = json!({
            "securityOrigin": "https://example.com",
            "storageArea": "localStorage"
        });
        let get_result = bridge.get_storage_items(Some(get_params)).await.unwrap();
        assert!(get_result["items"].as_array().unwrap().is_empty());
    }

    // ==================== Cookie Tests ====================

    #[tokio::test]
    async fn test_get_cookies() {
        let bridge = StorageBridge::new();

        let result = bridge.get_cookies(None).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        assert!(value["cookies"].is_array());
    }

    #[tokio::test]
    async fn test_set_cookie() {
        let bridge = StorageBridge::empty();
        let params = json!({
            "name": "test_cookie",
            "value": "test_value",
            "domain": "example.com",
            "path": "/",
            "httpOnly": true,
            "secure": true
        });

        let result = bridge.set_cookie(Some(params)).await;
        assert!(result.is_ok());
        assert!(result.unwrap()["success"].as_bool().unwrap());

        // Verify cookie was set
        let get_result = bridge.get_cookies(None).await.unwrap();
        let cookies = get_result["cookies"].as_array().unwrap();
        assert!(cookies.iter().any(|c| c["name"] == "test_cookie"));
    }

    #[tokio::test]
    async fn test_delete_cookie() {
        let bridge = StorageBridge::new();

        // Add a cookie first
        let set_params = json!({
            "name": "to_delete",
            "value": "value",
            "domain": "example.com"
        });
        bridge.set_cookie(Some(set_params)).await.unwrap();

        // Delete it
        let delete_params = json!({
            "name": "to_delete",
            "domain": "example.com"
        });
        let result = bridge.delete_cookie(Some(delete_params)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_clear_cookies() {
        let bridge = StorageBridge::new();

        let result = bridge.clear_cookies(None).await;
        assert!(result.is_ok());

        let get_result = bridge.get_cookies(None).await.unwrap();
        assert!(get_result["cookies"].as_array().unwrap().is_empty());
    }

    // ==================== IndexedDB Tests ====================

    #[tokio::test]
    async fn test_get_indexed_db_databases() {
        let bridge = StorageBridge::new();
        let params = json!({
            "securityOrigin": "https://example.com"
        });

        let result = bridge.get_indexed_db_databases(Some(params)).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        assert!(value["databases"].is_array());
    }

    #[tokio::test]
    async fn test_get_indexed_db_database() {
        let bridge = StorageBridge::new();
        let params = json!({
            "securityOrigin": "https://example.com",
            "databaseName": "myApp"
        });

        let result = bridge.get_indexed_db_database(Some(params)).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        assert!(value["database"].is_object());
        assert_eq!(value["database"]["name"], "myApp");
    }

    #[tokio::test]
    async fn test_delete_indexed_db_database() {
        let bridge = StorageBridge::new();
        let params = json!({
            "securityOrigin": "https://example.com",
            "databaseName": "myApp"
        });

        let result = bridge.delete_indexed_db_database(Some(params)).await;
        assert!(result.is_ok());
    }

    // ==================== Usage Tests ====================

    #[tokio::test]
    async fn test_get_usage_and_quota() {
        let bridge = StorageBridge::new();
        let params = json!({
            "securityOrigin": "https://example.com"
        });

        let result = bridge.get_usage_and_quota(Some(params)).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        assert!(value["usage"].is_number());
        assert!(value["quota"].is_number());
        assert!(value["usageBreakdown"].is_array());
    }

    #[tokio::test]
    async fn test_unknown_method() {
        let bridge = StorageBridge::new();
        let result = bridge.handle_method("unknownMethod", None).await;
        assert!(result.is_err());
    }
}
