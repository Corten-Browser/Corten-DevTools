//! Unit tests for origin validation module

use cdp_server::{
    validate_origin_detailed, OriginValidationResult, OriginValidator, OriginValidatorConfig,
};

// =========================================
// OriginValidator Comprehensive Tests
// =========================================

mod origin_validator_basic {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = OriginValidatorConfig::default();
        assert!(!config.allow_null_origin);
        assert!(!config.require_https);
        assert!(config.allow_localhost);
        assert!(config.allow_ip_addresses);
    }

    #[test]
    fn test_validator_with_default_config() {
        let validator = OriginValidator::new(OriginValidatorConfig::default());
        assert!(validator.validate("http://localhost:3000").is_ok());
    }

    #[test]
    fn test_permissive_validator_allows_all() {
        let validator = OriginValidator::permissive();
        assert!(validator.validate("http://localhost:3000").is_ok());
        assert!(validator.validate("https://example.com").is_ok());
        assert!(validator.validate("http://192.168.1.1:8080").is_ok());
        assert!(validator.validate("null").is_ok());
    }
}

mod origin_format_validation {
    use super::*;

    #[test]
    fn test_valid_http_origin() {
        let config = OriginValidatorConfig {
            allowed_origins: vec!["http://example.com".to_string()],
            ..Default::default()
        };
        let validator = OriginValidator::new(config);
        assert!(validator.validate("http://example.com").is_ok());
    }

    #[test]
    fn test_valid_https_origin() {
        let config = OriginValidatorConfig {
            allowed_origins: vec!["https://example.com".to_string()],
            ..Default::default()
        };
        let validator = OriginValidator::new(config);
        assert!(validator.validate("https://example.com").is_ok());
    }

    #[test]
    fn test_valid_ws_origin() {
        let config = OriginValidatorConfig {
            allowed_origins: vec!["ws://localhost:9222".to_string()],
            ..Default::default()
        };
        let validator = OriginValidator::new(config);
        assert!(validator.validate("ws://localhost:9222").is_ok());
    }

    #[test]
    fn test_valid_wss_origin() {
        let config = OriginValidatorConfig {
            allowed_origins: vec!["wss://example.com".to_string()],
            ..Default::default()
        };
        let validator = OriginValidator::new(config);
        assert!(validator.validate("wss://example.com").is_ok());
    }

    #[test]
    fn test_invalid_ftp_scheme() {
        let config = OriginValidatorConfig {
            allowed_origins: vec!["*".to_string()],
            ..Default::default()
        };
        let validator = OriginValidator::new(config);
        assert!(validator.validate("ftp://example.com").is_err());
    }

    #[test]
    fn test_invalid_file_scheme() {
        let config = OriginValidatorConfig {
            allowed_origins: vec!["*".to_string()],
            ..Default::default()
        };
        let validator = OriginValidator::new(config);
        assert!(validator.validate("file:///path/to/file").is_err());
    }

    #[test]
    fn test_missing_scheme() {
        let config = OriginValidatorConfig {
            allowed_origins: vec!["*".to_string()],
            ..Default::default()
        };
        let validator = OriginValidator::new(config);
        assert!(validator.validate("localhost:3000").is_err());
        assert!(validator.validate("example.com").is_err());
    }
}

mod origin_port_handling {
    use super::*;

    #[test]
    fn test_origin_with_port() {
        let config = OriginValidatorConfig {
            allowed_origins: vec!["http://localhost:3000".to_string()],
            ..Default::default()
        };
        let validator = OriginValidator::new(config);
        assert!(validator.validate("http://localhost:3000").is_ok());
        assert!(validator.validate("http://localhost:4000").is_err());
    }

    #[test]
    fn test_origin_without_port() {
        let config = OriginValidatorConfig {
            allowed_origins: vec!["http://example.com".to_string()],
            ..Default::default()
        };
        let validator = OriginValidator::new(config);
        assert!(validator.validate("http://example.com").is_ok());
    }

    #[test]
    fn test_wildcard_port() {
        let config = OriginValidatorConfig {
            allowed_origins: vec!["http://localhost:*".to_string()],
            ..Default::default()
        };
        let validator = OriginValidator::new(config);
        assert!(validator.validate("http://localhost:1").is_ok());
        assert!(validator.validate("http://localhost:80").is_ok());
        assert!(validator.validate("http://localhost:443").is_ok());
        assert!(validator.validate("http://localhost:8080").is_ok());
        assert!(validator.validate("http://localhost:9222").is_ok());
        assert!(validator.validate("http://localhost:65535").is_ok());
    }

