//! Source Map Support for debugging
//!
//! Implements FEAT-028: Source Map Support
//!
//! Features:
//! - Source map parsing (JSON format, VLQ decoding)
//! - Original position lookup
//! - Generated position lookup
//! - Source content resolution
//! - Inline source map support (data URLs)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Source map parsing and lookup errors
#[derive(Error, Debug)]
pub enum SourceMapError {
    /// Invalid source map JSON
    #[error("Invalid source map JSON: {0}")]
    InvalidJson(String),

    /// Invalid VLQ encoding
    #[error("Invalid VLQ encoding: {0}")]
    InvalidVlq(String),

    /// Invalid base64 encoding
    #[error("Invalid base64 encoding: {0}")]
    InvalidBase64(String),

    /// Source not found
    #[error("Source not found: {0}")]
    SourceNotFound(String),

    /// Invalid data URL
    #[error("Invalid data URL: {0}")]
    InvalidDataUrl(String),

    /// Mapping not found
    #[error("No mapping found for position")]
    MappingNotFound,
}

/// Result type for source map operations
pub type Result<T> = std::result::Result<T, SourceMapError>;

/// Represents a position in source code
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Position {
    /// Line number (0-based)
    pub line: u32,
    /// Column number (0-based)
    pub column: u32,
}

impl Position {
    /// Create a new position
    pub fn new(line: u32, column: u32) -> Self {
        Self { line, column }
    }
}

/// Represents a mapping from generated to original position
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mapping {
    /// Generated position
    pub generated: Position,
    /// Original position (if available)
    pub original: Option<Position>,
    /// Source file index (if available)
    pub source_index: Option<usize>,
    /// Name index (if available)
    pub name_index: Option<usize>,
}

/// Represents original location with source info
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OriginalLocation {
    /// Source file path/URL
    pub source: String,
    /// Position in original source
    pub position: Position,
    /// Original name (if available)
    pub name: Option<String>,
}

/// Represents generated location
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedLocation {
    /// Position in generated source
    pub position: Position,
}

/// Raw source map JSON structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawSourceMap {
    /// Source map version (should be 3)
    pub version: u32,
    /// Generated file name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// Source root prefix
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_root: Option<String>,
    /// List of original source files
    pub sources: Vec<String>,
    /// Optional source content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sources_content: Option<Vec<Option<String>>>,
    /// List of symbol names
    #[serde(default)]
    pub names: Vec<String>,
    /// VLQ-encoded mappings
    pub mappings: String,
}

/// Parsed source map with efficient lookup
#[derive(Debug, Clone)]
pub struct SourceMap {
    /// Source map version
    pub version: u32,
    /// Generated file name
    pub file: Option<String>,
    /// Source root prefix
    pub source_root: Option<String>,
    /// List of original source files
    pub sources: Vec<String>,
    /// Source content (indexed by source)
    pub sources_content: HashMap<usize, String>,
    /// List of symbol names
    pub names: Vec<String>,
    /// All parsed mappings
    mappings: Vec<Mapping>,
    /// Index for generated position lookup (line -> column -> mapping index)
    generated_index: HashMap<u32, Vec<(u32, usize)>>,
    /// Index for original position lookup (source_idx -> line -> column -> mapping index)
    original_index: HashMap<usize, HashMap<u32, Vec<(u32, usize)>>>,
}

impl SourceMap {
    /// Parse a source map from JSON string
    pub fn parse(json: &str) -> Result<Self> {
        let raw: RawSourceMap =
            serde_json::from_str(json).map_err(|e| SourceMapError::InvalidJson(e.to_string()))?;

        Self::from_raw(raw)
    }

