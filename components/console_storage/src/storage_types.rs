// Storage domain types
// Since these are not in cdp_types yet, we define them here

use serde::{Deserialize, Serialize};

/// Cookie object
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_only: Option<bool>,
    /// True if cookie is secure
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secure: Option<bool>,
    /// True in case of session cookie
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<bool>,
    /// Cookie SameSite type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub same_site: Option<CookieSameSite>,
}

/// Cookie SameSite type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CookieSameSite {
    Strict,
    Lax,
    None,
}

/// Storage type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StorageType {
    LocalStorage,
    SessionStorage,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cookie_creation() {
        let cookie = Cookie {
            name: "test".to_string(),
            value: "value".to_string(),
            domain: "example.com".to_string(),
            path: "/".to_string(),
            expires: Some(123456.0),
            size: 10,
            http_only: Some(true),
            secure: Some(true),
            session: Some(false),
            same_site: Some(CookieSameSite::Lax),
        };

        assert_eq!(cookie.name, "test");
        assert_eq!(cookie.domain, "example.com");
    }

    #[test]
    fn test_cookie_serialization() {
        let cookie = Cookie {
            name: "test".to_string(),
            value: "value".to_string(),
            domain: "example.com".to_string(),
            path: "/".to_string(),
            expires: None,
            size: 10,
            http_only: None,
            secure: None,
            session: None,
            same_site: None,
        };

        let json = serde_json::to_string(&cookie).unwrap();
        assert!(json.contains("test"));
        assert!(json.contains("value"));
    }
}