    #[test]
    fn test_invalid_port() {
        let config = OriginValidatorConfig {
            allowed_origins: vec!["*".to_string()],
            ..Default::default()
        };
        let validator = OriginValidator::new(config);
        assert!(validator.validate("http://localhost:abc").is_err());
        assert!(validator.validate("http://localhost:99999").is_err());
    }
}

mod localhost_handling {
    use super::*;

    #[test]
    fn test_localhost_variations() {
        let config = OriginValidatorConfig {
            allowed_origins: vec![
                "http://localhost:*".to_string(),
                "http://127.0.0.1:*".to_string(),
            ],
            ..Default::default()
        };
        let validator = OriginValidator::new(config);

        assert!(validator.validate("http://localhost:3000").is_ok());
        assert!(validator.validate("http://127.0.0.1:3000").is_ok());
    }

    #[test]
    fn test_localhost_ipv6() {
        let config = OriginValidatorConfig {
            allowed_origins: vec!["http://[::1]:*".to_string()],
            ..Default::default()
        };
        let validator = OriginValidator::new(config);

        assert!(validator.validate("http://[::1]:3000").is_ok());
    }

    #[test]
    fn test_localhost_denied_when_disabled() {
        let config = OriginValidatorConfig {
            allowed_origins: vec!["*".to_string()],
            allow_localhost: false,
            ..Default::default()
        };
        let validator = OriginValidator::new(config);

        assert!(validator.validate("http://localhost:3000").is_err());
        assert!(validator.validate("http://127.0.0.1:3000").is_err());
    }
}

mod ip_address_handling {
    use super::*;

    #[test]
    fn test_ipv4_address() {
        let config = OriginValidatorConfig {
            allowed_origins: vec!["http://192.168.1.1:*".to_string()],
            ..Default::default()
        };
        let validator = OriginValidator::new(config);

        assert!(validator.validate("http://192.168.1.1:8080").is_ok());
    }

    #[test]
    fn test_ipv6_address_bracketed() {
        let config = OriginValidatorConfig {
            allowed_origins: vec!["http://[2001:db8::1]:*".to_string()],
            ..Default::default()
        };
        let validator = OriginValidator::new(config);

        assert!(validator.validate("http://[2001:db8::1]:8080").is_ok());
    }

    #[test]
    fn test_ip_addresses_denied_when_disabled() {
        let config = OriginValidatorConfig {
            allowed_origins: vec!["*".to_string()],
            allow_ip_addresses: false,
            allow_localhost: true,
            ..Default::default()
        };
        let validator = OriginValidator::new(config);

        // Localhost should still work
        assert!(validator.validate("http://localhost:3000").is_ok());
        // IP addresses should fail
        assert!(validator.validate("http://192.168.1.1:3000").is_err());
    }
}

mod https_enforcement {
    use super::*;

    #[test]
    fn test_https_required_for_non_localhost() {
        let config = OriginValidatorConfig {
            allowed_origins: vec!["*".to_string()],
            require_https: true,
            ..Default::default()
        };
        let validator = OriginValidator::new(config);

        // Localhost can use HTTP
        assert!(validator.validate("http://localhost:3000").is_ok());
        // Non-localhost must use HTTPS
        assert!(validator.validate("http://example.com").is_err());
        assert!(validator.validate("https://example.com").is_ok());
    }

    #[test]
    fn test_wss_satisfies_https_requirement() {
        let config = OriginValidatorConfig {
            allowed_origins: vec!["wss://example.com".to_string()],
            require_https: true,
            ..Default::default()
        };
        let validator = OriginValidator::new(config);

        assert!(validator.validate("wss://example.com").is_ok());
    }
}

mod null_origin_handling {
    use super::*;

    #[test]
    fn test_null_origin_denied_by_default() {
        let config = OriginValidatorConfig::default();
        let validator = OriginValidator::new(config);

        assert!(validator.validate("null").is_err());
    }