    /// Parse a source map from an inline data URL
    pub fn parse_data_url(data_url: &str) -> Result<Self> {
        // Expected format: data:application/json;base64,<base64-encoded-json>
        let prefix = "data:application/json;base64,";
        if !data_url.starts_with(prefix) {
            // Try alternate format without explicit mime type
            let alt_prefix = "data:;base64,";
            if data_url.starts_with(alt_prefix) {
                let base64_data = &data_url[alt_prefix.len()..];
                let decoded = decode_base64(base64_data)?;
                let json = String::from_utf8(decoded)
                    .map_err(|e| SourceMapError::InvalidDataUrl(e.to_string()))?;
                return Self::parse(&json);
            }

            return Err(SourceMapError::InvalidDataUrl(format!(
                "Expected data:application/json;base64, prefix, got: {}",
                &data_url[..data_url.len().min(50)]
            )));
        }

        let base64_data = &data_url[prefix.len()..];
        let decoded = decode_base64(base64_data)?;
        let json = String::from_utf8(decoded)
            .map_err(|e| SourceMapError::InvalidDataUrl(e.to_string()))?;

        Self::parse(&json)
    }

    /// Extract source map URL from source file comment
    pub fn extract_url_from_source(source: &str) -> Option<String> {
        // Look for //# sourceMappingURL=<url> or /*# sourceMappingURL=<url> */
        for line in source.lines().rev() {
            let trimmed = line.trim();
            if let Some(url) = trimmed.strip_prefix("//# sourceMappingURL=") {
                return Some(url.trim().to_string());
            }
            if let Some(rest) = trimmed.strip_prefix("/*# sourceMappingURL=") {
                if let Some(url) = rest.strip_suffix("*/") {
                    return Some(url.trim().to_string());
                }
            }
            // Legacy format with @
            if let Some(url) = trimmed.strip_prefix("//@ sourceMappingURL=") {
                return Some(url.trim().to_string());
            }
        }
        None
    }

    /// Create source map from raw parsed JSON
    fn from_raw(raw: RawSourceMap) -> Result<Self> {
        let mappings = parse_vlq_mappings(&raw.mappings)?;

        // Build sources_content map
        let mut sources_content = HashMap::new();
        if let Some(contents) = raw.sources_content {
            for (idx, content) in contents.into_iter().enumerate() {
                if let Some(c) = content {
                    sources_content.insert(idx, c);
                }
            }
        }

        // Build generated index
        let mut generated_index: HashMap<u32, Vec<(u32, usize)>> = HashMap::new();
        for (idx, mapping) in mappings.iter().enumerate() {
            generated_index
                .entry(mapping.generated.line)
                .or_default()
                .push((mapping.generated.column, idx));
        }

        // Sort columns within each line
        for columns in generated_index.values_mut() {
            columns.sort_by_key(|(col, _)| *col);
        }

        // Build original index
        let mut original_index: HashMap<usize, HashMap<u32, Vec<(u32, usize)>>> = HashMap::new();
        for (idx, mapping) in mappings.iter().enumerate() {
            if let (Some(source_idx), Some(original)) = (mapping.source_index, mapping.original) {
                original_index
                    .entry(source_idx)
                    .or_default()
                    .entry(original.line)
                    .or_default()
                    .push((original.column, idx));
            }
        }

        // Sort columns within each line in original index
        for source_map in original_index.values_mut() {
            for columns in source_map.values_mut() {
                columns.sort_by_key(|(col, _)| *col);
            }
        }

        Ok(Self {
            version: raw.version,
            file: raw.file,
            source_root: raw.source_root,
            sources: raw.sources,
            sources_content,
            names: raw.names,
            mappings,
            generated_index,
            original_index,
        })
    }

    /// Look up original position from generated position
    pub fn original_position_for(&self, generated: Position) -> Result<OriginalLocation> {
        let columns = self
            .generated_index
            .get(&generated.line)
            .ok_or(SourceMapError::MappingNotFound)?;

        // Find the mapping with column <= generated.column (binary search)
        let mapping_idx = find_closest_mapping(columns, generated.column)
            .ok_or(SourceMapError::MappingNotFound)?;

        let mapping = &self.mappings[mapping_idx];
        let source_idx = mapping
            .source_index
            .ok_or(SourceMapError::MappingNotFound)?;
        let original = mapping.original.ok_or(SourceMapError::MappingNotFound)?;

        let source = self
            .sources
            .get(source_idx)
            .ok_or_else(|| SourceMapError::SourceNotFound(format!("index {}", source_idx)))?
            .clone();

        let full_source = if let Some(ref root) = self.source_root {
            format!("{}{}", root, source)
        } else {
            source
        };

        let name = mapping
            .name_index
            .and_then(|idx| self.names.get(idx))
            .cloned();

        Ok(OriginalLocation {
            source: full_source,
            position: original,
            name,
        })
    }

