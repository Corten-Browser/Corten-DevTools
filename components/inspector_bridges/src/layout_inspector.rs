//! Layout Inspector implementation
//!
//! Provides CSS layout and flexbox/grid inspection via Chrome DevTools Protocol.
//! Implements FEAT-023: Layout Inspector.
//!
//! Features:
//! - Flexbox overlay
//! - Grid overlay
//! - Box model visualization

use async_trait::async_trait;
use cdp_types::domains::dom::NodeId;
use cdp_types::CdpError;
use protocol_handler::DomainHandler;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn};

use crate::mock_browser::MockBrowser;
use crate::types::{BoxModel, RGBA};

/// Flexbox container info
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FlexContainerInfo {
    /// Node ID of the flex container
    pub node_id: NodeId,
    /// Flex direction (row, column, row-reverse, column-reverse)
    pub flex_direction: String,
    /// Flex wrap (nowrap, wrap, wrap-reverse)
    pub flex_wrap: String,
    /// Align items
    pub align_items: String,
    /// Justify content
    pub justify_content: String,
    /// Flex items
    pub flex_items: Vec<FlexItemInfo>,
}

/// Flexbox item info
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FlexItemInfo {
    /// Node ID of the flex item
    pub node_id: NodeId,
    /// Flex grow value
    pub flex_grow: f64,
    /// Flex shrink value
    pub flex_shrink: f64,
    /// Flex basis
    pub flex_basis: String,
    /// Align self
    #[serde(skip_serializing_if = "Option::is_none")]
    pub align_self: Option<String>,
    /// Order
    pub order: i32,
}

/// Grid container info
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GridContainerInfo {
    /// Node ID of the grid container
    pub node_id: NodeId,
    /// Grid template columns
    pub grid_template_columns: String,
    /// Grid template rows
    pub grid_template_rows: String,
    /// Grid gap
    pub grid_gap: String,
    /// Column gap
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column_gap: Option<String>,
    /// Row gap
    #[serde(skip_serializing_if = "Option::is_none")]
    pub row_gap: Option<String>,
    /// Grid items
    pub grid_items: Vec<GridItemInfo>,
    /// Grid overlay colors
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overlay_colors: Option<GridOverlayColors>,
}

/// Grid item info
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GridItemInfo {
    /// Node ID of the grid item
    pub node_id: NodeId,
    /// Grid column start
    pub grid_column_start: i32,
    /// Grid column end
    pub grid_column_end: i32,
    /// Grid row start
    pub grid_row_start: i32,
    /// Grid row end
    pub grid_row_end: i32,
    /// Grid area name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grid_area: Option<String>,
}

/// Grid overlay colors
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GridOverlayColors {
    /// Color for grid lines
    pub grid_border_color: RGBA,
    /// Color for row lines
    pub row_line_color: RGBA,
    /// Color for column lines
    pub column_line_color: RGBA,
    /// Color for row gaps
    pub row_gap_color: RGBA,
    /// Color for column gaps
    pub column_gap_color: RGBA,
}

impl Default for GridOverlayColors {
    fn default() -> Self {
        Self {
            grid_border_color: RGBA::new(255, 0, 170, 1.0),
            row_line_color: RGBA::new(128, 0, 128, 0.8),
            column_line_color: RGBA::new(128, 0, 128, 0.8),
            row_gap_color: RGBA::new(255, 165, 0, 0.3),
            column_gap_color: RGBA::new(255, 165, 0, 0.3),
        }
    }
}

/// Flexbox overlay configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FlexOverlayConfig {
    /// Show flex items
    #[serde(default = "default_true")]
    pub show_items: bool,
    /// Show flex lines
    #[serde(default = "default_true")]
    pub show_lines: bool,
    /// Show item order
    #[serde(default)]
    pub show_order: bool,
    /// Flex container color
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_color: Option<RGBA>,
    /// Flex item color
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_color: Option<RGBA>,
}

fn default_true() -> bool {
    true
}

impl Default for FlexOverlayConfig {
    fn default() -> Self {
        Self {
            show_items: true,
            show_lines: true,
            show_order: false,
            container_color: Some(RGBA::new(147, 112, 219, 0.66)),
            item_color: Some(RGBA::new(175, 238, 238, 0.66)),
        }
    }
}

