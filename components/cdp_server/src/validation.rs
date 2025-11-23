//! Validation module for CDP server
//!
//! This module provides comprehensive validation for WebSocket connections
//! including origin validation and security checks.

use crate::error::{CdpServerError, Result};
use std::net::IpAddr;

/// Origin validation configuration
#[derive(Debug, Clone)]
pub struct OriginValidatorConfig {
    /// List of allowed origins (supports wildcards)
    pub allowed_origins: Vec<String>,
    /// Whether to allow null origins (typically from file:// or data: URLs)
    pub allow_null_origin: bool,
    /// Whether to require HTTPS for non-localhost origins
    pub require_https: bool,
    /// Whether to allow localhost connections
    pub allow_localhost: bool,
    /// Whether to allow IP address origins
    pub allow_ip_addresses: bool,
}

impl Default for OriginValidatorConfig {
    fn default() -> Self {
        Self {
            allowed_origins: vec!["http://localhost:*".to_string()],
            allow_null_origin: false,
            require_https: false,
            allow_localhost: true,
            allow_ip_addresses: true,
        }
    }
}

/// Origin validator for WebSocket connections
#[derive(Debug, Clone)]
pub struct OriginValidator {
    config: OriginValidatorConfig,
}

impl OriginValidator {
    /// Create a new origin validator with the given configuration
    pub fn new(config: OriginValidatorConfig) -> Self {
        Self { config }
    }

    /// Create a permissive validator that allows all origins
    /// WARNING: Only use this for development/testing
    pub fn permissive() -> Self {
        Self {
            config: OriginValidatorConfig {
                allowed_origins: vec!["*".to_string()],
                allow_null_origin: true,
                require_https: false,
                allow_localhost: true,
                allow_ip_addresses: true,
            },
        }
    }

    /// Validate an origin string
    ///
    /// # Arguments
    /// * `origin` - The origin header value to validate
    ///
    /// # Returns
    /// * `Ok(())` if the origin is valid
    /// * `Err(CdpServerError::InvalidOrigin)` if the origin is invalid
    pub fn validate(&self, origin: &str) -> Result<()> {
        // Handle null origin
        if origin == "null" {
            return if self.config.allow_null_origin {
                Ok(())
            } else {
                Err(CdpServerError::InvalidOrigin(
                    "Null origins are not allowed".to_string(),
                ))
            };
        }

        // Parse the origin
        let parsed = self.parse_origin(origin)?;

        // Check if it's localhost
        if parsed.is_localhost {
            return if self.config.allow_localhost {
                self.check_allowed_patterns(origin)
            } else {
                Err(CdpServerError::InvalidOrigin(
                    "Localhost origins are not allowed".to_string(),
                ))
            };
        }

        // Check if it's an IP address
        if parsed.is_ip_address {
            if !self.config.allow_ip_addresses {
                return Err(CdpServerError::InvalidOrigin(
                    "IP address origins are not allowed".to_string(),
                ));
            }
        }

        // Check HTTPS requirement for non-localhost
        if self.config.require_https && !parsed.is_localhost && parsed.scheme != "https" {
            return Err(CdpServerError::InvalidOrigin(
                "HTTPS is required for non-localhost origins".to_string(),
            ));
        }

        // Check against allowed patterns
        self.check_allowed_patterns(origin)
    }

    /// Parse an origin string into its components
    fn parse_origin(&self, origin: &str) -> Result<ParsedOrigin> {
        // Check for scheme separator
        let scheme_end = origin.find("://").ok_or_else(|| {
            CdpServerError::InvalidOrigin("Invalid origin format: missing scheme".to_string())
        })?;

        let scheme = &origin[..scheme_end];
        let rest = &origin[scheme_end + 3..];

        // Validate scheme
        if !["http", "https", "ws", "wss"].contains(&scheme) {
            return Err(CdpServerError::InvalidOrigin(format!(
                "Invalid scheme: {}",
                scheme
            )));
        }

        // Parse host and port
        let (host, port) = Self::parse_host_port(rest)?;

        // Check if localhost
        let is_localhost = Self::is_localhost(&host);

        // Check if IP address
        let is_ip_address = host.parse::<IpAddr>().is_ok() || Self::is_ipv6_bracket(&host);

        Ok(ParsedOrigin {
            scheme: scheme.to_string(),
            host: host.to_string(),
            port,
            is_localhost,
            is_ip_address,
        })
    }