    /// Look up generated position from original position
    pub fn generated_position_for(
        &self,
        source: &str,
        original: Position,
    ) -> Result<GeneratedLocation> {
        // Find source index
        let source_idx = self.find_source_index(source)?;

        let line_map = self
            .original_index
            .get(&source_idx)
            .ok_or(SourceMapError::MappingNotFound)?;

        let columns = line_map
            .get(&original.line)
            .ok_or(SourceMapError::MappingNotFound)?;

        let mapping_idx = find_closest_mapping(columns, original.column)
            .ok_or(SourceMapError::MappingNotFound)?;

        let mapping = &self.mappings[mapping_idx];

        Ok(GeneratedLocation {
            position: mapping.generated,
        })
    }

    /// Get source content for a source file
    pub fn source_content(&self, source: &str) -> Option<&str> {
        let source_idx = self.find_source_index(source).ok()?;
        self.sources_content.get(&source_idx).map(|s| s.as_str())
    }

    /// Get all source files
    pub fn source_files(&self) -> &[String] {
        &self.sources
    }

    /// Get number of mappings
    pub fn mapping_count(&self) -> usize {
        self.mappings.len()
    }

    /// Find source index by name (with or without source root)
    fn find_source_index(&self, source: &str) -> Result<usize> {
        // Try exact match first
        if let Some(idx) = self.sources.iter().position(|s| s == source) {
            return Ok(idx);
        }

        // Try with source root stripped
        if let Some(ref root) = self.source_root {
            if let Some(stripped) = source.strip_prefix(root) {
                if let Some(idx) = self.sources.iter().position(|s| s == stripped) {
                    return Ok(idx);
                }
            }
        }

        // Try matching just the filename
        let source_name = source.rsplit('/').next().unwrap_or(source);
        if let Some(idx) = self
            .sources
            .iter()
            .position(|s| s.rsplit('/').next() == Some(source_name))
        {
            return Ok(idx);
        }

        Err(SourceMapError::SourceNotFound(source.to_string()))
    }
}

/// Find the mapping index with column closest to but not exceeding the target
fn find_closest_mapping(columns: &[(u32, usize)], target_column: u32) -> Option<usize> {
    if columns.is_empty() {
        return None;
    }

    // Binary search for the largest column <= target_column
    let mut left = 0;
    let mut right = columns.len();

    while left < right {
        let mid = (left + right) / 2;
        if columns[mid].0 <= target_column {
            left = mid + 1;
        } else {
            right = mid;
        }
    }

    // left - 1 is the index of the largest column <= target_column
    if left > 0 {
        Some(columns[left - 1].1)
    } else {
        // If target is less than all columns, use the first one
        Some(columns[0].1)
    }
}

/// Decode base64 to bytes
fn decode_base64(input: &str) -> Result<Vec<u8>> {
    const BASE64_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let input = input.trim_end_matches('=');
    let mut output = Vec::with_capacity(input.len() * 3 / 4);

    let mut buffer: u32 = 0;
    let mut bits_collected = 0;

    for c in input.bytes() {
        let value = match BASE64_CHARS.iter().position(|&b| b == c) {
            Some(v) => v as u32,
            None if c == b'-' => 62, // URL-safe base64
            None if c == b'_' => 63, // URL-safe base64
            None if c.is_ascii_whitespace() => continue,
            None => {
                return Err(SourceMapError::InvalidBase64(format!(
                    "Invalid character: {}",
                    c as char
                )))
            }
        };

        buffer = (buffer << 6) | value;
        bits_collected += 6;

        if bits_collected >= 8 {
            bits_collected -= 8;
            output.push((buffer >> bits_collected) as u8);
            buffer &= (1 << bits_collected) - 1;
        }
    }

    Ok(output)
}