/// Grid overlay configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GridOverlayConfig {
    /// Show grid lines
    #[serde(default = "default_true")]
    pub show_grid_lines: bool,
    /// Show line names
    #[serde(default)]
    pub show_line_names: bool,
    /// Show track sizes
    #[serde(default)]
    pub show_track_sizes: bool,
    /// Show area names
    #[serde(default)]
    pub show_area_names: bool,
    /// Overlay colors
    #[serde(skip_serializing_if = "Option::is_none")]
    pub colors: Option<GridOverlayColors>,
}

impl Default for GridOverlayConfig {
    fn default() -> Self {
        Self {
            show_grid_lines: true,
            show_line_names: false,
            show_track_sizes: false,
            show_area_names: false,
            colors: Some(GridOverlayColors::default()),
        }
    }
}

/// Layout overlay state
#[derive(Debug, Clone, Default)]
pub struct LayoutOverlayState {
    /// Active flex overlays (node_id -> config)
    pub flex_overlays: HashMap<NodeId, FlexOverlayConfig>,
    /// Active grid overlays (node_id -> config)
    pub grid_overlays: HashMap<NodeId, GridOverlayConfig>,
    /// Whether layout debugging is enabled
    pub enabled: bool,
}

/// Layout Inspector
///
/// Provides CSS layout inspection including flexbox and grid overlays,
/// box model visualization, and layout debugging tools.
pub struct LayoutInspector {
    /// Mock browser for testing
    browser: Arc<MockBrowser>,
    /// Layout overlay state
    overlay_state: Arc<RwLock<LayoutOverlayState>>,
}

impl LayoutInspector {
    /// Create a new Layout Inspector
    pub fn new() -> Self {
        Self {
            browser: Arc::new(MockBrowser::new()),
            overlay_state: Arc::new(RwLock::new(LayoutOverlayState::default())),
        }
    }

    /// Create with custom browser (for testing)
    pub fn with_browser(browser: Arc<MockBrowser>) -> Self {
        Self {
            browser,
            overlay_state: Arc::new(RwLock::new(LayoutOverlayState::default())),
        }
    }

    /// Get flexbox container info
    async fn get_flex_container_info(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("LayoutInspector.getFlexContainerInfo called");

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Params {
            node_id: NodeId,
        }

        let params: Params = serde_json::from_value(
            params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?,
        )
        .map_err(|e| CdpError::invalid_params(format!("Invalid parameters: {}", e)))?;

        if !self.browser.node_exists(params.node_id) {
            return Err(CdpError::server_error(
                -32000,
                format!("Node {} not found", params.node_id.0),
            ));
        }

        // Return mock flex container info
        let flex_info = FlexContainerInfo {
            node_id: params.node_id,
            flex_direction: "row".to_string(),
            flex_wrap: "nowrap".to_string(),
            align_items: "stretch".to_string(),
            justify_content: "flex-start".to_string(),
            flex_items: vec![
                FlexItemInfo {
                    node_id: NodeId(params.node_id.0 + 100),
                    flex_grow: 1.0,
                    flex_shrink: 1.0,
                    flex_basis: "auto".to_string(),
                    align_self: None,
                    order: 0,
                },
                FlexItemInfo {
                    node_id: NodeId(params.node_id.0 + 101),
                    flex_grow: 0.0,
                    flex_shrink: 1.0,
                    flex_basis: "200px".to_string(),
                    align_self: Some("center".to_string()),
                    order: 1,
                },
            ],
        };

        serde_json::to_value(flex_info)
            .map_err(|e| CdpError::internal_error(format!("Serialization error: {}", e)))
    }

    /// Get grid container info
    async fn get_grid_container_info(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("LayoutInspector.getGridContainerInfo called");

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Params {
            node_id: NodeId,
        }

        let params: Params = serde_json::from_value(
            params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?,
        )
        .map_err(|e| CdpError::invalid_params(format!("Invalid parameters: {}", e)))?;

        if !self.browser.node_exists(params.node_id) {
            return Err(CdpError::server_error(
                -32000,
                format!("Node {} not found", params.node_id.0),
            ));
        }

        // Return mock grid container info
        let grid_info = GridContainerInfo {
            node_id: params.node_id,
            grid_template_columns: "1fr 2fr 1fr".to_string(),
            grid_template_rows: "auto 1fr auto".to_string(),
            grid_gap: "10px".to_string(),
            column_gap: Some("10px".to_string()),
            row_gap: Some("10px".to_string()),
            grid_items: vec![
                GridItemInfo {
                    node_id: NodeId(params.node_id.0 + 100),
                    grid_column_start: 1,
                    grid_column_end: 2,
                    grid_row_start: 1,
                    grid_row_end: 2,
                    grid_area: Some("header".to_string()),
                },
                GridItemInfo {
                    node_id: NodeId(params.node_id.0 + 101),
                    grid_column_start: 1,
                    grid_column_end: 4,
                    grid_row_start: 2,
                    grid_row_end: 3,
                    grid_area: Some("main".to_string()),
                },
            ],
            overlay_colors: Some(GridOverlayColors::default()),
        };

        serde_json::to_value(grid_info)
            .map_err(|e| CdpError::internal_error(format!("Serialization error: {}", e)))
    }

