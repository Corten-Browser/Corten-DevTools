//! Main DevTools component implementation

use crate::{DevToolsConfig, DevToolsError, Result};
use cdp_server::{CdpWebSocketServer, ServerConfig};
use protocol_handler::ProtocolHandler;
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tracing::{debug, error, info};

// Import all domain handlers
use browser_page_domains::{BrowserDomain, EmulationDomain, PageDomain, SecurityDomain};
use console_storage::{ConsoleDomain, StorageDomain};
use dom_domain::{CssDomain, DomDomain};
use network_domain::NetworkDomain;
use profiler_domains::{HeapProfilerDomain, ProfilerDomain};
use runtime_debugger::{DebuggerDomain, RuntimeDomain};

/// Main DevTools component that orchestrates all domains and the CDP server
///
/// This component is responsible for:
/// - Managing the WebSocket server lifecycle
/// - Registering all CDP domain handlers
/// - Routing messages to appropriate handlers
/// - Providing public API for DevTools functionality
///
/// # Example
///
/// ```no_run
/// use devtools_component::{DevToolsComponent, DevToolsConfig};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = DevToolsConfig::builder()
///         .port(9222)
///         .build();
///
///     let devtools = DevToolsComponent::new(config)?;
///     devtools.start().await?;
///
///     // DevTools server is now running and ready for connections
///     println!("DevTools available at: {}", devtools.get_json_url());
///
///     Ok(())
/// }
/// ```
pub struct DevToolsComponent {
    /// Configuration for this component
    config: DevToolsConfig,

    /// Protocol handler that routes messages to domains
    /// Note: Currently registered with all domains but not yet integrated with server message handling.
    /// Future enhancement will connect this to the server's message processing.
    #[allow(dead_code)]
    protocol_handler: Arc<ProtocolHandler>,

    /// Server task handle (when running)
    server_handle: Arc<RwLock<Option<JoinHandle<()>>>>,

    /// Whether the server is currently running
    running: Arc<AtomicBool>,

    /// Actual port the server is bound to (may differ from config if using ephemeral port)
    actual_port: Arc<AtomicU16>,
}

impl DevToolsComponent {
    /// Create a new DevToolsComponent with the given configuration
    ///
    /// This will:
    /// 1. Create the protocol handler
    /// 2. Register all 13 CDP domain handlers
    /// 3. Create the WebSocket server (but not start it)
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration for the DevTools component
    ///
    /// # Returns
    ///
    /// A Result containing the DevToolsComponent or an error
    ///
    /// # Example
    ///
    /// ```
    /// use devtools_component::{DevToolsComponent, DevToolsConfig};
    ///
    /// let config = DevToolsConfig::default();
    /// let devtools = DevToolsComponent::new(config).unwrap();
    /// ```
    pub fn new(config: DevToolsConfig) -> Result<Self> {
        debug!("Creating DevToolsComponent with config: {:?}", config);

        // Create protocol handler
        let protocol_handler = Arc::new(ProtocolHandler::new());

        // Register all domains
        Self::register_all_domains(&protocol_handler)?;

        Ok(Self {
            config,
            protocol_handler,
            server_handle: Arc::new(RwLock::new(None)),
            running: Arc::new(AtomicBool::new(false)),
            actual_port: Arc::new(AtomicU16::new(0)),
        })
    }

    /// Register all CDP domain handlers
    ///
    /// This registers all 13 domains:
    /// - Browser, Page, Security, Emulation (browser_page_domains)
    /// - DOM, CSS (dom_domain)
    /// - Network (network_domain)
    /// - Runtime, Debugger (runtime_debugger)
    /// - Profiler, HeapProfiler (profiler_domains)
    /// - Console, Storage (console_storage)
    fn register_all_domains(handler: &Arc<ProtocolHandler>) -> Result<()> {
        debug!("Registering all domain handlers");

        // Browser/Page domains (4)
        handler.register_domain(Arc::new(BrowserDomain::new()));
        handler.register_domain(Arc::new(PageDomain::new()));
        handler.register_domain(Arc::new(SecurityDomain::new()));
        handler.register_domain(Arc::new(EmulationDomain::new()));

        // DOM domains (2)
        handler.register_domain(Arc::new(DomDomain::new()));
        handler.register_domain(Arc::new(CssDomain::new()));

        // Network domain (1)
        handler.register_domain(Arc::new(NetworkDomain::new()));

        // Runtime/Debugger domains (2)
        handler.register_domain(Arc::new(RuntimeDomain::new()));
        handler.register_domain(Arc::new(DebuggerDomain::new()));

        // Profiler domains (2)
        handler.register_domain(Arc::new(ProfilerDomain::new()));
        handler.register_domain(Arc::new(HeapProfilerDomain::new()));

        // Console/Storage domains (2)
        handler.register_domain(Arc::new(ConsoleDomain::new()));
        handler.register_domain(Arc::new(StorageDomain::new()));

        info!("Successfully registered 13 CDP domain handlers");

        Ok(())
    }

