//! Mock DOM bridge for testing DOM and CSS domain handlers
//!
//! This provides a simple in-memory DOM tree for testing purposes.
//! In production, this would be replaced with a real bridge to the browser's DOM implementation.

use cdp_types::domains::css::{CSSProperty, ComputedStyles};
use cdp_types::domains::dom::{Node, NodeId, NodeType};
use dashmap::DashMap;
use std::sync::Arc;

/// Mock DOM bridge for testing
///
/// Simulates a simple DOM tree with a document root and a few child elements
pub struct MockDomBridge {
    /// Map of node IDs to nodes
    nodes: Arc<DashMap<NodeId, Node>>,
    /// Counter for generating unique node IDs
    next_node_id: std::sync::atomic::AtomicU32,
}

impl MockDomBridge {
    /// Create a new mock DOM bridge with a sample DOM tree
    pub fn new() -> Self {
        let bridge = Self {
            nodes: Arc::new(DashMap::new()),
            next_node_id: std::sync::atomic::AtomicU32::new(1),
        };

        // Create a simple document structure for testing
        bridge.initialize_test_dom();
        bridge
    }

    /// Initialize a test DOM structure
    fn initialize_test_dom(&self) {
        // Document node (#document)
        self.nodes.insert(
            NodeId(1),
            Node {
                node_id: NodeId(1),
                node_type: NodeType::Document,
                node_name: "#document".to_string(),
                local_name: None,
                node_value: None,
                child_node_count: Some(1),
                children: None,
                attributes: None,
            },
        );

        // HTML element
        self.nodes.insert(
            NodeId(2),
            Node {
                node_id: NodeId(2),
                node_type: NodeType::Element,
                node_name: "HTML".to_string(),
                local_name: Some("html".to_string()),
                node_value: None,
                child_node_count: Some(2),
                children: None,
                attributes: Some(vec![]),
            },
        );

        // Body element
        self.nodes.insert(
            NodeId(3),
            Node {
                node_id: NodeId(3),
                node_type: NodeType::Element,
                node_name: "BODY".to_string(),
                local_name: Some("body".to_string()),
                node_value: None,
                child_node_count: Some(1),
                children: None,
                attributes: Some(vec![]),
            },
        );

        // Div element
        self.nodes.insert(
            NodeId(4),
            Node {
                node_id: NodeId(4),
                node_type: NodeType::Element,
                node_name: "DIV".to_string(),
                local_name: Some("div".to_string()),
                node_value: None,
                child_node_count: Some(0),
                children: None,
                attributes: Some(vec!["id".to_string(), "test-div".to_string()]),
            },
        );

        self.next_node_id
            .store(5, std::sync::atomic::Ordering::SeqCst);
    }

    /// Get the document root node
    pub fn get_document(&self) -> Option<Node> {
        self.nodes.get(&NodeId(1)).map(|n| n.value().clone())
    }

    /// Get a node by ID
    pub fn get_node(&self, node_id: NodeId) -> Option<Node> {
        self.nodes.get(&node_id).map(|n| n.value().clone())
    }

    /// Query for an element using a CSS selector
    ///
    /// This is a very simplified implementation for testing
    pub fn query_selector(&self, _node_id: NodeId, selector: &str) -> Option<NodeId> {
        // Simple mock implementation - just match some basic selectors
        match selector {
            "div" | "#test-div" => Some(NodeId(4)),
            "body" => Some(NodeId(3)),
            "html" => Some(NodeId(2)),
            _ => None,
        }
    }

    /// Set an attribute value on a node
    pub fn set_attribute(&self, node_id: NodeId, name: &str, value: &str) -> Result<(), String> {
        if let Some(mut node_entry) = self.nodes.get_mut(&node_id) {
            let node = node_entry.value_mut();

            // Ensure the node is an element
            if node.node_type != NodeType::Element {
                return Err("Cannot set attribute on non-element node".to_string());
            }

            // Get or create attributes vec
            let attributes = node.attributes.get_or_insert_with(Vec::new);

            // Look for existing attribute
            let mut found = false;
            for i in (0..attributes.len()).step_by(2) {
                if attributes[i] == name {
                    attributes[i + 1] = value.to_string();
                    found = true;
                    break;
                }
            }

            // Add new attribute if not found
            if !found {
                attributes.push(name.to_string());
                attributes.push(value.to_string());
            }

            Ok(())
        } else {
            Err(format!("Node {} not found", node_id.0))
        }
    }

    /// Get computed styles for a node
    ///
    /// Returns mock computed styles for testing
    pub fn get_computed_styles(&self, _node_id: NodeId) -> ComputedStyles {
        // Return some mock computed styles
        ComputedStyles {
            properties: vec![
                CSSProperty {
                    name: "display".to_string(),
                    value: "block".to_string(),
                    important: Some(false),
                    implicit: Some(false),
                    text: Some("display: block".to_string()),
                    parsed_ok: Some(true),
                    disabled: Some(false),
                    range: None,
                },
                CSSProperty {
                    name: "color".to_string(),
                    value: "rgb(0, 0, 0)".to_string(),
                    important: Some(false),
                    implicit: Some(false),
                    text: Some("color: rgb(0, 0, 0)".to_string()),
                    parsed_ok: Some(true),
                    disabled: Some(false),
                    range: None,
                },
            ],
        }
    }
}

impl Default for MockDomBridge {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_dom_creation() {
        let bridge = MockDomBridge::new();
        assert!(bridge.get_document().is_some());
    }

    #[test]
    fn test_get_document() {
        let bridge = MockDomBridge::new();
        let doc = bridge.get_document().unwrap();

        assert_eq!(doc.node_id, NodeId(1));
        assert_eq!(doc.node_type, NodeType::Document);
        assert_eq!(doc.node_name, "#document");
    }

    #[test]
    fn test_get_node() {
        let bridge = MockDomBridge::new();

        let node = bridge.get_node(NodeId(4)).unwrap();
        assert_eq!(node.node_name, "DIV");
        assert_eq!(node.node_type, NodeType::Element);
    }

    #[test]
    fn test_query_selector() {
        let bridge = MockDomBridge::new();

        let result = bridge.query_selector(NodeId(1), "div");
        assert_eq!(result, Some(NodeId(4)));

        let result = bridge.query_selector(NodeId(1), "#test-div");
        assert_eq!(result, Some(NodeId(4)));

        let result = bridge.query_selector(NodeId(1), ".nonexistent");
        assert_eq!(result, None);
    }

    #[test]
    fn test_set_attribute() {
        let bridge = MockDomBridge::new();

        let result = bridge.set_attribute(NodeId(4), "class", "test-class");
        assert!(result.is_ok());

        // Verify attribute was set
        let node = bridge.get_node(NodeId(4)).unwrap();
        let attrs = node.attributes.unwrap();
        assert!(attrs.contains(&"class".to_string()));
        assert!(attrs.contains(&"test-class".to_string()));
    }

    #[test]
    fn test_set_attribute_invalid_node() {
        let bridge = MockDomBridge::new();

        let result = bridge.set_attribute(NodeId(99999), "class", "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_computed_styles() {
        let bridge = MockDomBridge::new();

        let styles = bridge.get_computed_styles(NodeId(4));
        assert!(!styles.properties.is_empty());
        assert!(styles
            .properties
            .iter()
            .any(|p| p.name == "display" && p.value == "block"));
    }
}