    /// Parse host and optional port from the rest of the origin URL
    fn parse_host_port(rest: &str) -> Result<(String, Option<u16>)> {
        // Handle IPv6 addresses in brackets
        if rest.starts_with('[') {
            let bracket_end = rest.find(']').ok_or_else(|| {
                CdpServerError::InvalidOrigin("Invalid IPv6 address format".to_string())
            })?;

            let host = &rest[1..bracket_end];
            let after_bracket = &rest[bracket_end + 1..];

            let port = if after_bracket.starts_with(':') {
                let port_str = &after_bracket[1..];
                Some(port_str.parse::<u16>().map_err(|_| {
                    CdpServerError::InvalidOrigin("Invalid port number".to_string())
                })?)
            } else if after_bracket.is_empty() {
                None
            } else {
                return Err(CdpServerError::InvalidOrigin(
                    "Invalid characters after IPv6 address".to_string(),
                ));
            };

            return Ok((format!("[{}]", host), port));
        }

        // Handle regular host:port or host
        let (host, port) = if let Some(colon_pos) = rest.rfind(':') {
            // Check if this could be an unbracketed IPv6 address
            if rest.matches(':').count() > 1 {
                // Multiple colons - likely IPv6 without brackets (invalid but handle gracefully)
                (rest.to_string(), None)
            } else {
                let host = &rest[..colon_pos];
                let port_str = &rest[colon_pos + 1..];
                let port = port_str.parse::<u16>().map_err(|_| {
                    CdpServerError::InvalidOrigin("Invalid port number".to_string())
                })?;
                (host.to_string(), Some(port))
            }
        } else {
            (rest.to_string(), None)
        };

        // Validate host is not empty
        if host.is_empty() {
            return Err(CdpServerError::InvalidOrigin(
                "Host cannot be empty".to_string(),
            ));
        }

        Ok((host, port))
    }

    /// Check if a host is localhost
    fn is_localhost(host: &str) -> bool {
        let host_lower = host.to_lowercase();
        host_lower == "localhost"
            || host_lower == "127.0.0.1"
            || host_lower == "[::1]"
            || host_lower == "::1"
    }

    /// Check if a string is a bracketed IPv6 address
    fn is_ipv6_bracket(host: &str) -> bool {
        host.starts_with('[') && host.ends_with(']')
    }

    /// Check if the origin matches any allowed pattern
    fn check_allowed_patterns(&self, origin: &str) -> Result<()> {
        for pattern in &self.config.allowed_origins {
            if self.matches_pattern(origin, pattern) {
                return Ok(());
            }
        }

        Err(CdpServerError::InvalidOrigin(format!(
            "Origin '{}' is not in the allowed list",
            origin
        )))
    }

    /// Check if an origin matches a pattern
    ///
    /// Supports:
    /// - Exact match: "http://example.com"
    /// - Wildcard port: "http://localhost:*"
    /// - Full wildcard: "*"
    /// - Subdomain wildcard: "https://*.example.com"
    fn matches_pattern(&self, origin: &str, pattern: &str) -> bool {
        // Full wildcard
        if pattern == "*" {
            return true;
        }

        // Handle port wildcard (e.g., "http://localhost:*")
        if pattern.ends_with(":*") {
            let prefix = &pattern[..pattern.len() - 1]; // Remove the *
            if let Some(colon_pos) = origin.rfind(':') {
                let origin_prefix = &origin[..=colon_pos];
                return origin_prefix == prefix;
            }
            return false;
        }

        // Handle subdomain wildcard (e.g., "https://*.example.com")
        if pattern.contains("://*.") {
            if let (Some(pattern_scheme_end), Some(origin_scheme_end)) =
                (pattern.find("://"), origin.find("://"))
            {
                let pattern_scheme = &pattern[..pattern_scheme_end];
                let origin_scheme = &origin[..origin_scheme_end];

                if pattern_scheme != origin_scheme {
                    return false;
                }

                let pattern_host = &pattern[pattern_scheme_end + 5..]; // Skip "://*."
                let origin_host = &origin[origin_scheme_end + 3..];

                // Check if origin host ends with the pattern suffix
                return origin_host.ends_with(pattern_host)
                    || origin_host == &pattern_host[..pattern_host.len()];
            }
            return false;
        }

        // Exact match
        origin == pattern
    }
}

/// Parsed origin components
#[derive(Debug)]
#[allow(dead_code)]
struct ParsedOrigin {
    scheme: String,
    host: String,
    port: Option<u16>,
    is_localhost: bool,
    is_ip_address: bool,
}

/// Validate origin header against allowed origins list
/// This is the simple public API that wraps OriginValidator
pub fn validate_origin(origin: &str, allowed_origins: &[String]) -> bool {
    let config = OriginValidatorConfig {
        allowed_origins: allowed_origins.to_vec(),
        allow_null_origin: false,
        require_https: false,
        allow_localhost: true,
        allow_ip_addresses: true,
    };

    let validator = OriginValidator::new(config);
    validator.validate(origin).is_ok()
}

