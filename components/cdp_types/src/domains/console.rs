// Console domain types

use serde::{Deserialize, Serialize};

/// Console message source
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ConsoleMessageSource {
    Xml,
    Javascript,
    Network,
    #[serde(rename = "console-api")]
    ConsoleApi,
    Storage,
    Appcache,
    Rendering,
    Security,
    Other,
    Deprecation,
    Worker,
    #[serde(rename = "console")]
    Console,
}

/// Console message level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ConsoleMessageLevel {
    Log,
    Warning,
    Error,
    Debug,
    Info,
}

/// Console message
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConsoleMessage {
    /// Message source
    pub source: ConsoleMessageSource,
    /// Message severity level
    pub level: ConsoleMessageLevel,
    /// Message text
    pub text: String,
    /// URL of the message origin
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Line number in the resource (0-based)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    /// Column number in the resource (0-based)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_console_message_source() {
        let source = ConsoleMessageSource::Console;
        let json = serde_json::to_string(&source).unwrap();
        assert_eq!(json, "\"console\"");
    }

    #[test]
    fn test_console_message_level() {
        let level = ConsoleMessageLevel::Error;
        let json = serde_json::to_string(&level).unwrap();
        assert_eq!(json, "\"error\"");
    }

    #[test]
    fn test_console_message() {
        let msg = ConsoleMessage {
            source: ConsoleMessageSource::Console,
            level: ConsoleMessageLevel::Log,
            text: "Hello".to_string(),
            url: Some("http://example.com".to_string()),
            line: Some(10),
            column: Some(5),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("Hello"));
    }
}
