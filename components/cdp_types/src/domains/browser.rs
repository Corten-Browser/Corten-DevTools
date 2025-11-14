// Browser domain types

use serde::{Deserialize, Serialize};

/// Response for Browser.getVersion
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GetVersionResponse {
    /// Protocol version
    pub protocol_version: String,
    /// Product name and version
    pub product: String,
    /// Product revision
    pub revision: String,
    /// User-Agent string
    pub user_agent: String,
    /// V8 or JavaScript engine version
    pub js_version: String,
}

/// Response for Browser.getBrowserCommandLine
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetBrowserCommandLineResponse {
    /// Command line arguments
    pub arguments: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_version_response() {
        let response = GetVersionResponse {
            protocol_version: "1.3".to_string(),
            product: "CortenBrowser/1.0".to_string(),
            revision: "abc123".to_string(),
            user_agent: "Mozilla/5.0".to_string(),
            js_version: "V8/11.0".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("protocolVersion"));
        assert!(json.contains("1.3"));
    }
}