    /// Get box model for a node
    async fn get_box_model(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("LayoutInspector.getBoxModel called");

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Params {
            node_id: NodeId,
        }

        let params: Params = serde_json::from_value(
            params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?,
        )
        .map_err(|e| CdpError::invalid_params(format!("Invalid parameters: {}", e)))?;

        let box_model = self.browser.get_box_model(params.node_id).ok_or_else(|| {
            CdpError::server_error(-32000, format!("Node {} not found", params.node_id.0))
        })?;

        Ok(serde_json::json!({
            "model": box_model
        }))
    }

    /// Enable flex overlay for a container
    async fn show_flex_overlay(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("LayoutInspector.showFlexOverlay called");

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Params {
            node_id: NodeId,
            #[serde(default)]
            config: Option<FlexOverlayConfig>,
        }

        let params: Params = serde_json::from_value(
            params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?,
        )
        .map_err(|e| CdpError::invalid_params(format!("Invalid parameters: {}", e)))?;

        if !self.browser.node_exists(params.node_id) {
            return Err(CdpError::server_error(
                -32000,
                format!("Node {} not found", params.node_id.0),
            ));
        }

        let config = params.config.unwrap_or_default();

        let mut state = self.overlay_state.write().await;
        state.flex_overlays.insert(params.node_id, config.clone());

        Ok(serde_json::json!({
            "nodeId": params.node_id,
            "overlayEnabled": true,
            "config": config
        }))
    }

    /// Disable flex overlay for a container
    async fn hide_flex_overlay(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("LayoutInspector.hideFlexOverlay called");

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Params {
            node_id: NodeId,
        }

        let params: Params = serde_json::from_value(
            params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?,
        )
        .map_err(|e| CdpError::invalid_params(format!("Invalid parameters: {}", e)))?;

        let mut state = self.overlay_state.write().await;
        state.flex_overlays.remove(&params.node_id);

        Ok(serde_json::json!({
            "nodeId": params.node_id,
            "overlayEnabled": false
        }))
    }

    /// Enable grid overlay for a container
    async fn show_grid_overlay(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("LayoutInspector.showGridOverlay called");

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Params {
            node_id: NodeId,
            #[serde(default)]
            config: Option<GridOverlayConfig>,
        }

        let params: Params = serde_json::from_value(
            params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?,
        )
        .map_err(|e| CdpError::invalid_params(format!("Invalid parameters: {}", e)))?;

        if !self.browser.node_exists(params.node_id) {
            return Err(CdpError::server_error(
                -32000,
                format!("Node {} not found", params.node_id.0),
            ));
        }

        let config = params.config.unwrap_or_default();

        let mut state = self.overlay_state.write().await;
        state.grid_overlays.insert(params.node_id, config.clone());

        Ok(serde_json::json!({
            "nodeId": params.node_id,
            "overlayEnabled": true,
            "config": config
        }))
    }

    /// Disable grid overlay for a container
    async fn hide_grid_overlay(&self, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("LayoutInspector.hideGridOverlay called");

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Params {
            node_id: NodeId,
        }

        let params: Params = serde_json::from_value(
            params.ok_or_else(|| CdpError::invalid_params("Missing parameters"))?,
        )
        .map_err(|e| CdpError::invalid_params(format!("Invalid parameters: {}", e)))?;

        let mut state = self.overlay_state.write().await;
        state.grid_overlays.remove(&params.node_id);

        Ok(serde_json::json!({
            "nodeId": params.node_id,
            "overlayEnabled": false
        }))
    }