    /// Start the DevTools server
    ///
    /// This will start the WebSocket server and begin accepting connections.
    ///
    /// # Returns
    ///
    /// A Result indicating success or failure
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The server is already running
    /// - Failed to bind to the configured port
    /// - Server initialization failed
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use devtools_component::{DevToolsComponent, DevToolsConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let devtools = DevToolsComponent::new(DevToolsConfig::default())?;
    /// devtools.start().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn start(&self) -> Result<()> {
        // Check if already running
        if self.running.load(Ordering::SeqCst) {
            return Err(DevToolsError::ServerAlreadyRunning);
        }

        info!("Starting DevTools server on port {}", self.config.port());

        // Get actual port by binding a TcpListener first
        let addr = format!("127.0.0.1:{}", self.config.port());
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        let actual_port = listener.local_addr()?.port();

        info!("Bound to port {}", actual_port);

        // Store actual port (important for ephemeral ports)
        self.actual_port.store(actual_port, Ordering::SeqCst);

        // Mark as running before spawning task
        self.running.store(true, Ordering::SeqCst);

        // Create server configuration
        let server_config = ServerConfig {
            port: actual_port,
            bind_address: "127.0.0.1".to_string(),
            allowed_origins: self.config.allowed_origins().to_vec(),
            max_message_size: self.config.max_message_size(),
        };

        // Create server
        let server = CdpWebSocketServer::new(server_config)?;

        // Spawn server in background task
        // Note: We drop the listener here - the server will create its own
        drop(listener);

        let handle = tokio::spawn(async move {
            if let Err(e) = server.start().await {
                error!("Server error: {}", e);
            }
        });

        // Store task handle
        *self.server_handle.write().await = Some(handle);

        info!(
            "DevTools server started successfully on port {}",
            actual_port
        );
        info!("WebSocket URL: ws://localhost:{}", actual_port);
        info!("JSON endpoint: http://localhost:{}/json", actual_port);

        Ok(())
    }

    /// Stop the DevTools server
    ///
    /// This will gracefully shut down the WebSocket server and close all connections.
    ///
    /// # Returns
    ///
    /// A Result indicating success or failure
    ///
    /// # Errors
    ///
    /// Returns an error if the server is not running
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use devtools_component::{DevToolsComponent, DevToolsConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let devtools = DevToolsComponent::new(DevToolsConfig::default())?;
    /// devtools.start().await?;
    ///
    /// // Later...
    /// devtools.stop().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn stop(&self) -> Result<()> {
        // Check if running
        if !self.running.load(Ordering::SeqCst) {
            return Err(DevToolsError::ServerNotRunning);
        }

        info!("Stopping DevTools server");

        // Abort the server task
        if let Some(handle) = self.server_handle.write().await.take() {
            handle.abort();
            // Wait for the task to finish (it should abort quickly)
            let _ = handle.await;
        }

        // Mark as not running
        self.running.store(false, Ordering::SeqCst);
        self.actual_port.store(0, Ordering::SeqCst);

        info!("DevTools server stopped successfully");

        Ok(())
    }

