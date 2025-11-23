//! Unit tests for inspector bridges

use inspector_bridges::*;
use protocol_handler::DomainHandler;
use serde_json::json;

mod dom_inspector_tests {
    use super::*;
    use cdp_types::domains::dom::NodeId;

    #[tokio::test]
    async fn test_get_document_returns_root() {
        let bridge = DomInspectorBridge::new();
        let result = bridge.handle_method("getDocument", None).await;

        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value["root"]["nodeId"], 1);
        assert_eq!(value["root"]["nodeType"], 9); // Document type
    }

    #[tokio::test]
    async fn test_query_selector_finds_element_by_id() {
        let bridge = DomInspectorBridge::new();
        let params = json!({
            "nodeId": 1,
            "selector": "#container"
        });

        let result = bridge.handle_method("querySelector", Some(params)).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap()["nodeId"], 6);
    }

    #[tokio::test]
    async fn test_query_selector_finds_element_by_class() {
        let bridge = DomInspectorBridge::new();
        let params = json!({
            "nodeId": 1,
            "selector": ".wrapper"
        });

        let result = bridge.handle_method("querySelector", Some(params)).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap()["nodeId"], 6);
    }

    #[tokio::test]
    async fn test_query_selector_all_finds_multiple() {
        let bridge = DomInspectorBridge::new();
        let params = json!({
            "nodeId": 1,
            "selector": "div"
        });

        let result = bridge.handle_method("querySelectorAll", Some(params)).await;
        assert!(result.is_ok());

        let node_ids = result.unwrap()["nodeIds"].as_array().unwrap();
        assert!(node_ids.len() >= 2);
    }

    #[tokio::test]
    async fn test_set_attribute_adds_new_attribute() {
        let bridge = DomInspectorBridge::new();
        let params = json!({
            "nodeId": 6,
            "name": "data-new",
            "value": "new-value"
        });

        let result = bridge.handle_method("setAttributeValue", Some(params)).await;
        assert!(result.is_ok());

        // Verify the attribute was set
        let node = bridge.browser().get_node(NodeId(6)).unwrap();
        let attrs = node.attributes.unwrap();
        assert!(attrs.contains(&"data-new".to_string()));
        assert!(attrs.contains(&"new-value".to_string()));
    }

    #[tokio::test]
    async fn test_highlight_updates_selection_state() {
        let bridge = DomInspectorBridge::new();
        let params = json!({
            "highlightConfig": {
                "showInfo": true,
                "showRulers": true
            },
            "nodeId": 6
        });

        bridge.handle_method("highlightNode", Some(params)).await.unwrap();

        let state = bridge.get_selection_state().await;
        assert_eq!(state.highlighted_node, Some(NodeId(6)));
        assert!(state.highlight_config.show_info);
        assert!(state.highlight_config.show_rulers);
    }

    #[tokio::test]
    async fn test_hide_highlight_clears_state() {
        let bridge = DomInspectorBridge::new();

        // First highlight
        let params = json!({
            "highlightConfig": {},
            "nodeId": 6
        });
        bridge.handle_method("highlightNode", Some(params)).await.unwrap();

        // Then hide
        bridge.handle_method("hideHighlight", None).await.unwrap();

        let state = bridge.get_selection_state().await;
        assert!(state.highlighted_node.is_none());
    }

    #[tokio::test]
    async fn test_search_finds_text_content() {
        let bridge = DomInspectorBridge::new();
        let params = json!({
            "query": "Hello",
            "includeUserAgentShadowDOM": false
        });

        let result = bridge.handle_method("performSearch", Some(params)).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        assert!(value["resultCount"].as_u64().unwrap() > 0);
    }

    #[tokio::test]
    async fn test_search_results_workflow() {
        let bridge = DomInspectorBridge::new();

        // Search
        let search_result = bridge
            .handle_method("performSearch", Some(json!({
                "query": "div",
                "includeUserAgentShadowDOM": false
            })))
            .await
            .unwrap();

        let search_id = search_result["searchId"].as_str().unwrap();
        let count = search_result["resultCount"].as_u64().unwrap() as u32;

        // Get results
        let results = bridge
            .handle_method("getSearchResults", Some(json!({
                "searchId": search_id,
                "fromIndex": 0,
                "toIndex": count
            })))
            .await
            .unwrap();

        assert!(results["nodeIds"].as_array().unwrap().len() > 0);

        // Discard
        let discard_result = bridge
            .handle_method("discardSearchResults", Some(json!({
                "searchId": search_id
            })))
            .await;

        assert!(discard_result.is_ok());
    }

    #[tokio::test]
    async fn test_describe_node() {
        let bridge = DomInspectorBridge::new();
        let params = json!({
            "nodeId": 5,
            "depth": 2
        });

        let result = bridge.handle_method("describeNode", Some(params)).await;
        assert!(result.is_ok());

        let node = &result.unwrap()["node"];
        assert_eq!(node["nodeName"], "BODY");
    }

    #[tokio::test]
    async fn test_error_on_invalid_node() {
        let bridge = DomInspectorBridge::new();
        let params = json!({
            "nodeId": 99999,
            "selector": "div"
        });

        let result = bridge.handle_method("querySelector", Some(params)).await;
        assert!(result.is_err());
    }
}