    #[test]
    fn test_null_origin_allowed_when_enabled() {
        let config = OriginValidatorConfig {
            allow_null_origin: true,
            ..Default::default()
        };
        let validator = OriginValidator::new(config);

        assert!(validator.validate("null").is_ok());
    }
}

mod pattern_matching {
    use super::*;

    #[test]
    fn test_exact_match() {
        let config = OriginValidatorConfig {
            allowed_origins: vec!["http://example.com:8080".to_string()],
            ..Default::default()
        };
        let validator = OriginValidator::new(config);

        assert!(validator.validate("http://example.com:8080").is_ok());
        assert!(validator.validate("http://example.com:9000").is_err());
    }

    #[test]
    fn test_wildcard_match() {
        let config = OriginValidatorConfig {
            allowed_origins: vec!["*".to_string()],
            ..Default::default()
        };
        let validator = OriginValidator::new(config);

        assert!(validator.validate("http://localhost:3000").is_ok());
        assert!(validator.validate("https://example.com").is_ok());
    }

    #[test]
    fn test_subdomain_wildcard() {
        let config = OriginValidatorConfig {
            allowed_origins: vec!["https://*.example.com".to_string()],
            ..Default::default()
        };
        let validator = OriginValidator::new(config);

        assert!(validator.validate("https://api.example.com").is_ok());
        assert!(validator.validate("https://dev.example.com").is_ok());
        // Wrong scheme
        assert!(validator.validate("http://api.example.com").is_err());
    }

    #[test]
    fn test_multiple_patterns() {
        let config = OriginValidatorConfig {
            allowed_origins: vec![
                "http://localhost:*".to_string(),
                "https://example.com".to_string(),
                "https://api.example.com".to_string(),
            ],
            ..Default::default()
        };
        let validator = OriginValidator::new(config);

        assert!(validator.validate("http://localhost:3000").is_ok());
        assert!(validator.validate("https://example.com").is_ok());
        assert!(validator.validate("https://api.example.com").is_ok());
        assert!(validator.validate("https://other.com").is_err());
    }
}

mod detailed_validation {
    use super::*;

    #[test]
    fn test_detailed_result_valid() {
        let config = OriginValidatorConfig {
            allowed_origins: vec!["https://example.com".to_string()],
            ..Default::default()
        };

        let result = validate_origin_detailed("https://example.com", &config);

        assert!(result.is_valid);
        assert!(result.is_secure);
        assert!(!result.is_localhost);
        assert!(result.error.is_none());
        assert_eq!(result.origin, "https://example.com");
    }

    #[test]
    fn test_detailed_result_invalid() {
        let config = OriginValidatorConfig {
            allowed_origins: vec!["https://example.com".to_string()],
            ..Default::default()
        };

        let result = validate_origin_detailed("https://other.com", &config);

        assert!(!result.is_valid);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_detailed_result_localhost() {
        let config = OriginValidatorConfig::default();

        let result = validate_origin_detailed("http://localhost:3000", &config);

        assert!(result.is_valid);
        assert!(!result.is_secure); // HTTP is not secure
    }

    #[test]
    fn test_detailed_result_https_secure() {
        let config = OriginValidatorConfig {
            allowed_origins: vec!["https://example.com".to_string()],
            ..Default::default()
        };

        let result = validate_origin_detailed("https://example.com", &config);

        assert!(result.is_valid);
        assert!(result.is_secure);
    }
}

mod edge_cases {
    use super::*;

    #[test]
    fn test_empty_allowed_origins() {
        let config = OriginValidatorConfig {
            allowed_origins: vec![],
            ..Default::default()
        };
        let validator = OriginValidator::new(config);

        // Nothing should be allowed
        assert!(validator.validate("http://localhost:3000").is_err());
    }

    #[test]
    fn test_empty_host() {
        let config = OriginValidatorConfig {
            allowed_origins: vec!["*".to_string()],
            ..Default::default()
        };
        let validator = OriginValidator::new(config);

        assert!(validator.validate("http://:3000").is_err());
    }

    #[test]
    fn test_origin_with_path_rejected() {
        // Origins should not have paths - this tests the parsing
        let config = OriginValidatorConfig {
            allowed_origins: vec!["http://localhost:3000".to_string()],
            ..Default::default()
        };
        let validator = OriginValidator::new(config);

        // This won't match because of the path
        assert!(validator.validate("http://localhost:3000/path").is_err());
    }
}
