//! Inspector Bridges for Chrome DevTools Protocol
//!
//! This crate provides bridge implementations between the Chrome DevTools Protocol
//! and browser internals for DOM, render tree, JavaScript debugging, elements,
//! layout, accessibility, and storage inspection.
//!
//! ## Features
//!
//! - **DOM Inspector Bridge** (FEAT-017): Bridge between CDP DOM domain and browser DOM
//!   - Node tree traversal
//!   - Node selection and highlighting
//!   - DOM mutation tracking
//!   - Node search functionality
//!
//! - **Render Inspector Bridge** (FEAT-020): Bridge for render tree inspection
//!   - Box model inspection
//!   - Computed styles access
//!   - Layer tree representation
//!
//! - **JavaScript Debug Bridge** (FEAT-019): Bridge between CDP Debugger and JS engine
//!   - Script source management
//!   - Breakpoint coordination
//!   - Step execution control
//!   - Call stack management
//!   - Scope chain access
//!
//! - **Source Map Support** (FEAT-028): Parse and apply source maps for debugging
//!   - Source map parsing (JSON format, VLQ decoding)
//!   - Original position lookup
//!   - Generated position lookup
//!   - Source content resolution
//!   - Inline source map support (data URLs)
//!
//! - **Storage Bridge** (FEAT-021): Bridge for browser storage access
//!   - Local/Session storage access
//!   - IndexedDB inspection
//!   - Cookie management
//!
//! - **Elements Inspector** (FEAT-022): DOM element inspection and editing
//!   - DOM tree view
//!   - Attribute editing
//!   - Styles panel integration
//!
//! - **Layout Inspector** (FEAT-023): CSS layout and flexbox/grid inspection
//!   - Flexbox overlay
//!   - Grid overlay
//!   - Box model visualization
//!
//! - **Accessibility Inspector** (FEAT-024): Accessibility tree inspection
//!   - Accessibility tree inspection
//!   - ARIA attributes inspection
//!   - Contrast ratio checking
//!
//! ## Usage
//!
//! ```rust
//! use inspector_bridges::{DomInspectorBridge, RenderInspectorBridge, JsDebugBridge};
//! use protocol_handler::DomainHandler;
//!
//! #[tokio::main]
//! async fn main() {
//!     // Create DOM inspector bridge
//!     let dom_bridge = DomInspectorBridge::new();
//!
//!     // Create Render inspector bridge
//!     let render_bridge = RenderInspectorBridge::new();
//!
//!     // Create JavaScript Debug bridge
//!     let debug_bridge = JsDebugBridge::new();
//!
//!     // Handle CDP methods
//!     let doc = dom_bridge.handle_method("getDocument", None).await;
//!     let layers = render_bridge.handle_method("getLayerTree", None).await;
//!     let _ = debug_bridge.handle_method("enable", None).await;
//! }
//! ```
//!
//! ## Architecture
//!
//! All bridges implement the `DomainHandler` trait from `protocol_handler`,
//! allowing them to be registered with the protocol handler for processing
//! CDP requests.
//!
//! For testing purposes, a `MockBrowser` is used to simulate browser internals.
//! In production, the bridges would connect to actual browser components.

mod accessibility_inspector;
mod dom_inspector_bridge;
mod elements_inspector;
mod js_debug_bridge;
mod layout_inspector;
mod mock_browser;
mod render_inspector_bridge;
mod storage_bridge;
pub mod source_map;
pub mod types;

pub use accessibility_inspector::AccessibilityInspector;
pub use dom_inspector_bridge::DomInspectorBridge;
pub use elements_inspector::ElementsInspector;
pub use js_debug_bridge::{
    BreakpointInfo, DebugEvent, JsDebugBridge, JsDebugBridgeError, PauseOnExceptionsMode,
    PauseState, PropertyInfo, ScriptInfo, StepAction,
};
pub use layout_inspector::LayoutInspector;
pub use mock_browser::MockBrowser;
pub use render_inspector_bridge::RenderInspectorBridge;
pub use source_map::{
    GeneratedLocation, Mapping, OriginalLocation, Position, RawSourceMap, SourceMap,
    SourceMapError,
};
pub use storage_bridge::StorageBridge;

// Re-export commonly used types
pub use types::{
    BoxModel, HighlightConfig, LayerInfo, MutationRecord, MutationType, SearchResult, RGBA,
};

// Re-export types from new modules
pub use accessibility_inspector::{AXNode, AXValue, AriaAttributesInfo, ContrastInfo};
pub use elements_inspector::{ElementInfo, ElementState, StyleModification};
pub use layout_inspector::{
    FlexContainerInfo, FlexItemInfo, FlexOverlayConfig, GridContainerInfo, GridItemInfo,
    GridOverlayConfig, LayoutOverlayState,
};
pub use storage_bridge::{
    Cookie, CookieSameSite, DatabaseInfo, IndexInfo, ObjectStoreInfo, StorageAreaType,
    StorageBridgeState, StorageItem,
};