mod render_inspector_tests {
    use super::*;

    #[tokio::test]
    async fn test_get_box_model_returns_all_boxes() {
        let bridge = RenderInspectorBridge::new();
        let params = json!({ "nodeId": 6 });

        let result = bridge.handle_method("getBoxModel", Some(params)).await;
        assert!(result.is_ok());

        let model = &result.unwrap()["model"];
        assert!(model["content"].is_array());
        assert!(model["padding"].is_array());
        assert!(model["border"].is_array());
        assert!(model["margin"].is_array());
        assert!(model["width"].as_f64().unwrap() > 0.0);
        assert!(model["height"].as_f64().unwrap() > 0.0);
    }

    #[tokio::test]
    async fn test_computed_styles_include_expected_properties() {
        let bridge = RenderInspectorBridge::new();
        let params = json!({ "nodeId": 6 });

        let result = bridge.handle_method("getComputedStyleForNode", Some(params)).await;
        assert!(result.is_ok());

        let styles = result.unwrap()["computedStyle"].as_array().unwrap();

        let property_names: Vec<&str> = styles
            .iter()
            .filter_map(|s| s["name"].as_str())
            .collect();

        assert!(property_names.contains(&"display"));
        assert!(property_names.contains(&"color"));
    }

    #[tokio::test]
    async fn test_matched_styles_structure() {
        let bridge = RenderInspectorBridge::new();
        let params = json!({ "nodeId": 6 });

        let result = bridge.handle_method("getMatchedStylesForNode", Some(params)).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        assert!(value["matchedCSSRules"].is_array());
    }

    #[tokio::test]
    async fn test_inline_styles_structure() {
        let bridge = RenderInspectorBridge::new();
        let params = json!({ "nodeId": 6 });

        let result = bridge.handle_method("getInlineStylesForNode", Some(params)).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        assert!(value["inlineStyle"].is_object());
    }