/// Parse VLQ-encoded mappings string
fn parse_vlq_mappings(mappings: &str) -> Result<Vec<Mapping>> {
    let mut result = Vec::new();

    // State: previous values for delta encoding
    let mut prev_source: i64 = 0;
    let mut prev_orig_line: i64 = 0;
    let mut prev_orig_col: i64 = 0;
    let mut prev_name: i64 = 0;

    for (gen_line, line_mappings) in mappings.split(';').enumerate() {
        let mut prev_gen_col: i64 = 0; // Reset column at start of each line

        for segment in line_mappings.split(',') {
            if segment.is_empty() {
                continue;
            }

            let values = decode_vlq(segment)?;
            if values.is_empty() {
                continue;
            }

            // First value: generated column (delta from previous)
            prev_gen_col += values[0];

            let mut mapping = Mapping {
                generated: Position::new(gen_line as u32, prev_gen_col as u32),
                original: None,
                source_index: None,
                name_index: None,
            };

            if values.len() >= 4 {
                // Second value: source index (delta)
                prev_source += values[1];
                mapping.source_index = Some(prev_source as usize);

                // Third value: original line (delta)
                prev_orig_line += values[2];

                // Fourth value: original column (delta)
                prev_orig_col += values[3];

                mapping.original = Some(Position::new(prev_orig_line as u32, prev_orig_col as u32));

                if values.len() >= 5 {
                    // Fifth value: name index (delta)
                    prev_name += values[4];
                    mapping.name_index = Some(prev_name as usize);
                }
            }

            result.push(mapping);
        }
    }

    Ok(result)
}