    /// Get all active overlays
    async fn get_active_overlays(&self, _params: Option<Value>) -> Result<Value, CdpError> {
        debug!("LayoutInspector.getActiveOverlays called");

        let state = self.overlay_state.read().await;

        let flex_nodes: Vec<u32> = state.flex_overlays.keys().map(|id| id.0).collect();
        let grid_nodes: Vec<u32> = state.grid_overlays.keys().map(|id| id.0).collect();

        Ok(serde_json::json!({
            "flexOverlays": flex_nodes,
            "gridOverlays": grid_nodes
        }))
    }

    /// Clear all overlays
    async fn clear_all_overlays(&self, _params: Option<Value>) -> Result<Value, CdpError> {
        debug!("LayoutInspector.clearAllOverlays called");

        let mut state = self.overlay_state.write().await;
        state.flex_overlays.clear();
        state.grid_overlays.clear();

        Ok(serde_json::json!({}))
    }

    /// Enable layout debugging
    async fn enable(&self, _params: Option<Value>) -> Result<Value, CdpError> {
        debug!("LayoutInspector.enable called");

        let mut state = self.overlay_state.write().await;
        state.enabled = true;

        Ok(serde_json::json!({}))
    }

    /// Disable layout debugging
    async fn disable(&self, _params: Option<Value>) -> Result<Value, CdpError> {
        debug!("LayoutInspector.disable called");

        let mut state = self.overlay_state.write().await;
        state.enabled = false;
        state.flex_overlays.clear();
        state.grid_overlays.clear();

        Ok(serde_json::json!({}))
    }

    /// Get overlay state
    pub async fn get_overlay_state(&self) -> LayoutOverlayState {
        self.overlay_state.read().await.clone()
    }

    /// Get box model directly
    pub fn get_element_box_model(&self, node_id: NodeId) -> Option<BoxModel> {
        self.browser.get_box_model(node_id)
    }

    /// Get browser (for testing)
    pub fn browser(&self) -> &MockBrowser {
        &self.browser
    }
}

impl Default for LayoutInspector {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DomainHandler for LayoutInspector {
    fn name(&self) -> &str {
        "LayoutInspector"
    }