#[cfg(test)]
mod tests {
    use super::*;
    use protocol_handler::DomainHandler;
    use serde_json::json;

    #[tokio::test]
    async fn test_dom_inspector_bridge_creation() {
        let bridge = DomInspectorBridge::new();
        assert_eq!(bridge.name(), "DOMInspector");
    }

    #[tokio::test]
    async fn test_render_inspector_bridge_creation() {
        let bridge = RenderInspectorBridge::new();
        assert_eq!(bridge.name(), "RenderInspector");
    }

    #[tokio::test]
    async fn test_dom_get_document() {
        let bridge = DomInspectorBridge::new();
        let result = bridge.handle_method("getDocument", None).await;

        assert!(result.is_ok());
        let value = result.unwrap();
        assert!(value["root"].is_object());
        assert_eq!(value["root"]["nodeId"], 1);
    }

    #[tokio::test]
    async fn test_dom_query_selector() {
        let bridge = DomInspectorBridge::new();
        let params = json!({
            "nodeId": 1,
            "selector": "#container"
        });

        let result = bridge.handle_method("querySelector", Some(params)).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        assert_eq!(value["nodeId"], 6);
    }

    #[tokio::test]
    async fn test_dom_highlight_node() {
        let bridge = DomInspectorBridge::new();
        let params = json!({
            "highlightConfig": {
                "showInfo": true
            },
            "nodeId": 6
        });

        let result = bridge.handle_method("highlightNode", Some(params)).await;
        assert!(result.is_ok());

        let state = bridge.get_selection_state().await;
        assert_eq!(
            state.highlighted_node,
            Some(cdp_types::domains::dom::NodeId(6))
        );
    }

    #[tokio::test]
    async fn test_dom_search() {
        let bridge = DomInspectorBridge::new();

        // Perform search
        let search_params = json!({
            "query": "div",
            "includeUserAgentShadowDOM": false
        });
        let result = bridge
            .handle_method("performSearch", Some(search_params))
            .await;
        assert!(result.is_ok());

        let search_result = result.unwrap();
        assert!(search_result["searchId"].is_string());
        assert!(search_result["resultCount"].as_u64().unwrap() > 0);
    }

    #[tokio::test]
    async fn test_render_get_box_model() {
        let bridge = RenderInspectorBridge::new();
        let params = json!({ "nodeId": 6 });

        let result = bridge.handle_method("getBoxModel", Some(params)).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        assert!(value["model"]["content"].is_array());
        assert!(value["model"]["width"].as_f64().unwrap() > 0.0);
    }

    #[tokio::test]
    async fn test_render_get_computed_style() {
        let bridge = RenderInspectorBridge::new();
        let params = json!({ "nodeId": 6 });

        let result = bridge
            .handle_method("getComputedStyleForNode", Some(params))
            .await;
        assert!(result.is_ok());

        let value = result.unwrap();
        let styles = value["computedStyle"].as_array().unwrap();
        assert!(!styles.is_empty());
    }

    #[tokio::test]
    async fn test_render_get_layer_tree() {
        let bridge = RenderInspectorBridge::new();

        let result = bridge.handle_method("getLayerTree", None).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        let layers = value["layers"].as_array().unwrap();
        assert!(!layers.is_empty());
        assert_eq!(layers[0]["layerId"], "root-layer");
    }

    #[tokio::test]
    async fn test_shared_browser_instance() {
        use std::sync::Arc;

        let browser = Arc::new(MockBrowser::new());
        let dom_bridge = DomInspectorBridge::with_browser(browser.clone());
        let render_bridge = RenderInspectorBridge::with_browser(browser);

        // Both bridges should see the same DOM
        let dom_doc = dom_bridge.handle_method("getDocument", None).await.unwrap();
        let render_box = render_bridge
            .handle_method("getBoxModel", Some(json!({"nodeId": 6})))
            .await
            .unwrap();

        assert_eq!(dom_doc["root"]["nodeId"], 1);
        assert!(render_box["model"]["width"].as_f64().unwrap() > 0.0);
    }

    #[tokio::test]
    async fn test_mutation_subscription() {
        let bridge = DomInspectorBridge::new();
        let mut receiver = bridge.subscribe_mutations();

        // Set an attribute to trigger a mutation
        let params = json!({
            "nodeId": 6,
            "name": "data-mutation-test",
            "value": "test-value"
        });
        bridge
            .handle_method("setAttributeValue", Some(params))
            .await
            .unwrap();

        // Should receive mutation event
        let mutation =
            tokio::time::timeout(std::time::Duration::from_millis(100), receiver.recv()).await;

        assert!(mutation.is_ok());
        let record = mutation.unwrap().unwrap();
        assert_eq!(
            record.attribute_name,
            Some("data-mutation-test".to_string())
        );
    }

    #[tokio::test]
    async fn test_types_serialization() {
        let config = HighlightConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("showInfo"));

        let color = RGBA::new(255, 0, 0, 1.0);
        let json = serde_json::to_string(&color).unwrap();
        assert!(json.contains("\"r\":255"));
    }
}