/// Decode a VLQ-encoded segment into a list of integers
fn decode_vlq(segment: &str) -> Result<Vec<i64>> {
    const VLQ_CHARS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    const VLQ_CONTINUATION_BIT: u8 = 32; // 6th bit
    const VLQ_VALUE_MASK: u8 = 31; // Lower 5 bits

    let mut result = Vec::new();
    let mut value: i64 = 0;
    let mut shift = 0;

    for c in segment.chars() {
        let digit = VLQ_CHARS
            .find(c)
            .ok_or_else(|| SourceMapError::InvalidVlq(format!("Invalid VLQ character: {}", c)))?
            as u8;

        value += ((digit & VLQ_VALUE_MASK) as i64) << shift;
        shift += 5;

        if digit & VLQ_CONTINUATION_BIT == 0 {
            // Last digit for this value
            // Sign is stored in LSB
            let signed_value = if value & 1 == 1 {
                -(value >> 1)
            } else {
                value >> 1
            };

            result.push(signed_value);
            value = 0;
            shift = 0;
        }
    }

    if shift > 0 {
        return Err(SourceMapError::InvalidVlq(
            "Incomplete VLQ sequence".to_string(),
        ));
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_new() {
        let pos = Position::new(10, 5);
        assert_eq!(pos.line, 10);
        assert_eq!(pos.column, 5);
    }

    #[test]
    fn test_decode_vlq_simple() {
        // 'A' = 0
        let result = decode_vlq("A").unwrap();
        assert_eq!(result, vec![0]);

        // 'C' = 1
        let result = decode_vlq("C").unwrap();
        assert_eq!(result, vec![1]);

        // 'D' = -1
        let result = decode_vlq("D").unwrap();
        assert_eq!(result, vec![-1]);
    }

    #[test]
    fn test_decode_vlq_multi_digit() {
        // Test larger values that require continuation bits
        // 'gB' = 16 (continuation bit set, then final digit)
        let result = decode_vlq("gB").unwrap();
        assert_eq!(result, vec![16]);
    }

    #[test]
    fn test_decode_vlq_multiple_values() {
        // Multiple values in one segment
        let result = decode_vlq("AACA").unwrap();
        assert_eq!(result, vec![0, 0, 1, 0]);
    }

    #[test]
    fn test_decode_base64_simple() {
        let result = decode_base64("SGVsbG8=").unwrap();
        assert_eq!(result, b"Hello");

        let result = decode_base64("V29ybGQ=").unwrap();
        assert_eq!(result, b"World");
    }

    #[test]
    fn test_decode_base64_no_padding() {
        let result = decode_base64("SGVsbG8").unwrap();
        assert_eq!(result, b"Hello");
    }

    #[test]
    fn test_parse_simple_source_map() {
        let source_map_json = r#"{
            "version": 3,
            "file": "out.js",
            "sources": ["input.js"],
            "names": ["foo"],
            "mappings": "AAAA"
        }"#;

        let sm = SourceMap::parse(source_map_json).unwrap();
        assert_eq!(sm.version, 3);
        assert_eq!(sm.file, Some("out.js".to_string()));
        assert_eq!(sm.sources, vec!["input.js".to_string()]);
        assert_eq!(sm.names, vec!["foo".to_string()]);
    }

    #[test]
    fn test_parse_source_map_with_source_root() {
        let source_map_json = r#"{
            "version": 3,
            "sourceRoot": "src/",
            "sources": ["app.ts"],
            "names": [],
            "mappings": "AAAA"
        }"#;

        let sm = SourceMap::parse(source_map_json).unwrap();
        assert_eq!(sm.source_root, Some("src/".to_string()));
    }

    #[test]
    fn test_parse_source_map_with_sources_content() {
        let source_map_json = r#"{
            "version": 3,
            "sources": ["input.js"],
            "sourcesContent": ["const x = 1;"],
            "names": [],
            "mappings": "AAAA"
        }"#;

        let sm = SourceMap::parse(source_map_json).unwrap();
        let content = sm.source_content("input.js");
        assert_eq!(content, Some("const x = 1;"));
    }

    #[test]
    fn test_original_position_lookup() {
        // This source map maps generated position (0,0) to original position (0,0) in "input.js"
        let source_map_json = r#"{
            "version": 3,
            "sources": ["input.js"],
            "names": ["myVar"],
            "mappings": "AAAAA"
        }"#;

        let sm = SourceMap::parse(source_map_json).unwrap();
        let original = sm.original_position_for(Position::new(0, 0)).unwrap();

        assert_eq!(original.source, "input.js");
        assert_eq!(original.position.line, 0);
        assert_eq!(original.position.column, 0);
        assert_eq!(original.name, Some("myVar".to_string()));
    }

    #[test]
    fn test_generated_position_lookup() {
        let source_map_json = r#"{
            "version": 3,
            "sources": ["input.js"],
            "names": [],
            "mappings": "AAAA"
        }"#;

        let sm = SourceMap::parse(source_map_json).unwrap();
        let generated = sm
            .generated_position_for("input.js", Position::new(0, 0))
            .unwrap();

        assert_eq!(generated.position.line, 0);
        assert_eq!(generated.position.column, 0);
    }

    #[test]
    fn test_multi_line_mappings() {
        // Semicolons separate lines in generated code
        let source_map_json = r#"{
            "version": 3,
            "sources": ["input.js"],
            "names": [],
            "mappings": "AAAA;AACA;AACA"
        }"#;

        let sm = SourceMap::parse(source_map_json).unwrap();
        assert_eq!(sm.mapping_count(), 3);

        let orig0 = sm.original_position_for(Position::new(0, 0)).unwrap();
        assert_eq!(orig0.position.line, 0);

        let orig1 = sm.original_position_for(Position::new(1, 0)).unwrap();
        assert_eq!(orig1.position.line, 1);

        let orig2 = sm.original_position_for(Position::new(2, 0)).unwrap();
        assert_eq!(orig2.position.line, 2);
    }

    #[test]
    fn test_extract_url_single_line_comment() {
        let source = r#"
            function foo() {}
            //# sourceMappingURL=app.js.map
        "#;
        let url = SourceMap::extract_url_from_source(source);
        assert_eq!(url, Some("app.js.map".to_string()));
    }

    #[test]
    fn test_extract_url_multi_line_comment() {
        let source = r#"
            function foo() {}
            /*# sourceMappingURL=app.js.map */
        "#;
        let url = SourceMap::extract_url_from_source(source);
        assert_eq!(url, Some("app.js.map".to_string()));
    }

    #[test]
    fn test_extract_url_data_url() {
        let source = r#"
            function foo() {}
            //# sourceMappingURL=data:application/json;base64,eyJ2ZXJzaW9uIjozfQ==
        "#;
        let url = SourceMap::extract_url_from_source(source);
        assert!(url.is_some());
        assert!(url.unwrap().starts_with("data:"));
    }

    #[test]
    fn test_extract_url_legacy_format() {
        let source = r#"
            function foo() {}
            //@ sourceMappingURL=legacy.js.map
        "#;
        let url = SourceMap::extract_url_from_source(source);
        assert_eq!(url, Some("legacy.js.map".to_string()));
    }

    #[test]
    fn test_extract_url_no_url() {
        let source = r#"
            function foo() {}
            // No source map here
        "#;
        let url = SourceMap::extract_url_from_source(source);
        assert!(url.is_none());
    }

    #[test]
    fn test_parse_inline_source_map() {
        // Base64 of {"version":3,"sources":["a.js"],"names":[],"mappings":"AAAA"}
        let data_url =
            "data:application/json;base64,eyJ2ZXJzaW9uIjozLCJzb3VyY2VzIjpbImEuanMiXSwibmFtZXMiOltdLCJtYXBwaW5ncyI6IkFBQUEifQ==";
        let sm = SourceMap::parse_data_url(data_url).unwrap();

        assert_eq!(sm.version, 3);
        assert_eq!(sm.sources, vec!["a.js".to_string()]);
    }

    #[test]
    fn test_invalid_data_url() {
        let result = SourceMap::parse_data_url("not-a-data-url");
        assert!(result.is_err());
    }

    #[test]
    fn test_mapping_not_found() {
        let source_map_json = r#"{
            "version": 3,
            "sources": ["input.js"],
            "names": [],
            "mappings": ""
        }"#;

        let sm = SourceMap::parse(source_map_json).unwrap();
        let result = sm.original_position_for(Position::new(0, 0));
        assert!(result.is_err());
    }

    #[test]
    fn test_source_not_found() {
        let source_map_json = r#"{
            "version": 3,
            "sources": ["input.js"],
            "names": [],
            "mappings": "AAAA"
        }"#;

        let sm = SourceMap::parse(source_map_json).unwrap();
        let result = sm.generated_position_for("nonexistent.js", Position::new(0, 0));
        assert!(matches!(result, Err(SourceMapError::SourceNotFound(_))));
    }

    #[test]
    fn test_column_binary_search() {
        // Source map with multiple columns on the same line
        let source_map_json = r#"{
            "version": 3,
            "sources": ["input.js"],
            "names": [],
            "mappings": "AAAA,GAAG,QAAQ"
        }"#;

        let sm = SourceMap::parse(source_map_json).unwrap();

        // Column 0 should map to first segment
        let orig0 = sm.original_position_for(Position::new(0, 0)).unwrap();
        assert_eq!(orig0.position.column, 0);

        // Column 3 should map to second segment (column 3 in generated)
        let orig3 = sm.original_position_for(Position::new(0, 3)).unwrap();
        // Verify we got a valid mapping
        assert!(orig3.source == "input.js");

        // Column 10 should map to third segment
        let orig10 = sm.original_position_for(Position::new(0, 10)).unwrap();
        // Verify we got a valid mapping
        assert!(orig10.source == "input.js");
    }

    #[test]
    fn test_source_files() {
        let source_map_json = r#"{
            "version": 3,
            "sources": ["a.js", "b.js", "c.js"],
            "names": [],
            "mappings": ""
        }"#;

        let sm = SourceMap::parse(source_map_json).unwrap();
        assert_eq!(sm.source_files(), &["a.js", "b.js", "c.js"]);
    }

    #[test]
    fn test_negative_vlq_values() {
        // Test that negative values are handled correctly
        // 'D' = -1, 'F' = -2, 'H' = -3
        let result = decode_vlq("D").unwrap();
        assert_eq!(result, vec![-1]);

        let result = decode_vlq("F").unwrap();
        assert_eq!(result, vec![-2]);

        let result = decode_vlq("H").unwrap();
        assert_eq!(result, vec![-3]);
    }
}
