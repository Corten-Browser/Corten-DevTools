# Inspector Bridges

DOM and Render Inspector Bridges for Chrome DevTools Protocol integration.

## Overview

This crate provides bridge implementations between the Chrome DevTools Protocol (CDP) and browser internals for DOM and render tree inspection. It implements two key features:

- **FEAT-017**: DOM Inspector Bridge - Bridge between CDP DOM domain and browser DOM
- **FEAT-020**: Render Inspector Bridge - Bridge for render tree inspection

## Features

### DOM Inspector Bridge

The `DomInspectorBridge` provides:

- **Node tree traversal**: Navigate the DOM tree structure
- **Node selection and highlighting**: Select and highlight nodes for inspection
- **DOM mutation tracking**: Track changes to the DOM
- **Node search functionality**: Search for nodes by text, selector, or attributes

Supported CDP methods:
- `getDocument` - Get the document root
- `requestChildNodes` - Request child nodes for a parent
- `querySelector` - Query for a single element
- `querySelectorAll` - Query for all matching elements
- `setAttributeValue` - Set attribute value on a node
- `removeAttribute` - Remove attribute from a node
- `highlightNode` - Highlight a node
- `hideHighlight` - Hide highlight
- `performSearch` - Search in DOM
- `getSearchResults` - Get search results
- `discardSearchResults` - Discard search results
- `describeNode` - Get detailed node description

### Render Inspector Bridge

The `RenderInspectorBridge` provides:

- **Box model inspection**: Get content, padding, border, and margin boxes
- **Computed styles access**: Get computed CSS styles for elements
- **Layer tree representation**: Access the compositing layer tree

Supported CDP methods:
- `getBoxModel` - Get box model for a node
- `getComputedStyleForNode` - Get computed styles
- `getMatchedStylesForNode` - Get matched CSS rules
- `getInlineStylesForNode` - Get inline styles
- `getLayerTree` - Get layer tree
- `getLayer` - Get specific layer info
- `composeLayers` - Create layer snapshot
- `enableLayerTree` - Enable layer tracking
- `disableLayerTree` - Disable layer tracking

## Usage

```rust
use inspector_bridges::{DomInspectorBridge, RenderInspectorBridge};
use protocol_handler::DomainHandler;
use serde_json::json;

#[tokio::main]
async fn main() {
    // Create bridges
    let dom_bridge = DomInspectorBridge::new();
    let render_bridge = RenderInspectorBridge::new();

    // Get document
    let doc = dom_bridge.handle_method("getDocument", None).await.unwrap();
    println!("Document: {:?}", doc);

    // Query for an element
    let params = json!({
        "nodeId": 1,
        "selector": "#container"
    });
    let result = dom_bridge.handle_method("querySelector", Some(params)).await.unwrap();
    println!("Found node: {:?}", result);

    // Get box model
    let params = json!({ "nodeId": 6 });
    let box_model = render_bridge.handle_method("getBoxModel", Some(params)).await.unwrap();
    println!("Box model: {:?}", box_model);

    // Get computed styles
    let styles = render_bridge.handle_method("getComputedStyleForNode", Some(params)).await.unwrap();
    println!("Computed styles: {:?}", styles);

    // Get layer tree
    let layers = render_bridge.handle_method("getLayerTree", None).await.unwrap();
    println!("Layers: {:?}", layers);
}
```

### Using Shared Browser Instance

Both bridges can share a browser instance for coordinated inspection:

```rust
use std::sync::Arc;
use inspector_bridges::{DomInspectorBridge, RenderInspectorBridge, MockBrowser};

let browser = Arc::new(MockBrowser::new());
let dom_bridge = DomInspectorBridge::with_browser(browser.clone());
let render_bridge = RenderInspectorBridge::with_browser(browser);

// Both bridges now operate on the same DOM
```

### Subscribing to DOM Mutations

```rust
use inspector_bridges::DomInspectorBridge;

let bridge = DomInspectorBridge::new();
let mut receiver = bridge.subscribe_mutations();

// Listen for mutations
tokio::spawn(async move {
    while let Ok(mutation) = receiver.recv().await {
        println!("DOM mutation: {:?}", mutation);
    }
});
```

## Testing

Run tests with:

```bash
cargo test -p inspector_bridges
```

## Architecture

Both bridges implement the `DomainHandler` trait from `protocol_handler`, allowing them to be registered with the protocol handler for processing CDP requests:

```rust
use protocol_handler::ProtocolHandler;
use inspector_bridges::{DomInspectorBridge, RenderInspectorBridge};
use std::sync::Arc;

let handler = ProtocolHandler::new();
handler.register_domain(Arc::new(DomInspectorBridge::new()));
handler.register_domain(Arc::new(RenderInspectorBridge::new()));
```

## Dependencies

- `cdp_types` - CDP type definitions
- `protocol_handler` - CDP message routing
- `dom_domain` - DOM domain types
- `async-trait` - Async trait support
- `serde` - Serialization
- `tokio` - Async runtime
- `dashmap` - Concurrent hashmap
- `tracing` - Logging

## License

MIT OR Apache-2.0
