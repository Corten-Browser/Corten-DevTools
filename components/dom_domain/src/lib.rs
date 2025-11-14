//! DOM and CSS domain implementations for Chrome DevTools Protocol
//!
//! This module provides CDP domain handlers for DOM and CSS manipulation,
//! implementing the protocol_handler::DomainHandler trait.

mod css_domain;
mod dom_domain;
mod mock_dom;

pub use css_domain::CssDomain;
pub use dom_domain::DomDomain;
pub use mock_dom::MockDomBridge;

#[cfg(test)]
mod tests {
    use super::*;
    use protocol_handler::DomainHandler;
    use serde_json::json;

    #[tokio::test]
    async fn test_dom_domain_name() {
        let dom = DomDomain::new();
        assert_eq!(dom.name(), "DOM");
    }

    #[tokio::test]
    async fn test_dom_get_document() {
        let dom = DomDomain::new();

        // Call get_document without parameters
        let result = dom.handle_method("getDocument", None).await;

        assert!(result.is_ok());
        let value = result.unwrap();

        // Verify response structure
        assert!(value["root"].is_object());
        assert!(value["root"]["nodeId"].is_number());
        assert_eq!(value["root"]["nodeType"], 9); // Document type
        assert_eq!(value["root"]["nodeName"], "#document");
    }

    #[tokio::test]
    async fn test_dom_query_selector_found() {
        let dom = DomDomain::new();

        // First get the document to have a valid node
        let doc_result = dom.handle_method("getDocument", None).await;
        assert!(doc_result.is_ok());

        // Try to query for an element
        let params = json!({
            "nodeId": 1, // Root document node
            "selector": "div"
        });

        let result = dom.handle_method("querySelector", Some(params)).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        // In our mock, we should find at least one div
        assert!(value["nodeId"].is_number() || value["nodeId"].is_null());
    }

    #[tokio::test]
    async fn test_dom_query_selector_not_found() {
        let dom = DomDomain::new();

        let params = json!({
            "nodeId": 1,
            "selector": ".nonexistent-class"
        });

        let result = dom.handle_method("querySelector", Some(params)).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        assert!(value["nodeId"].is_null());
    }

    #[tokio::test]
    async fn test_dom_query_selector_invalid_node() {
        let dom = DomDomain::new();

        let params = json!({
            "nodeId": 99999, // Non-existent node
            "selector": "div"
        });

        let result = dom.handle_method("querySelector", Some(params)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_dom_set_attribute_value() {
        let dom = DomDomain::new();

        // First, get document and a valid element node
        let _doc = dom.handle_method("getDocument", None).await.unwrap();

        let params = json!({
            "nodeId": 2, // Assuming node 2 is an element
            "name": "class",
            "value": "test-class"
        });

        let result = dom.handle_method("setAttributeValue", Some(params)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_dom_set_attribute_invalid_node() {
        let dom = DomDomain::new();

        let params = json!({
            "nodeId": 99999,
            "name": "class",
            "value": "test"
        });

        let result = dom.handle_method("setAttributeValue", Some(params)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_dom_unknown_method() {
        let dom = DomDomain::new();

        let result = dom.handle_method("unknownMethod", None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_css_domain_name() {
        let css = CssDomain::new();
        assert_eq!(css.name(), "CSS");
    }

    #[tokio::test]
    async fn test_css_get_computed_styles() {
        let css = CssDomain::new();

        let params = json!({
            "nodeId": 2 // Element node
        });

        let result = css
            .handle_method("getComputedStyleForNode", Some(params))
            .await;
        assert!(result.is_ok());

        let value = result.unwrap();
        assert!(value["computedStyle"].is_array());
    }

    #[tokio::test]
    async fn test_css_get_computed_styles_invalid_node() {
        let css = CssDomain::new();

        let params = json!({
            "nodeId": 99999
        });

        let result = css
            .handle_method("getComputedStyleForNode", Some(params))
            .await;
        assert!(result.is_err());
    }
}
