// DOM domain types

use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

/// Unique DOM node identifier
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct NodeId(pub u32);

/// DOM node type constants
#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr, PartialEq, Eq)]
#[repr(u32)]
pub enum NodeType {
    Element = 1,
    Attribute = 2,
    Text = 3,
    CData = 4,
    EntityReference = 5,
    Entity = 6,
    ProcessingInstruction = 7,
    Comment = 8,
    Document = 9,
    DocumentType = 10,
    DocumentFragment = 11,
    Notation = 12,
}

/// DOM Node description
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    /// Node identifier (assigned by the backend)
    pub node_id: NodeId,
    /// Node type
    pub node_type: NodeType,
    /// Node name
    pub node_name: String,
    /// Local name for elements and attributes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_name: Option<String>,
    /// Node value for text nodes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_value: Option<String>,
    /// Child count for container nodes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub child_node_count: Option<u32>,
    /// Child nodes (if requested)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<Node>>,
    /// Attributes (flat array: [name1, value1, name2, value2, ...])
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<Vec<String>>,
}

/// Response for DOM.getDocument
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetDocumentResponse {
    /// Root document node
    pub root: Node,
}

/// Parameters for DOM.querySelector
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct QuerySelectorParams {
    /// Node to query selector on
    pub node_id: NodeId,
    /// Selector string
    pub selector: String,
}

/// Response for DOM.querySelector
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct QuerySelectorResponse {
    /// Matching node ID (if found)
    pub node_id: Option<NodeId>,
}

/// Parameters for DOM.setAttributeValue
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SetAttributeValueParams {
    /// Node ID
    pub node_id: NodeId,
    /// Attribute name
    pub name: String,
    /// Attribute value
    pub value: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_id() {
        let id = NodeId(123);
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "123");
    }

    #[test]
    fn test_node_type() {
        assert_eq!(NodeType::Element as u32, 1);
        assert_eq!(NodeType::Document as u32, 9);
    }

    #[test]
    fn test_node_serialization() {
        let node = Node {
            node_id: NodeId(1),
            node_type: NodeType::Element,
            node_name: "div".to_string(),
            local_name: Some("div".to_string()),
            node_value: None,
            child_node_count: Some(0),
            children: None,
            attributes: Some(vec!["class".to_string(), "test".to_string()]),
        };

        let json = serde_json::to_string(&node).unwrap();
        assert!(json.contains("\"nodeId\":1"));
        assert!(json.contains("\"nodeName\":\"div\""));
    }
}
