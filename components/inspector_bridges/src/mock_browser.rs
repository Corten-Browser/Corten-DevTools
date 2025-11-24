//! Mock browser implementation for testing inspector bridges
//!
//! Provides a simulated browser environment for testing DOM and render inspection.

use cdp_types::domains::css::{CSSProperty, ComputedStyles};
use cdp_types::domains::dom::{Node, NodeId, NodeType};
use dashmap::DashMap;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::types::{BoxModel, LayerInfo, MutationRecord, MutationType};

/// Mock browser for testing inspector bridges
pub struct MockBrowser {
    /// Map of node IDs to nodes
    nodes: Arc<DashMap<NodeId, Node>>,
    /// Parent-child relationships
    children: Arc<DashMap<NodeId, Vec<NodeId>>>,
    /// Counter for generating unique node IDs
    next_node_id: AtomicU32,
    /// Search ID counter
    next_search_id: AtomicU64,
    /// Active searches: search_id -> matching node IDs
    searches: Arc<DashMap<String, Vec<NodeId>>>,
    /// Mutation broadcast channel
    mutation_sender: broadcast::Sender<MutationRecord>,
}

impl MockBrowser {
    /// Create a new mock browser with a sample DOM tree
    pub fn new() -> Self {
        let (mutation_sender, _) = broadcast::channel(100);
        let browser = Self {
            nodes: Arc::new(DashMap::new()),
            children: Arc::new(DashMap::new()),
            next_node_id: AtomicU32::new(1),
            next_search_id: AtomicU64::new(1),
            searches: Arc::new(DashMap::new()),
            mutation_sender,
        };

        browser.initialize_test_dom();
        browser
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
                attributes: Some(vec!["lang".to_string(), "en".to_string()]),
            },
        );

        // Head element
        self.nodes.insert(
            NodeId(3),
            Node {
                node_id: NodeId(3),
                node_type: NodeType::Element,
                node_name: "HEAD".to_string(),
                local_name: Some("head".to_string()),
                node_value: None,
                child_node_count: Some(1),
                children: None,
                attributes: Some(vec![]),
            },
        );

        // Title element
        self.nodes.insert(
            NodeId(4),
            Node {
                node_id: NodeId(4),
                node_type: NodeType::Element,
                node_name: "TITLE".to_string(),
                local_name: Some("title".to_string()),
                node_value: None,
                child_node_count: Some(1),
                children: None,
                attributes: Some(vec![]),
            },
        );

        // Body element
        self.nodes.insert(
            NodeId(5),
            Node {
                node_id: NodeId(5),
                node_type: NodeType::Element,
                node_name: "BODY".to_string(),
                local_name: Some("body".to_string()),
                node_value: None,
                child_node_count: Some(2),
                children: None,
                attributes: Some(vec!["class".to_string(), "main-body".to_string()]),
            },
        );

        // Div element with ID
        self.nodes.insert(
            NodeId(6),
            Node {
                node_id: NodeId(6),
                node_type: NodeType::Element,
                node_name: "DIV".to_string(),
                local_name: Some("div".to_string()),
                node_value: None,
                child_node_count: Some(1),
                children: None,
                attributes: Some(vec![
                    "id".to_string(),
                    "container".to_string(),
                    "class".to_string(),
                    "wrapper".to_string(),
                ]),
            },
        );

        // Span element
        self.nodes.insert(
            NodeId(7),
            Node {
                node_id: NodeId(7),
                node_type: NodeType::Element,
                node_name: "SPAN".to_string(),
                local_name: Some("span".to_string()),
                node_value: None,
                child_node_count: Some(1),
                children: None,
                attributes: Some(vec!["class".to_string(), "text-content".to_string()]),
            },
        );

        // Text node
        self.nodes.insert(
            NodeId(8),
            Node {
                node_id: NodeId(8),
                node_type: NodeType::Text,
                node_name: "#text".to_string(),
                local_name: None,
                node_value: Some("Hello, World!".to_string()),
                child_node_count: None,
                children: None,
                attributes: None,
            },
        );

        // Another div element
        self.nodes.insert(
            NodeId(9),
            Node {
                node_id: NodeId(9),
                node_type: NodeType::Element,
                node_name: "DIV".to_string(),
                local_name: Some("div".to_string()),
                node_value: None,
                child_node_count: Some(0),
                children: None,
                attributes: Some(vec!["id".to_string(), "footer".to_string()]),
            },
        );

        // Set up parent-child relationships
        self.children.insert(NodeId(1), vec![NodeId(2)]);
        self.children.insert(NodeId(2), vec![NodeId(3), NodeId(5)]);
        self.children.insert(NodeId(3), vec![NodeId(4)]);
        self.children.insert(NodeId(5), vec![NodeId(6), NodeId(9)]);
        self.children.insert(NodeId(6), vec![NodeId(7)]);
        self.children.insert(NodeId(7), vec![NodeId(8)]);

        self.next_node_id.store(10, Ordering::SeqCst);
    }

    /// Get the document root node
    pub fn get_document(&self) -> Option<Node> {
        self.get_node_with_children(NodeId(1), 0)
    }

    /// Get a node by ID
    pub fn get_node(&self, node_id: NodeId) -> Option<Node> {
        self.nodes.get(&node_id).map(|n| n.value().clone())
    }

    /// Get a node with its children up to specified depth
    pub fn get_node_with_children(&self, node_id: NodeId, depth: u32) -> Option<Node> {
        let mut node = self.nodes.get(&node_id)?.value().clone();

        if depth > 0 {
            if let Some(child_ids) = self.children.get(&node_id) {
                let children: Vec<Node> = child_ids
                    .iter()
                    .filter_map(|id| self.get_node_with_children(*id, depth - 1))
                    .collect();

                if !children.is_empty() {
                    node.children = Some(children);
                }
            }
        }

        Some(node)
    }

    /// Get child nodes of a parent
    pub fn get_children(&self, parent_id: NodeId) -> Vec<NodeId> {
        self.children
            .get(&parent_id)
            .map(|c| c.value().clone())
            .unwrap_or_default()
    }

    /// Get all descendant node IDs
    pub fn get_descendants(&self, node_id: NodeId) -> Vec<NodeId> {
        let mut descendants = Vec::new();
        let mut stack = vec![node_id];

        while let Some(current) = stack.pop() {
            if current != node_id {
                descendants.push(current);
            }
            if let Some(children) = self.children.get(&current) {
                stack.extend(children.iter().copied());
            }
        }

        descendants
    }

    /// Query for elements using a CSS selector (simplified)
    pub fn query_selector(&self, _root_id: NodeId, selector: &str) -> Option<NodeId> {
        // Simple selector matching for testing
        self.query_selector_all(_root_id, selector).first().copied()
    }

    /// Query for all elements matching a CSS selector (simplified)
    pub fn query_selector_all(&self, _root_id: NodeId, selector: &str) -> Vec<NodeId> {
        let mut results = Vec::new();

        for entry in self.nodes.iter() {
            let node = entry.value();
            if self.matches_selector(node, selector) {
                results.push(node.node_id);
            }
        }

        results.sort_by_key(|id| id.0);
        results
    }

    /// Simple selector matching for testing
    fn matches_selector(&self, node: &Node, selector: &str) -> bool {
        if node.node_type != NodeType::Element {
            return false;
        }

        let selector = selector.trim();

        // ID selector
        if let Some(id_name) = selector.strip_prefix('#') {
            if let Some(ref attrs) = node.attributes {
                for i in (0..attrs.len()).step_by(2) {
                    if attrs[i] == "id" && i + 1 < attrs.len() && attrs[i + 1] == id_name {
                        return true;
                    }
                }
            }
            return false;
        }

        // Class selector
        if let Some(class_name) = selector.strip_prefix('.') {
            if let Some(ref attrs) = node.attributes {
                for i in (0..attrs.len()).step_by(2) {
                    if attrs[i] == "class" && i + 1 < attrs.len() {
                        let classes: Vec<&str> = attrs[i + 1].split_whitespace().collect();
                        if classes.contains(&class_name) {
                            return true;
                        }
                    }
                }
            }
            return false;
        }

        // Tag selector
        let local_name = node.local_name.as_deref().unwrap_or("");
        local_name.eq_ignore_ascii_case(selector) || node.node_name.eq_ignore_ascii_case(selector)
    }

    /// Set an attribute value on a node
    pub fn set_attribute(&self, node_id: NodeId, name: &str, value: &str) -> Result<(), String> {
        if let Some(mut node_entry) = self.nodes.get_mut(&node_id) {
            let node = node_entry.value_mut();

            if node.node_type != NodeType::Element {
                return Err("Cannot set attribute on non-element node".to_string());
            }

            let old_value = self.get_attribute_value(node, name);
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

            // Emit mutation
            let _ = self.mutation_sender.send(MutationRecord {
                mutation_type: MutationType::AttributeModified,
                target_node_id: node_id,
                added_node_ids: None,
                removed_node_ids: None,
                previous_sibling_id: None,
                attribute_name: Some(name.to_string()),
                old_value,
            });

            Ok(())
        } else {
            Err(format!("Node {} not found", node_id.0))
        }
    }

    /// Get attribute value helper
    fn get_attribute_value(&self, node: &Node, name: &str) -> Option<String> {
        if let Some(ref attrs) = node.attributes {
            for i in (0..attrs.len()).step_by(2) {
                if attrs[i] == name && i + 1 < attrs.len() {
                    return Some(attrs[i + 1].clone());
                }
            }
        }
        None
    }

    /// Remove an attribute from a node
    pub fn remove_attribute(&self, node_id: NodeId, name: &str) -> Result<(), String> {
        if let Some(mut node_entry) = self.nodes.get_mut(&node_id) {
            let node = node_entry.value_mut();

            if node.node_type != NodeType::Element {
                return Err("Cannot remove attribute from non-element node".to_string());
            }

            let old_value = self.get_attribute_value(node, name);

            if let Some(ref mut attributes) = node.attributes {
                let mut i = 0;
                while i < attributes.len() {
                    if attributes[i] == name {
                        attributes.remove(i + 1);
                        attributes.remove(i);
                        break;
                    }
                    i += 2;
                }
            }

            // Emit mutation
            let _ = self.mutation_sender.send(MutationRecord {
                mutation_type: MutationType::AttributeModified,
                target_node_id: node_id,
                added_node_ids: None,
                removed_node_ids: None,
                previous_sibling_id: None,
                attribute_name: Some(name.to_string()),
                old_value,
            });

            Ok(())
        } else {
            Err(format!("Node {} not found", node_id.0))
        }
    }

    /// Search for nodes containing text
    pub fn perform_search(
        &self,
        query: &str,
        include_user_agent_shadow_dom: bool,
    ) -> (String, u32) {
        let search_id = format!(
            "search-{}",
            self.next_search_id.fetch_add(1, Ordering::SeqCst)
        );
        let mut matching_nodes = Vec::new();

        let query_lower = query.to_lowercase();

        for entry in self.nodes.iter() {
            let node = entry.value();

            // Skip shadow DOM if not requested
            if !include_user_agent_shadow_dom {
                // In a real implementation, we'd check shadow DOM
            }

            // Check node name
            if node.node_name.to_lowercase().contains(&query_lower) {
                matching_nodes.push(node.node_id);
                continue;
            }

            // Check text content
            if let Some(ref value) = node.node_value {
                if value.to_lowercase().contains(&query_lower) {
                    matching_nodes.push(node.node_id);
                    continue;
                }
            }

            // Check attributes
            if let Some(ref attrs) = node.attributes {
                for attr in attrs {
                    if attr.to_lowercase().contains(&query_lower) {
                        matching_nodes.push(node.node_id);
                        break;
                    }
                }
            }
        }

        let count = matching_nodes.len() as u32;
        self.searches.insert(search_id.clone(), matching_nodes);
        (search_id, count)
    }

    /// Get search results
    pub fn get_search_results(
        &self,
        search_id: &str,
        from_index: u32,
        to_index: u32,
    ) -> Vec<NodeId> {
        self.searches
            .get(search_id)
            .map(|results| {
                let from = from_index as usize;
                let to = (to_index as usize).min(results.len());
                if from < to {
                    results[from..to].to_vec()
                } else {
                    vec![]
                }
            })
            .unwrap_or_default()
    }

    /// Discard search results
    pub fn discard_search_results(&self, search_id: &str) {
        self.searches.remove(search_id);
    }

    /// Get computed styles for a node (mock)
    pub fn get_computed_styles(&self, node_id: NodeId) -> Option<ComputedStyles> {
        if self.nodes.contains_key(&node_id) {
            Some(ComputedStyles {
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
                    CSSProperty {
                        name: "font-size".to_string(),
                        value: "16px".to_string(),
                        important: Some(false),
                        implicit: Some(false),
                        text: Some("font-size: 16px".to_string()),
                        parsed_ok: Some(true),
                        disabled: Some(false),
                        range: None,
                    },
                    CSSProperty {
                        name: "margin".to_string(),
                        value: "0px".to_string(),
                        important: Some(false),
                        implicit: Some(false),
                        text: Some("margin: 0px".to_string()),
                        parsed_ok: Some(true),
                        disabled: Some(false),
                        range: None,
                    },
                    CSSProperty {
                        name: "padding".to_string(),
                        value: "0px".to_string(),
                        important: Some(false),
                        implicit: Some(false),
                        text: Some("padding: 0px".to_string()),
                        parsed_ok: Some(true),
                        disabled: Some(false),
                        range: None,
                    },
                ],
            })
        } else {
            None
        }
    }

    /// Get box model for a node (mock)
    pub fn get_box_model(&self, node_id: NodeId) -> Option<BoxModel> {
        if self.nodes.contains_key(&node_id) {
            // Return mock box model based on node ID for variety
            let base = (node_id.0 * 10) as f64;
            let width = 100.0 + base;
            let height = 50.0 + base;

            Some(BoxModel {
                content: [
                    base + 10.0,
                    base + 10.0,
                    base + 10.0 + width,
                    base + 10.0,
                    base + 10.0 + width,
                    base + 10.0 + height,
                    base + 10.0,
                    base + 10.0 + height,
                ],
                padding: [
                    base + 5.0,
                    base + 5.0,
                    base + 15.0 + width,
                    base + 5.0,
                    base + 15.0 + width,
                    base + 15.0 + height,
                    base + 5.0,
                    base + 15.0 + height,
                ],
                border: [
                    base + 3.0,
                    base + 3.0,
                    base + 17.0 + width,
                    base + 3.0,
                    base + 17.0 + width,
                    base + 17.0 + height,
                    base + 3.0,
                    base + 17.0 + height,
                ],
                margin: [
                    base,
                    base,
                    base + 20.0 + width,
                    base,
                    base + 20.0 + width,
                    base + 20.0 + height,
                    base,
                    base + 20.0 + height,
                ],
                width,
                height,
                shape_outside: None,
            })
        } else {
            None
        }
    }

    /// Get layer tree (mock)
    pub fn get_layer_tree(&self) -> Vec<LayerInfo> {
        vec![
            LayerInfo {
                layer_id: "root-layer".to_string(),
                parent_layer_id: None,
                node_id: Some(NodeId(1)),
                offset_x: 0.0,
                offset_y: 0.0,
                width: 1920.0,
                height: 1080.0,
                transform: None,
                compositing_reasons: vec!["root".to_string()],
                draws_content: true,
            },
            LayerInfo {
                layer_id: "body-layer".to_string(),
                parent_layer_id: Some("root-layer".to_string()),
                node_id: Some(NodeId(5)),
                offset_x: 0.0,
                offset_y: 0.0,
                width: 1920.0,
                height: 1080.0,
                transform: None,
                compositing_reasons: vec!["layoutObject".to_string()],
                draws_content: true,
            },
            LayerInfo {
                layer_id: "container-layer".to_string(),
                parent_layer_id: Some("body-layer".to_string()),
                node_id: Some(NodeId(6)),
                offset_x: 10.0,
                offset_y: 10.0,
                width: 800.0,
                height: 600.0,
                transform: None,
                compositing_reasons: vec!["transform".to_string()],
                draws_content: true,
            },
        ]
    }

    /// Subscribe to mutations
    pub fn subscribe_mutations(&self) -> broadcast::Receiver<MutationRecord> {
        self.mutation_sender.subscribe()
    }

    /// Check if node exists
    pub fn node_exists(&self, node_id: NodeId) -> bool {
        self.nodes.contains_key(&node_id)
    }
}