    async fn handle_method(&self, method: &str, params: Option<Value>) -> Result<Value, CdpError> {
        debug!("LayoutInspector domain handling method: {}", method);

        match method {
            "enable" => self.enable(params).await,
            "disable" => self.disable(params).await,
            "getFlexContainerInfo" => self.get_flex_container_info(params).await,
            "getGridContainerInfo" => self.get_grid_container_info(params).await,
            "getBoxModel" => self.get_box_model(params).await,
            "showFlexOverlay" => self.show_flex_overlay(params).await,
            "hideFlexOverlay" => self.hide_flex_overlay(params).await,
            "showGridOverlay" => self.show_grid_overlay(params).await,
            "hideGridOverlay" => self.hide_grid_overlay(params).await,
            "getActiveOverlays" => self.get_active_overlays(params).await,
            "clearAllOverlays" => self.clear_all_overlays(params).await,
            _ => {
                warn!("Unknown LayoutInspector method: {}", method);
                Err(CdpError::method_not_found(format!(
                    "LayoutInspector.{}",
                    method
                )))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_domain_name() {
        let inspector = LayoutInspector::new();
        assert_eq!(inspector.name(), "LayoutInspector");
    }

    #[tokio::test]
    async fn test_enable_disable() {
        let inspector = LayoutInspector::new();

        let result = inspector.enable(None).await;
        assert!(result.is_ok());

        let state = inspector.get_overlay_state().await;
        assert!(state.enabled);

        let result = inspector.disable(None).await;
        assert!(result.is_ok());

        let state = inspector.get_overlay_state().await;
        assert!(!state.enabled);
    }

    #[tokio::test]
    async fn test_get_flex_container_info() {
        let inspector = LayoutInspector::new();
        let params = json!({ "nodeId": 6 });

        let result = inspector.get_flex_container_info(Some(params)).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        assert_eq!(value["nodeId"], 6);
        assert!(value["flexDirection"].is_string());
        assert!(value["flexItems"].is_array());
    }

    #[tokio::test]
    async fn test_get_flex_container_info_not_found() {
        let inspector = LayoutInspector::new();
        let params = json!({ "nodeId": 99999 });

        let result = inspector.get_flex_container_info(Some(params)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_grid_container_info() {
        let inspector = LayoutInspector::new();
        let params = json!({ "nodeId": 6 });

        let result = inspector.get_grid_container_info(Some(params)).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        assert_eq!(value["nodeId"], 6);
        assert!(value["gridTemplateColumns"].is_string());
        assert!(value["gridItems"].is_array());
    }

    #[tokio::test]
    async fn test_get_box_model() {
        let inspector = LayoutInspector::new();
        let params = json!({ "nodeId": 6 });

        let result = inspector.get_box_model(Some(params)).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        assert!(value["model"]["content"].is_array());
        assert!(value["model"]["width"].is_number());
    }

    #[tokio::test]
    async fn test_show_flex_overlay() {
        let inspector = LayoutInspector::new();
        let params = json!({
            "nodeId": 6,
            "config": {
                "showItems": true,
                "showLines": true
            }
        });

        let result = inspector.show_flex_overlay(Some(params)).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        assert!(value["overlayEnabled"].as_bool().unwrap());

        let state = inspector.get_overlay_state().await;
        assert!(state.flex_overlays.contains_key(&NodeId(6)));
    }

    #[tokio::test]
    async fn test_hide_flex_overlay() {
        let inspector = LayoutInspector::new();

        // First show overlay
        let show_params = json!({ "nodeId": 6 });
        inspector.show_flex_overlay(Some(show_params)).await.unwrap();

        // Then hide it
        let hide_params = json!({ "nodeId": 6 });
        let result = inspector.hide_flex_overlay(Some(hide_params)).await;
        assert!(result.is_ok());

        let state = inspector.get_overlay_state().await;
        assert!(!state.flex_overlays.contains_key(&NodeId(6)));
    }

    #[tokio::test]
    async fn test_show_grid_overlay() {
        let inspector = LayoutInspector::new();
        let params = json!({
            "nodeId": 6,
            "config": {
                "showGridLines": true,
                "showAreaNames": true
            }
        });

        let result = inspector.show_grid_overlay(Some(params)).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        assert!(value["overlayEnabled"].as_bool().unwrap());

        let state = inspector.get_overlay_state().await;
        assert!(state.grid_overlays.contains_key(&NodeId(6)));
    }

    #[tokio::test]
    async fn test_hide_grid_overlay() {
        let inspector = LayoutInspector::new();

        // First show overlay
        let show_params = json!({ "nodeId": 6 });
        inspector.show_grid_overlay(Some(show_params)).await.unwrap();

        // Then hide it
        let hide_params = json!({ "nodeId": 6 });
        let result = inspector.hide_grid_overlay(Some(hide_params)).await;
        assert!(result.is_ok());

        let state = inspector.get_overlay_state().await;
        assert!(!state.grid_overlays.contains_key(&NodeId(6)));
    }

    #[tokio::test]
    async fn test_get_active_overlays() {
        let inspector = LayoutInspector::new();

        // Add some overlays
        let params1 = json!({ "nodeId": 6 });
        let params2 = json!({ "nodeId": 7 });

        inspector.show_flex_overlay(Some(params1)).await.unwrap();
        inspector.show_grid_overlay(Some(params2)).await.unwrap();

        let result = inspector.get_active_overlays(None).await;
        assert!(result.is_ok());

        let value = result.unwrap();
        assert!(value["flexOverlays"].as_array().unwrap().contains(&json!(6)));
        assert!(value["gridOverlays"].as_array().unwrap().contains(&json!(7)));
    }

    #[tokio::test]
    async fn test_clear_all_overlays() {
        let inspector = LayoutInspector::new();

        // Add some overlays
        let params1 = json!({ "nodeId": 6 });
        let params2 = json!({ "nodeId": 7 });

        inspector.show_flex_overlay(Some(params1)).await.unwrap();
        inspector.show_grid_overlay(Some(params2)).await.unwrap();

        // Clear all
        let result = inspector.clear_all_overlays(None).await;
        assert!(result.is_ok());

        let state = inspector.get_overlay_state().await;
        assert!(state.flex_overlays.is_empty());
        assert!(state.grid_overlays.is_empty());
    }

    #[tokio::test]
    async fn test_unknown_method() {
        let inspector = LayoutInspector::new();
        let result = inspector.handle_method("unknownMethod", None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_direct_api_get_box_model() {
        let inspector = LayoutInspector::new();
        let box_model = inspector.get_element_box_model(NodeId(6));
        assert!(box_model.is_some());

        let bm = box_model.unwrap();
        assert!(bm.width > 0.0);
        assert!(bm.height > 0.0);
    }
}
