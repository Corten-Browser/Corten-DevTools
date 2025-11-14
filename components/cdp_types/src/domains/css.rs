// CSS domain types

use serde::{Deserialize, Serialize};

/// Stylesheet identifier
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct StyleSheetId(pub String);

/// Stylesheet origin
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StyleSheetOrigin {
    /// Author stylesheets
    Regular,
    /// User agent stylesheets
    UserAgent,
    /// User stylesheets
    User,
    /// Inspector stylesheets
    Inspector,
}

/// CSS property declaration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CSSProperty {
    /// Property name
    pub name: String,
    /// Property value
    pub value: String,
    /// Whether the property has !important annotation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub important: Option<bool>,
    /// Whether the property is implicit
    #[serde(skip_serializing_if = "Option::is_none")]
    pub implicit: Option<bool>,
    /// Full property text
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Whether the property syntax is valid
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parsed_ok: Option<bool>,
    /// Whether the property is disabled
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    /// Property range in the stylesheet
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<SourceRange>,
}

/// Text range in a source file
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SourceRange {
    /// Start line (0-based)
    pub start_line: u32,
    /// Start column (0-based)
    pub start_column: u32,
    /// End line (0-based)
    pub end_line: u32,
    /// End column (0-based)
    pub end_column: u32,
}

/// Selector text
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Value {
    /// Selector text
    pub text: String,
}

/// Selector list
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SelectorList {
    /// Individual selectors
    pub selectors: Vec<Value>,
    /// Full selector list text
    pub text: String,
}

/// CSS style declaration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CSSStyle {
    /// Parent stylesheet ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style_sheet_id: Option<StyleSheetId>,
    /// CSS properties
    pub css_properties: Vec<CSSProperty>,
    /// Shorthand entries
    pub short_hand_entries: Vec<ShorthandEntry>,
    /// Style text (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub css_text: Option<String>,
    /// Style range in the stylesheet
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<SourceRange>,
}

/// Shorthand property entry
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShorthandEntry {
    /// Shorthand name
    pub name: String,
    /// Shorthand value
    pub value: String,
    /// Whether the property has !important annotation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub important: Option<bool>,
}

/// CSS rule
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CSSRule {
    /// Parent stylesheet ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style_sheet_id: Option<StyleSheetId>,
    /// Selector list
    pub selector_list: SelectorList,
    /// Rule origin
    pub origin: StyleSheetOrigin,
    /// Associated style declaration
    pub style: CSSStyle,
}

/// Computed styles
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComputedStyles {
    /// Computed CSS properties
    pub properties: Vec<CSSProperty>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stylesheet_id() {
        let id = StyleSheetId("sheet-1".to_string());
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"sheet-1\"");
    }

    #[test]
    fn test_css_property() {
        let prop = CSSProperty {
            name: "color".to_string(),
            value: "red".to_string(),
            important: Some(false),
            implicit: Some(false),
            text: Some("color: red".to_string()),
            parsed_ok: Some(true),
            disabled: Some(false),
            range: None,
        };

        let json = serde_json::to_string(&prop).unwrap();
        assert!(json.contains("\"name\":\"color\""));
    }
}