impl Default for MockBrowser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_browser_creation() {
        let browser = MockBrowser::new();
        assert!(browser.get_document().is_some());
    }

    #[test]
    fn test_get_document() {
        let browser = MockBrowser::new();
        let doc = browser.get_document().unwrap();
        assert_eq!(doc.node_id, NodeId(1));
        assert_eq!(doc.node_type, NodeType::Document);
    }

    #[test]
    fn test_get_node() {
        let browser = MockBrowser::new();
        let node = browser.get_node(NodeId(6)).unwrap();
        assert_eq!(node.node_name, "DIV");
    }

    #[test]
    fn test_get_children() {
        let browser = MockBrowser::new();
        let children = browser.get_children(NodeId(5));
        assert_eq!(children.len(), 2);
        assert!(children.contains(&NodeId(6)));
    }

    #[test]
    fn test_query_selector_by_tag() {
        let browser = MockBrowser::new();
        let result = browser.query_selector(NodeId(1), "div");
        assert!(result.is_some());
    }

    #[test]
    fn test_query_selector_by_id() {
        let browser = MockBrowser::new();
        let result = browser.query_selector(NodeId(1), "#container");
        assert_eq!(result, Some(NodeId(6)));
    }

    #[test]
    fn test_query_selector_by_class() {
        let browser = MockBrowser::new();
        let result = browser.query_selector(NodeId(1), ".wrapper");
        assert_eq!(result, Some(NodeId(6)));
    }

    #[test]
    fn test_set_attribute() {
        let browser = MockBrowser::new();
        let result = browser.set_attribute(NodeId(6), "data-test", "value");
        assert!(result.is_ok());

        let node = browser.get_node(NodeId(6)).unwrap();
        let attrs = node.attributes.unwrap();
        assert!(attrs.contains(&"data-test".to_string()));
    }

    #[test]
    fn test_search() {
        let browser = MockBrowser::new();
        let (search_id, count) = browser.perform_search("Hello", false);
        assert!(!search_id.is_empty());
        assert!(count > 0);
    }

    #[test]
    fn test_get_box_model() {
        let browser = MockBrowser::new();
        let box_model = browser.get_box_model(NodeId(6));
        assert!(box_model.is_some());
        let bm = box_model.unwrap();
        assert!(bm.width > 0.0);
        assert!(bm.height > 0.0);
    }

    #[test]
    fn test_get_computed_styles() {
        let browser = MockBrowser::new();
        let styles = browser.get_computed_styles(NodeId(6));
        assert!(styles.is_some());
        assert!(!styles.unwrap().properties.is_empty());
    }

    #[test]
    fn test_get_layer_tree() {
        let browser = MockBrowser::new();
        let layers = browser.get_layer_tree();
        assert!(!layers.is_empty());
        assert_eq!(layers[0].layer_id, "root-layer");
    }
}
