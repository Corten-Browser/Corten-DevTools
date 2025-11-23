//! Integration tests for inspector bridges

use inspector_bridges::*;
use protocol_handler::DomainHandler;
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn test_shared_browser_dom_and_render() {
    // Create shared browser instance
    let browser = Arc::new(MockBrowser::new());
    let dom_bridge = DomInspectorBridge::with_browser(browser.clone());
    let render_bridge = RenderInspectorBridge::with_browser(browser);

    // Get document via DOM bridge
    let doc = dom_bridge.handle_method("getDocument", None).await.unwrap();
    assert_eq!(doc["root"]["nodeId"], 1);

    // Query for an element via DOM bridge
    let query_result = dom_bridge
        .handle_method("querySelector", Some(json!({
            "nodeId": 1,
            "selector": "#container"
        })))
        .await
        .unwrap();

    let node_id = query_result["nodeId"].as_u64().unwrap();

    // Get box model via Render bridge for the same element
    let box_model = render_bridge
        .handle_method("getBoxModel", Some(json!({ "nodeId": node_id })))
        .await
        .unwrap();

    assert!(box_model["model"]["width"].as_f64().unwrap() > 0.0);

    // Get computed styles for the same element
    let styles = render_bridge
        .handle_method("getComputedStyleForNode", Some(json!({ "nodeId": node_id })))
        .await
        .unwrap();

    assert!(!styles["computedStyle"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_dom_mutation_propagation() {
    let bridge = DomInspectorBridge::new();
    let mut receiver = bridge.subscribe_mutations();

    // Make a change
    bridge
        .handle_method("setAttributeValue", Some(json!({
            "nodeId": 6,
            "name": "data-test-mutation",
            "value": "test-value"
        })))
        .await
        .unwrap();

    // Verify mutation was recorded
    let mutation = tokio::time::timeout(
        std::time::Duration::from_millis(100),
        receiver.recv()
    ).await.expect("Timeout waiting for mutation");

    let record = mutation.unwrap();
    assert_eq!(record.attribute_name.unwrap(), "data-test-mutation");
}

#[tokio::test]
async fn test_full_inspection_workflow() {
    let dom_bridge = DomInspectorBridge::new();
    let render_bridge = RenderInspectorBridge::new();

    // 1. Get document
    let doc = dom_bridge.handle_method("getDocument", Some(json!({ "depth": 3 }))).await.unwrap();
    assert!(doc["root"].is_object());

    // 2. Search for elements
    let search = dom_bridge
        .handle_method("performSearch", Some(json!({
            "query": "div",
            "includeUserAgentShadowDOM": false
        })))
        .await
        .unwrap();

    let search_id = search["searchId"].as_str().unwrap();
    let count = search["resultCount"].as_u64().unwrap() as u32;

    // 3. Get search results
    let results = dom_bridge
        .handle_method("getSearchResults", Some(json!({
            "searchId": search_id,
            "fromIndex": 0,
            "toIndex": count
        })))
        .await
        .unwrap();

    let node_ids = results["nodeIds"].as_array().unwrap();
    assert!(!node_ids.is_empty());

    // 4. Select first result
    let first_node_id = node_ids[0].as_u64().unwrap();

    // 5. Highlight it
    dom_bridge
        .handle_method("highlightNode", Some(json!({
            "highlightConfig": { "showInfo": true },
            "nodeId": first_node_id
        })))
        .await
        .unwrap();

    // 6. Get box model
    let box_model = render_bridge
        .handle_method("getBoxModel", Some(json!({ "nodeId": first_node_id })))
        .await
        .unwrap();

    assert!(box_model["model"]["content"].is_array());

    // 7. Get computed styles
    let styles = render_bridge
        .handle_method("getComputedStyleForNode", Some(json!({ "nodeId": first_node_id })))
        .await
        .unwrap();

    assert!(!styles["computedStyle"].as_array().unwrap().is_empty());

    // 8. Get layer tree
    let layers = render_bridge.handle_method("getLayerTree", None).await.unwrap();
    assert!(!layers["layers"].as_array().unwrap().is_empty());

    // 9. Cleanup
    dom_bridge.handle_method("hideHighlight", None).await.unwrap();
    dom_bridge
        .handle_method("discardSearchResults", Some(json!({ "searchId": search_id })))
        .await
        .unwrap();
}

#[tokio::test]
async fn test_dom_modification_workflow() {
    let bridge = DomInspectorBridge::new();

    // Get original node
    let original = bridge
        .handle_method("describeNode", Some(json!({ "nodeId": 6, "depth": 0 })))
        .await
        .unwrap();

    let original_attrs = original["node"]["attributes"].as_array().unwrap();

    // Add attribute
    bridge
        .handle_method("setAttributeValue", Some(json!({
            "nodeId": 6,
            "name": "data-modified",
            "value": "yes"
        })))
        .await
        .unwrap();

    // Verify change
    let modified = bridge
        .handle_method("describeNode", Some(json!({ "nodeId": 6, "depth": 0 })))
        .await
        .unwrap();

    let modified_attrs = modified["node"]["attributes"].as_array().unwrap();
    assert!(modified_attrs.len() > original_attrs.len());

    // Remove attribute
    bridge
        .handle_method("removeAttribute", Some(json!({
            "nodeId": 6,
            "name": "data-modified"
        })))
        .await
        .unwrap();
}

#[tokio::test]
async fn test_layer_inspection_workflow() {
    let bridge = RenderInspectorBridge::new();

    // Enable layer tree
    bridge.handle_method("enableLayerTree", None).await.unwrap();

    // Get layer tree
    let tree = bridge.handle_method("getLayerTree", None).await.unwrap();
    let layers = tree["layers"].as_array().unwrap();

    // Inspect each layer
    for layer in layers {
        let layer_id = layer["layerId"].as_str().unwrap();

        // Get layer details
        let layer_detail = bridge
            .handle_method("getLayer", Some(json!({ "layerId": layer_id })))
            .await
            .unwrap();

        assert_eq!(layer_detail["layer"]["layerId"], layer_id);

        // Compose layer
        let snapshot = bridge
            .handle_method("composeLayers", Some(json!({ "layerId": layer_id })))
            .await
            .unwrap();

        assert!(snapshot["snapshotId"].is_string());
    }

    // Disable layer tree
    bridge.handle_method("disableLayerTree", None).await.unwrap();
}

#[tokio::test]
async fn test_error_handling_consistency() {
    let dom_bridge = DomInspectorBridge::new();
    let render_bridge = RenderInspectorBridge::new();

    // DOM bridge error - invalid node
    let dom_err = dom_bridge
        .handle_method("querySelector", Some(json!({
            "nodeId": 99999,
            "selector": "div"
        })))
        .await;
    assert!(dom_err.is_err());

    // Render bridge error - invalid node
    let render_err = render_bridge
        .handle_method("getBoxModel", Some(json!({ "nodeId": 99999 })))
        .await;
    assert!(render_err.is_err());

    // DOM bridge error - unknown method
    let method_err = dom_bridge.handle_method("unknownMethod", None).await;
    assert!(method_err.is_err());

    // Render bridge error - unknown method
    let method_err = render_bridge.handle_method("unknownMethod", None).await;
    assert!(method_err.is_err());
}

#[tokio::test]
async fn test_concurrent_operations() {
    let browser = Arc::new(MockBrowser::new());
    let dom_bridge = Arc::new(DomInspectorBridge::with_browser(browser.clone()));
    let render_bridge = Arc::new(RenderInspectorBridge::with_browser(browser));

    // Run multiple operations concurrently
    let dom_bridge_clone = dom_bridge.clone();
    let render_bridge_clone = render_bridge.clone();

    let handles = vec![
        tokio::spawn(async move {
            dom_bridge_clone.handle_method("getDocument", None).await
        }),
        tokio::spawn(async move {
            render_bridge_clone.handle_method("getLayerTree", None).await
        }),
        tokio::spawn({
            let bridge = dom_bridge.clone();
            async move {
                bridge
                    .handle_method("querySelector", Some(json!({
                        "nodeId": 1,
                        "selector": "div"
                    })))
                    .await
            }
        }),
        tokio::spawn({
            let bridge = render_bridge.clone();
            async move {
                bridge
                    .handle_method("getBoxModel", Some(json!({ "nodeId": 6 })))
                    .await
            }
        }),
    ];

    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }
}