/// Extended validation result with details
#[derive(Debug, Clone)]
pub struct OriginValidationResult {
    /// Whether the origin is valid
    pub is_valid: bool,
    /// The validated origin (normalized)
    pub origin: String,
    /// Error message if invalid
    pub error: Option<String>,
    /// Whether the origin is localhost
    pub is_localhost: bool,
    /// Whether the origin uses HTTPS
    pub is_secure: bool,
}

/// Validate origin with detailed result
pub fn validate_origin_detailed(origin: &str, config: &OriginValidatorConfig) -> OriginValidationResult {
    let validator = OriginValidator::new(config.clone());

    match validator.validate(origin) {
        Ok(()) => {
            let is_localhost = OriginValidator::is_localhost(origin);
            let is_secure = origin.starts_with("https://") || origin.starts_with("wss://");

            OriginValidationResult {
                is_valid: true,
                origin: origin.to_string(),
                error: None,
                is_localhost,
                is_secure,
            }
        }
        Err(e) => OriginValidationResult {
            is_valid: false,
            origin: origin.to_string(),
            error: Some(e.to_string()),
            is_localhost: false,
            is_secure: false,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================
    // Basic Origin Validation Tests
    // =========================================

    #[test]
    fn test_validate_origin_exact_match() {
        let origins = vec!["http://localhost:3000".to_string()];
        assert!(validate_origin("http://localhost:3000", &origins));
        assert!(!validate_origin("http://localhost:4000", &origins));
    }

    #[test]
    fn test_validate_origin_wildcard_port() {
        let origins = vec!["http://localhost:*".to_string()];
        assert!(validate_origin("http://localhost:3000", &origins));
        assert!(validate_origin("http://localhost:9222", &origins));
        assert!(validate_origin("http://localhost:80", &origins));
        assert!(!validate_origin("http://example.com:3000", &origins));
    }

    #[test]
    fn test_validate_origin_full_wildcard() {
        let origins = vec!["*".to_string()];
        assert!(validate_origin("http://localhost:3000", &origins));
        assert!(validate_origin("https://example.com", &origins));
        assert!(validate_origin("http://192.168.1.1:8080", &origins));
    }

    #[test]
    fn test_validate_origin_subdomain_wildcard() {
        let origins = vec!["https://*.example.com".to_string()];
        let config = OriginValidatorConfig {
            allowed_origins: origins,
            ..Default::default()
        };
        let validator = OriginValidator::new(config);

        assert!(validator.validate("https://api.example.com").is_ok());
        assert!(validator.validate("https://dev.example.com").is_ok());
        assert!(validator.validate("http://api.example.com").is_err()); // Wrong scheme
    }

    #[test]
    fn test_validate_origin_multiple_patterns() {
        let origins = vec![
            "http://localhost:*".to_string(),
            "https://example.com".to_string(),
            "https://api.example.com".to_string(),
        ];

        assert!(validate_origin("http://localhost:3000", &origins));
        assert!(validate_origin("https://example.com", &origins));
        assert!(validate_origin("https://api.example.com", &origins));
        assert!(!validate_origin("https://other.com", &origins));
    }

    // =========================================
    // Null Origin Tests
    // =========================================

    #[test]
    fn test_null_origin_denied_by_default() {
        let config = OriginValidatorConfig::default();
        let validator = OriginValidator::new(config);

        assert!(validator.validate("null").is_err());
    }

    #[test]
    fn test_null_origin_allowed_when_configured() {
        let config = OriginValidatorConfig {
            allow_null_origin: true,
            ..Default::default()
        };
        let validator = OriginValidator::new(config);

        assert!(validator.validate("null").is_ok());
    }

    // =========================================
    // Localhost Tests
    // =========================================

    #[test]
    fn test_localhost_variations() {
        let origins = vec!["http://localhost:*".to_string(), "http://127.0.0.1:*".to_string()];

        assert!(validate_origin("http://localhost:3000", &origins));
        assert!(validate_origin("http://127.0.0.1:3000", &origins));
    }

    #[test]
    fn test_localhost_denied_when_configured() {
        let config = OriginValidatorConfig {
            allowed_origins: vec!["*".to_string()],
            allow_localhost: false,
            ..Default::default()
        };
        let validator = OriginValidator::new(config);

        assert!(validator.validate("http://localhost:3000").is_err());
        assert!(validator.validate("http://127.0.0.1:3000").is_err());
    }

    // =========================================
    // IP Address Tests
    // =========================================

    #[test]
    fn test_ipv4_address() {
        let origins = vec!["http://192.168.1.1:*".to_string()];
        assert!(validate_origin("http://192.168.1.1:8080", &origins));
    }

    #[test]
    fn test_ipv6_address() {
        let config = OriginValidatorConfig {
            allowed_origins: vec!["http://[::1]:*".to_string()],
            ..Default::default()
        };
        let validator = OriginValidator::new(config);

        assert!(validator.validate("http://[::1]:3000").is_ok());
    }

    #[test]
    fn test_ip_addresses_denied_when_configured() {
        let config = OriginValidatorConfig {
            allowed_origins: vec!["*".to_string()],
            allow_ip_addresses: false,
            allow_localhost: true,
            ..Default::default()
        };
        let validator = OriginValidator::new(config);

        // Localhost should still work (it's special-cased)
        assert!(validator.validate("http://localhost:3000").is_ok());
        // But IP addresses should fail
        assert!(validator.validate("http://192.168.1.1:3000").is_err());
    }

    // =========================================
    // HTTPS Requirement Tests
    // =========================================

    #[test]
    fn test_https_required() {
        let config = OriginValidatorConfig {
            allowed_origins: vec!["*".to_string()],
            require_https: true,
            ..Default::default()
        };
        let validator = OriginValidator::new(config);

        // Localhost should work with HTTP even when HTTPS is required
        assert!(validator.validate("http://localhost:3000").is_ok());

        // Non-localhost should require HTTPS
        assert!(validator.validate("http://example.com").is_err());
        assert!(validator.validate("https://example.com").is_ok());
    }

    // =========================================
    // Invalid Origin Format Tests
    // =========================================

    #[test]
    fn test_invalid_origin_no_scheme() {
        let config = OriginValidatorConfig::default();
        let validator = OriginValidator::new(config);

        assert!(validator.validate("localhost:3000").is_err());
        assert!(validator.validate("example.com").is_err());
    }

    #[test]
    fn test_invalid_origin_bad_scheme() {
        let config = OriginValidatorConfig::default();
        let validator = OriginValidator::new(config);

        assert!(validator.validate("ftp://localhost:3000").is_err());
        assert!(validator.validate("file://path/to/file").is_err());
    }

    #[test]
    fn test_invalid_origin_empty_host() {
        let config = OriginValidatorConfig::default();
        let validator = OriginValidator::new(config);

        assert!(validator.validate("http://:3000").is_err());
    }

    #[test]
    fn test_invalid_port() {
        let config = OriginValidatorConfig::default();
        let validator = OriginValidator::new(config);

        assert!(validator.validate("http://localhost:abc").is_err());
        assert!(validator.validate("http://localhost:99999").is_err());
    }

    // =========================================
    // Permissive Mode Tests
    // =========================================

    #[test]
    fn test_permissive_validator() {
        let validator = OriginValidator::permissive();

        assert!(validator.validate("http://localhost:3000").is_ok());
        assert!(validator.validate("https://example.com").is_ok());
        assert!(validator.validate("http://192.168.1.1:8080").is_ok());
        assert!(validator.validate("null").is_ok());
    }

    // =========================================
    // Detailed Validation Tests
    // =========================================

    #[test]
    fn test_validate_origin_detailed_valid() {
        let config = OriginValidatorConfig {
            allowed_origins: vec!["https://example.com".to_string()],
            ..Default::default()
        };

        let result = validate_origin_detailed("https://example.com", &config);

        assert!(result.is_valid);
        assert!(result.is_secure);
        assert!(!result.is_localhost);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_validate_origin_detailed_invalid() {
        let config = OriginValidatorConfig::default();

        let result = validate_origin_detailed("https://notallowed.com", &config);

        assert!(!result.is_valid);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_validate_origin_detailed_localhost() {
        let config = OriginValidatorConfig::default();

        let result = validate_origin_detailed("http://localhost:3000", &config);

        assert!(result.is_valid);
        assert!(!result.is_secure);
    }

    // =========================================
    // Edge Cases
    // =========================================

    #[test]
    fn test_case_sensitivity() {
        let origins = vec!["http://localhost:3000".to_string()];

        // Origin validation should be case-sensitive for the scheme
        assert!(validate_origin("http://localhost:3000", &origins));
        // But we should handle uppercase gracefully (most browsers lowercase the host)
    }

    #[test]
    fn test_empty_allowed_origins() {
        let origins: Vec<String> = vec![];

        assert!(!validate_origin("http://localhost:3000", &origins));
    }

    #[test]
    fn test_websocket_schemes() {
        let config = OriginValidatorConfig {
            allowed_origins: vec!["ws://localhost:*".to_string(), "wss://example.com".to_string()],
            ..Default::default()
        };
        let validator = OriginValidator::new(config);

        assert!(validator.validate("ws://localhost:3000").is_ok());
        assert!(validator.validate("wss://example.com").is_ok());
    }
}