    /// Check if the server is currently running
    ///
    /// # Returns
    ///
    /// true if the server is running, false otherwise
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Get the actual port the server is bound to
    ///
    /// This may differ from the configured port if using an ephemeral port (port 0).
    ///
    /// # Returns
    ///
    /// Some(port) if the server is running, None otherwise
    pub fn actual_port(&self) -> Option<u16> {
        if self.is_running() {
            let port = self.actual_port.load(Ordering::SeqCst);
            if port > 0 {
                Some(port)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Get the configuration used by this component
    ///
    /// # Returns
    ///
    /// A reference to the DevToolsConfig
    pub fn config(&self) -> &DevToolsConfig {
        &self.config
    }

    /// Get list of registered domain names
    ///
    /// # Returns
    ///
    /// A vector of domain names that are registered
    ///
    /// # Example
    ///
    /// ```
    /// # use devtools_component::{DevToolsComponent, DevToolsConfig};
    /// let devtools = DevToolsComponent::new(DevToolsConfig::default()).unwrap();
    /// let domains = devtools.registered_domains();
    ///
    /// assert!(domains.contains(&"DOM"));
    /// assert!(domains.contains(&"Network"));
    /// ```
    pub fn registered_domains(&self) -> Vec<&'static str> {
        vec![
            "Browser",
            "Page",
            "Security",
            "Emulation",
            "DOM",
            "CSS",
            "Network",
            "Runtime",
            "Debugger",
            "Profiler",
            "HeapProfiler",
            "Console",
            "Storage",
        ]
    }

    /// Get the WebSocket debugger URL for a specific target
    ///
    /// # Arguments
    ///
    /// * `target_id` - Identifier for the debugging target (e.g., page ID)
    ///
    /// # Returns
    ///
    /// The WebSocket URL for debugging the specified target
    ///
    /// # Example
    ///
    /// ```
    /// # use devtools_component::{DevToolsComponent, DevToolsConfig};
    /// let config = DevToolsConfig::builder().port(9222).build();
    /// let devtools = DevToolsComponent::new(config).unwrap();
    ///
    /// let url = devtools.get_debugger_url("page-123");
    /// assert_eq!(url, "ws://localhost:9222/devtools/page/page-123");
    /// ```
    pub fn get_debugger_url(&self, target_id: &str) -> String {
        let port = self.actual_port().unwrap_or(self.config.port());
        format!("ws://localhost:{}/devtools/page/{}", port, target_id)
    }

    /// Get the JSON endpoint URL
    ///
    /// The JSON endpoint provides information about available debugging targets.
    ///
    /// # Returns
    ///
    /// The HTTP URL for the JSON endpoint
    ///
    /// # Example
    ///
    /// ```
    /// # use devtools_component::{DevToolsComponent, DevToolsConfig};
    /// let config = DevToolsConfig::builder().port(9222).build();
    /// let devtools = DevToolsComponent::new(config).unwrap();
    ///
    /// let url = devtools.get_json_url();
    /// assert_eq!(url, "http://localhost:9222/json");
    /// ```
    pub fn get_json_url(&self) -> String {
        let port = self.actual_port().unwrap_or(self.config.port());
        format!("http://localhost:{}/json", port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_component() {
        let config = DevToolsConfig::default();
        let result = DevToolsComponent::new(config);

        assert!(result.is_ok());
    }

    #[test]
    fn test_component_stores_config() {
        let config = DevToolsConfig::builder().port(8888).build();
        let component = DevToolsComponent::new(config).unwrap();

        assert_eq!(component.config().port(), 8888);
    }

    #[test]
    fn test_registered_domains() {
        let component = DevToolsComponent::new(DevToolsConfig::default()).unwrap();
        let domains = component.registered_domains();

        assert_eq!(domains.len(), 13);
        assert!(domains.contains(&"Browser"));
        assert!(domains.contains(&"Page"));
        assert!(domains.contains(&"Security"));
        assert!(domains.contains(&"Emulation"));
        assert!(domains.contains(&"DOM"));
        assert!(domains.contains(&"CSS"));
        assert!(domains.contains(&"Network"));
        assert!(domains.contains(&"Runtime"));
        assert!(domains.contains(&"Debugger"));
        assert!(domains.contains(&"Profiler"));
        assert!(domains.contains(&"HeapProfiler"));
        assert!(domains.contains(&"Console"));
        assert!(domains.contains(&"Storage"));
    }

    #[test]
    fn test_initially_not_running() {
        let component = DevToolsComponent::new(DevToolsConfig::default()).unwrap();

        assert!(!component.is_running());
        assert!(component.actual_port().is_none());
    }

    #[test]
    fn test_get_debugger_url() {
        let config = DevToolsConfig::builder().port(9222).build();
        let component = DevToolsComponent::new(config).unwrap();

        let url = component.get_debugger_url("test-page");
        assert_eq!(url, "ws://localhost:9222/devtools/page/test-page");
    }

    #[test]
    fn test_get_json_url() {
        let config = DevToolsConfig::builder().port(9222).build();
        let component = DevToolsComponent::new(config).unwrap();

        let url = component.get_json_url();
        assert_eq!(url, "http://localhost:9222/json");
    }

    #[tokio::test]
    async fn test_start_stop_lifecycle() {
        let config = DevToolsConfig::builder().port(0).build();
        let component = DevToolsComponent::new(config).unwrap();

        // Initially not running
        assert!(!component.is_running());

        // Start
        let start_result = component.start().await;
        assert!(start_result.is_ok(), "Start failed: {:?}", start_result);
        assert!(component.is_running());
        assert!(component.actual_port().is_some());

        // Stop
        let stop_result = component.stop().await;
        assert!(stop_result.is_ok(), "Stop failed: {:?}", stop_result);
        assert!(!component.is_running());
        assert!(component.actual_port().is_none());
    }

    #[tokio::test]
    async fn test_cannot_start_twice() {
        let config = DevToolsConfig::builder().port(0).build();
        let component = DevToolsComponent::new(config).unwrap();

        component.start().await.unwrap();

        let result = component.start().await;
        assert!(result.is_err());

        match result {
            Err(DevToolsError::ServerAlreadyRunning) => {}
            _ => panic!("Expected ServerAlreadyRunning error"),
        }

        // Cleanup
        component.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_cannot_stop_when_not_running() {
        let component = DevToolsComponent::new(DevToolsConfig::default()).unwrap();

        let result = component.stop().await;
        assert!(result.is_err());

        match result {
            Err(DevToolsError::ServerNotRunning) => {}
            _ => panic!("Expected ServerNotRunning error"),
        }
    }

    #[tokio::test]
    async fn test_can_restart() {
        let config = DevToolsConfig::builder().port(0).build();
        let component = DevToolsComponent::new(config).unwrap();

        // First run
        component.start().await.unwrap();
        let first_port = component.actual_port();
        component.stop().await.unwrap();

        // Second run
        component.start().await.unwrap();
        let second_port = component.actual_port();
        component.stop().await.unwrap();

        // Both runs should have gotten valid ports
        assert!(first_port.is_some());
        assert!(second_port.is_some());
    }
}