    #[tokio::test]
    async fn test_layer_tree_has_root_layer() {
        let bridge = RenderInspectorBridge::new();

        let result = bridge.handle_method("getLayerTree", None).await;
        assert!(result.is_ok());

        let layers = result.unwrap()["layers"].as_array().unwrap();
        assert!(!layers.is_empty());

        let root_layer = &layers[0];
        assert_eq!(root_layer["layerId"], "root-layer");
        assert!(root_layer["drawsContent"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_layer_tree_has_hierarchy() {
        let bridge = RenderInspectorBridge::new();

        let result = bridge.handle_method("getLayerTree", None).await;
        let layers = result.unwrap()["layers"].as_array().unwrap();

        // Check parent-child relationship
        let child_layer = layers.iter().find(|l| l["layerId"] == "body-layer").unwrap();
        assert_eq!(child_layer["parentLayerId"], "root-layer");
    }

    #[tokio::test]
    async fn test_get_specific_layer() {
        let bridge = RenderInspectorBridge::new();
        let params = json!({ "layerId": "container-layer" });

        let result = bridge.handle_method("getLayer", Some(params)).await;
        assert!(result.is_ok());

        let layer = &result.unwrap()["layer"];
        assert_eq!(layer["layerId"], "container-layer");
        assert!(layer["width"].as_f64().unwrap() > 0.0);
    }

    #[tokio::test]
    async fn test_compose_layers() {
        let bridge = RenderInspectorBridge::new();
        let params = json!({ "layerId": "root-layer" });

        let result = bridge.handle_method("composeLayers", Some(params)).await;
        assert!(result.is_ok());

        let snapshot_id = result.unwrap()["snapshotId"].as_str().unwrap();
        assert!(snapshot_id.contains("root-layer"));
    }

    #[tokio::test]
    async fn test_enable_disable_layer_tree() {
        let bridge = RenderInspectorBridge::new();

        let enable_result = bridge.handle_method("enableLayerTree", None).await;
        assert!(enable_result.is_ok());

        let disable_result = bridge.handle_method("disableLayerTree", None).await;
        assert!(disable_result.is_ok());
    }

    #[tokio::test]
    async fn test_error_on_invalid_node_for_box_model() {
        let bridge = RenderInspectorBridge::new();
        let params = json!({ "nodeId": 99999 });

        let result = bridge.handle_method("getBoxModel", Some(params)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_error_on_nonexistent_layer() {
        let bridge = RenderInspectorBridge::new();
        let params = json!({ "layerId": "nonexistent" });

        let result = bridge.handle_method("getLayer", Some(params)).await;
        assert!(result.is_err());
    }
}

mod types_tests {
    use super::*;

    #[test]
    fn test_rgba_serialization() {
        let color = RGBA::new(128, 64, 32, 0.75);
        let json = serde_json::to_value(&color).unwrap();

        assert_eq!(json["r"], 128);
        assert_eq!(json["g"], 64);
        assert_eq!(json["b"], 32);
        assert!((json["a"].as_f64().unwrap() - 0.75).abs() < 0.01);
    }

    #[test]
    fn test_highlight_config_defaults() {
        let config = HighlightConfig::default();

        assert!(config.show_info);
        assert!(!config.show_rulers);
        assert!(config.content_color.is_some());
        assert!(config.padding_color.is_some());
        assert!(config.border_color.is_some());
        assert!(config.margin_color.is_some());
    }

    #[test]
    fn test_box_model_quad_format() {
        let box_model = BoxModel {
            content: [0.0, 0.0, 100.0, 0.0, 100.0, 100.0, 0.0, 100.0],
            padding: [0.0, 0.0, 100.0, 0.0, 100.0, 100.0, 0.0, 100.0],
            border: [0.0, 0.0, 100.0, 0.0, 100.0, 100.0, 0.0, 100.0],
            margin: [0.0, 0.0, 100.0, 0.0, 100.0, 100.0, 0.0, 100.0],
            width: 100.0,
            height: 100.0,
            shape_outside: None,
        };

        let json = serde_json::to_value(&box_model).unwrap();
        assert_eq!(json["content"].as_array().unwrap().len(), 8);
    }

    #[test]
    fn test_mutation_type_serialization() {
        use inspector_bridges::types::MutationType;

        let types = vec![
            MutationType::ChildListAdded,
            MutationType::ChildListRemoved,
            MutationType::AttributeModified,
            MutationType::CharacterDataModified,
            MutationType::SubtreeModified,
        ];

        for mt in types {
            let json = serde_json::to_value(&mt).unwrap();
            assert!(json.is_string());
        }
    }

    #[test]
    fn test_layer_info_serialization() {
        let layer = LayerInfo {
            layer_id: "test-layer".to_string(),
            parent_layer_id: Some("parent".to_string()),
            node_id: Some(cdp_types::domains::dom::NodeId(1)),
            offset_x: 10.0,
            offset_y: 20.0,
            width: 800.0,
            height: 600.0,
            transform: None,
            compositing_reasons: vec!["transform".to_string()],
            draws_content: true,
        };

        let json = serde_json::to_value(&layer).unwrap();
        assert_eq!(json["layerId"], "test-layer");
        assert_eq!(json["parentLayerId"], "parent");
        assert_eq!(json["width"], 800.0);
    }

    #[test]
    fn test_search_result_serialization() {
        let result = SearchResult {
            search_id: "search-123".to_string(),
            result_count: 42,
        };

        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["searchId"], "search-123");
        assert_eq!(json["resultCount"], 42);
    }
}
