# DevTools Component Specification
## CortenBrowser Developer Tools Implementation
### Version 1.0

## Component Overview

### Purpose and Responsibilities
The DevTools component provides comprehensive debugging, profiling, and inspection capabilities for web developers using CortenBrowser. It implements the Chrome DevTools Protocol (CDP) to ensure compatibility with existing tooling while providing a Rust-native implementation optimized for the browser's architecture.

### Core Responsibilities
1. **Protocol Server**: WebSocket server implementing Chrome DevTools Protocol
2. **Component Inspection**: Deep integration with all browser components for state inspection
3. **Performance Profiling**: CPU, memory, and rendering performance analysis
4. **Network Monitoring**: Full request/response inspection and modification
5. **JavaScript Debugging**: Breakpoints, stepping, and scope inspection
6. **DOM/CSS Inspection**: Live editing and computed style analysis
7. **Console Implementation**: JavaScript REPL and logging infrastructure
8. **Storage Inspection**: Cookies, LocalStorage, IndexedDB, etc.
9. **Security Auditing**: Mixed content, certificate inspection, CSP violations
10. **Accessibility Testing**: ARIA validation and screen reader simulation

### Implementation Strategy
- **Phase 1**: Core CDP server with basic DOM/CSS inspection
- **Phase 2**: Network panel and JavaScript debugging
- **Phase 3**: Performance profiling and memory analysis
- **Phase 4**: Advanced features (PWA, WebAuthn, Extensions)

## Architecture

### High-Level Architecture
```rust
pub struct DevToolsComponent {
    // Core server
    cdp_server: CdpWebSocketServer,
    protocol_handler: ProtocolHandler,
    
    // Component bridges
    dom_bridge: DomInspectorBridge,
    network_bridge: NetworkInspectorBridge,
    js_bridge: JavaScriptDebugBridge,
    render_bridge: RenderInspectorBridge,
    
    // Session management
    sessions: HashMap<SessionId, DevToolsSession>,
    
    // Feature modules
    inspector: ElementsInspector,
    console: ConsoleImplementation,
    profiler: PerformanceProfiler,
    network_monitor: NetworkMonitor,
    debugger: JavaScriptDebugger,
    
    // State
    config: DevToolsConfig,
    metrics: DevToolsMetrics,
}
```

### Module Structure
```
devtools/
├── Cargo.toml
├── src/
│   ├── lib.rs                    # Public API
│   ├── component.rs               # BrowserComponent trait impl
│   ├── server/
│   │   ├── mod.rs                # WebSocket server
│   │   ├── cdp_server.rs         # CDP protocol server
│   │   ├── session.rs            # Client session management
│   │   └── transport.rs          # Message transport layer
│   ├── protocol/
│   │   ├── mod.rs                # CDP protocol definitions
│   │   ├── domains/              # CDP domain implementations
│   │   │   ├── browser.rs        # Browser domain
│   │   │   ├── debugger.rs       # Debugger domain
│   │   │   ├── dom.rs            # DOM domain
│   │   │   ├── css.rs            # CSS domain
│   │   │   ├── network.rs        # Network domain
│   │   │   ├── page.rs           # Page domain
│   │   │   ├── runtime.rs        # Runtime domain
│   │   │   ├── console.rs        # Console domain
│   │   │   ├── profiler.rs       # Profiler domain
│   │   │   ├── heap_profiler.rs  # HeapProfiler domain
│   │   │   ├── security.rs       # Security domain
│   │   │   ├── storage.rs        # Storage domain
│   │   │   └── emulation.rs      # Emulation domain
│   │   ├── types.rs              # CDP type definitions
│   │   └── events.rs             # Event definitions
│   ├── bridges/
│   │   ├── mod.rs                # Component integration
│   │   ├── dom_bridge.rs         # DOM component bridge
│   │   ├── network_bridge.rs     # Network stack bridge
│   │   ├── js_bridge.rs          # JS runtime bridge
│   │   ├── render_bridge.rs      # Render engine bridge
│   │   └── storage_bridge.rs     # Storage system bridge
│   ├── inspector/
│   │   ├── mod.rs                # Inspector features
│   │   ├── elements.rs           # Elements panel
│   │   ├── styles.rs             # Style computation
│   │   ├── layout.rs             # Layout inspection
│   │   └── accessibility.rs      # Accessibility tree
│   ├── debugger/
│   │   ├── mod.rs                # JS debugging
│   │   ├── breakpoints.rs        # Breakpoint management
│   │   ├── stepping.rs           # Step execution
│   │   ├── scope.rs              # Scope inspection
│   │   └── source_maps.rs        # Source map support
│   ├── network/
│   │   ├── mod.rs                # Network monitoring
│   │   ├── interceptor.rs        # Request interception
│   │   ├── cache.rs              # Cache inspection
│   │   └── websocket.rs          # WebSocket inspection
│   ├── profiler/
│   │   ├── mod.rs                # Performance profiling
│   │   ├── cpu_profiler.rs       # CPU profiling
│   │   ├── memory_profiler.rs    # Memory profiling
│   │   ├── timeline.rs           # Timeline recording
│   │   └── coverage.rs           # Code coverage
│   ├── console/
│   │   ├── mod.rs                # Console implementation
│   │   ├── repl.rs               # JavaScript REPL
│   │   ├── logging.rs            # Log management
│   │   └── formatting.rs         # Output formatting
│   └── utils/
│       ├── mod.rs
│       ├── serialization.rs      # CDP serialization
│       └── object_preview.rs     # Object preview generation
└── tests/
    ├── unit/
    ├── integration/
    └── cdp_compliance/
```

## Chrome DevTools Protocol Implementation

### Protocol Version
Target CDP version: 1.3 (latest stable)
Protocol documentation: https://chromedevtools.github.io/devtools-protocol/

### Core Domains Implementation

#### 1. Browser Domain
```rust
pub struct BrowserDomain {
    browser: Arc<BrowserShell>,
}

impl BrowserDomain {
    pub async fn get_version(&self) -> CdpResult<GetVersionResponse> {
        Ok(GetVersionResponse {
            protocol_version: "1.3".to_string(),
            product: "CortenBrowser/1.0".to_string(),
            revision: env!("GIT_HASH"),
            user_agent: self.browser.get_user_agent(),
            js_version: "V8/11.0".to_string(), // Or Rust JS engine version
        })
    }
    
    pub async fn get_browser_command_line(&self) -> CdpResult<GetBrowserCommandLineResponse> {
        Ok(GetBrowserCommandLineResponse {
            arguments: std::env::args().collect(),
        })
    }
}
```

#### 2. DOM Domain
```rust
pub struct DomDomain {
    dom_impl: Arc<DomImplementation>,
    node_map: HashMap<NodeId, DomNode>,
}

impl DomDomain {
    pub async fn get_document(&self) -> CdpResult<GetDocumentResponse> {
        let root = self.dom_impl.get_document_root();
        let node_id = self.register_node(root.clone());
        
        Ok(GetDocumentResponse {
            root: self.build_node_description(root, node_id),
        })
    }
    
    pub async fn query_selector(&self, params: QuerySelectorParams) -> CdpResult<QuerySelectorResponse> {
        let node = self.node_map.get(&params.node_id)
            .ok_or(CdpError::NodeNotFound)?;
        
        let result = node.query_selector(&params.selector)?;
        let node_id = result.map(|n| self.register_node(n));
        
        Ok(QuerySelectorResponse { node_id })
    }
    
    pub async fn set_attribute_value(&self, params: SetAttributeValueParams) -> CdpResult<()> {
        let node = self.node_map.get_mut(&params.node_id)
            .ok_or(CdpError::NodeNotFound)?;
        
        node.set_attribute(&params.name, &params.value)?;
        self.emit_dom_update_event(params.node_id);
        Ok(())
    }
}
```

#### 3. Network Domain
```rust
pub struct NetworkDomain {
    network_stack: Arc<NetworkStack>,
    request_map: HashMap<RequestId, RequestInfo>,
    interception_enabled: AtomicBool,
}

impl NetworkDomain {
    pub async fn enable(&self, params: EnableParams) -> CdpResult<()> {
        self.network_stack.set_observer(Box::new(NetworkObserver {
            devtools: self.clone(),
        }));
        
        if params.max_total_buffer_size.is_some() {
            self.network_stack.set_buffer_limit(params.max_total_buffer_size);
        }
        
        Ok(())
    }
    
    pub async fn get_response_body(&self, params: GetResponseBodyParams) -> CdpResult<GetResponseBodyResponse> {
        let request = self.request_map.get(&params.request_id)
            .ok_or(CdpError::RequestNotFound)?;
        
        Ok(GetResponseBodyResponse {
            body: request.response_body.clone(),
            base64_encoded: request.is_binary,
        })
    }
    
    pub async fn set_request_interception(&self, params: SetRequestInterceptionParams) -> CdpResult<()> {
        self.interception_enabled.store(params.enabled, Ordering::SeqCst);
        
        for pattern in params.patterns {
            self.network_stack.add_interception_pattern(InterceptionPattern {
                url_pattern: pattern.url_pattern,
                resource_type: pattern.resource_type,
                stage: pattern.interception_stage,
            })?;
        }
        
        Ok(())
    }
}
```

#### 4. Runtime Domain (JavaScript)
```rust
pub struct RuntimeDomain {
    js_runtime: Arc<JsRuntime>,
    object_map: HashMap<RemoteObjectId, JsValue>,
}

impl RuntimeDomain {
    pub async fn evaluate(&self, params: EvaluateParams) -> CdpResult<EvaluateResponse> {
        let context = params.context_id
            .and_then(|id| self.js_runtime.get_context(id))
            .unwrap_or_else(|| self.js_runtime.get_default_context());
        
        let result = context.evaluate(&params.expression, params.await_promise)?;
        let remote_object = self.create_remote_object(result);
        
        Ok(EvaluateResponse {
            result: remote_object,
            exception_details: None,
        })
    }
    
    pub async fn call_function_on(&self, params: CallFunctionOnParams) -> CdpResult<CallFunctionOnResponse> {
        let object = self.object_map.get(&params.object_id)
            .ok_or(CdpError::ObjectNotFound)?;
        
        let args = params.arguments
            .map(|args| self.resolve_call_arguments(args))
            .transpose()?;
        
        let result = object.call_function(&params.function_declaration, args)?;
        
        Ok(CallFunctionOnResponse {
            result: self.create_remote_object(result),
            exception_details: None,
        })
    }
}
```

### WebSocket Server Implementation

```rust
use tokio_tungstenite::{WebSocketStream, tungstenite::Message};

pub struct CdpWebSocketServer {
    listener: TcpListener,
    sessions: Arc<RwLock<HashMap<SessionId, Session>>>,
    protocol_handler: Arc<ProtocolHandler>,
}

impl CdpWebSocketServer {
    pub async fn start(&self, port: u16) -> Result<()> {
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        let listener = TcpListener::bind(addr).await?;
        
        info!("DevTools server listening on ws://localhost:{}", port);
        
        while let Ok((stream, addr)) = listener.accept().await {
            let sessions = self.sessions.clone();
            let handler = self.protocol_handler.clone();
            
            tokio::spawn(async move {
                if let Err(e) = Self::handle_connection(stream, addr, sessions, handler).await {
                    error!("WebSocket connection error: {}", e);
                }
            });
        }
        
        Ok(())
    }
    
    async fn handle_connection(
        stream: TcpStream,
        addr: SocketAddr,
        sessions: Arc<RwLock<HashMap<SessionId, Session>>>,
        handler: Arc<ProtocolHandler>,
    ) -> Result<()> {
        let ws_stream = accept_async(stream).await?;
        let session_id = SessionId::new();
        
        let session = Session::new(session_id, ws_stream.clone());
        sessions.write().await.insert(session_id, session.clone());
        
        let (tx, mut rx) = ws_stream.split();
        
        // Handle incoming messages
        while let Some(msg) = rx.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    let response = handler.handle_message(&text, &session).await;
                    session.send_message(response).await?;
                }
                Ok(Message::Close(_)) => break,
                _ => {}
            }
        }
        
        sessions.write().await.remove(&session_id);
        Ok(())
    }
}
```

## Component Integration Bridges

### DOM Bridge
```rust
pub struct DomInspectorBridge {
    dom_component: Arc<DomImplementation>,
    mutation_observer: Arc<MutationObserver>,
}

impl DomInspectorBridge {
    pub fn setup_observers(&self) -> Result<()> {
        self.mutation_observer.observe(MutationObserverConfig {
            attributes: true,
            child_list: true,
            subtree: true,
            attribute_old_value: true,
            character_data: true,
        });
        
        self.mutation_observer.set_callback(Box::new(move |mutations| {
            for mutation in mutations {
                self.handle_dom_mutation(mutation);
            }
        }));
        
        Ok(())
    }
    
    fn handle_dom_mutation(&self, mutation: DomMutation) {
        // Convert DOM mutation to CDP event
        let event = match mutation.mutation_type {
            MutationType::Attributes => CdpEvent::Dom(DomEvent::AttributeModified {
                node_id: mutation.target_node_id,
                name: mutation.attribute_name,
                value: mutation.new_value,
            }),
            MutationType::ChildList => CdpEvent::Dom(DomEvent::ChildNodeCountUpdated {
                node_id: mutation.target_node_id,
                child_node_count: mutation.child_count,
            }),
            _ => return,
        };
        
        self.broadcast_event(event);
    }
}
```

### Network Bridge
```rust
pub struct NetworkInspectorBridge {
    network_stack: Arc<NetworkStack>,
    request_tracker: Arc<RequestTracker>,
}

impl NetworkObserver for NetworkInspectorBridge {
    fn on_request_will_be_sent(&self, request: &HttpRequest) -> Result<()> {
        let request_id = self.request_tracker.track_request(request);
        
        self.emit_event(CdpEvent::Network(NetworkEvent::RequestWillBeSent {
            request_id,
            loader_id: LoaderId::new(),
            document_url: self.get_document_url(),
            request: self.convert_request(request),
            timestamp: Timestamp::now(),
            wall_time: WallTime::now(),
            initiator: self.get_initiator(),
        }));
        
        Ok(())
    }
    
    fn on_response_received(&self, request_id: RequestId, response: &HttpResponse) -> Result<()> {
        self.emit_event(CdpEvent::Network(NetworkEvent::ResponseReceived {
            request_id,
            loader_id: self.get_loader_id(request_id),
            timestamp: Timestamp::now(),
            type_: self.get_resource_type(response),
            response: self.convert_response(response),
        }));
        
        Ok(())
    }
}
```

## Feature Implementations

### Elements Inspector
```rust
pub struct ElementsInspector {
    dom: Arc<DomImplementation>,
    css: Arc<CssEngine>,
    render: Arc<RenderEngine>,
}

impl ElementsInspector {
    pub fn get_computed_styles(&self, node_id: NodeId) -> Result<ComputedStyles> {
        let node = self.dom.get_node(node_id)?;
        let styles = self.css.get_computed_style(&node)?;
        
        Ok(ComputedStyles {
            properties: styles.iter().map(|(k, v)| CssProperty {
                name: k.clone(),
                value: v.clone(),
                important: v.is_important(),
                implicit: false,
                text: format!("{}: {}", k, v),
                parsed_ok: true,
                disabled: false,
                range: None,
            }).collect(),
        })
    }
    
    pub fn get_box_model(&self, node_id: NodeId) -> Result<BoxModel> {
        let node = self.dom.get_node(node_id)?;
        let layout = self.render.get_layout_box(&node)?;
        
        Ok(BoxModel {
            content: self.quad_from_rect(layout.content_rect),
            padding: self.quad_from_rect(layout.padding_rect),
            border: self.quad_from_rect(layout.border_rect),
            margin: self.quad_from_rect(layout.margin_rect),
            width: layout.width,
            height: layout.height,
        })
    }
}
```

### JavaScript Debugger
```rust
pub struct JavaScriptDebugger {
    js_runtime: Arc<JsRuntime>,
    breakpoints: HashMap<BreakpointId, Breakpoint>,
    call_frames: Vec<CallFrame>,
    paused: AtomicBool,
}

impl JavaScriptDebugger {
    pub async fn set_breakpoint(&mut self, params: SetBreakpointParams) -> Result<SetBreakpointResponse> {
        let location = self.resolve_location(params.location)?;
        let breakpoint_id = BreakpointId::new();
        
        self.js_runtime.set_breakpoint(location.clone())?;
        
        self.breakpoints.insert(breakpoint_id.clone(), Breakpoint {
            id: breakpoint_id.clone(),
            location,
            condition: params.condition,
            hit_count: 0,
        });
        
        Ok(SetBreakpointResponse {
            breakpoint_id,
            actual_location: location,
        })
    }
    
    pub async fn step_over(&self) -> Result<()> {
        self.js_runtime.step_over()?;
        self.wait_for_pause().await
    }
    
    pub async fn evaluate_on_call_frame(&self, params: EvaluateOnCallFrameParams) -> Result<EvaluateOnCallFrameResponse> {
        let frame = self.call_frames.get(params.call_frame_id)
            .ok_or(Error::InvalidCallFrame)?;
        
        let result = frame.evaluate(&params.expression)?;
        
        Ok(EvaluateOnCallFrameResponse {
            result: self.create_remote_object(result),
            exception_details: None,
        })
    }
}
```

### Performance Profiler
```rust
pub struct PerformanceProfiler {
    cpu_profiler: CpuProfiler,
    memory_profiler: MemoryProfiler,
    timeline_recorder: TimelineRecorder,
}

impl PerformanceProfiler {
    pub async fn start_precise_coverage(&mut self) -> Result<()> {
        self.cpu_profiler.start_coverage(CoverageMode::Precise)?;
        Ok(())
    }
    
    pub async fn take_heap_snapshot(&self) -> Result<HeapSnapshot> {
        let snapshot = self.memory_profiler.take_snapshot()?;
        
        Ok(HeapSnapshot {
            nodes: snapshot.nodes,
            edges: snapshot.edges,
            strings: snapshot.strings,
            snapshot_id: SnapshotId::new(),
        })
    }
    
    pub async fn start_timeline_recording(&mut self, params: TimelineParams) -> Result<()> {
        self.timeline_recorder.start(TimelineConfig {
            enable_js_sampling: params.enable_js_sampling,
            enable_network: params.enable_network,
            enable_paint: params.enable_paint,
            buffer_size: params.buffer_size.unwrap_or(1000000),
        })?;
        
        Ok(())
    }
}
```

## API Specifications

### Public API
```rust
// devtools/src/lib.rs

pub struct DevTools {
    component: DevToolsComponent,
}

impl DevTools {
    /// Create new DevTools instance
    pub fn new(config: DevToolsConfig) -> Result<Self> {
        Ok(Self {
            component: DevToolsComponent::new(config)?,
        })
    }
    
    /// Start DevTools server on specified port
    pub async fn start(&self, port: u16) -> Result<()> {
        self.component.start_server(port).await
    }
    
    /// Connect to browser components
    pub fn connect_components(&mut self, components: BrowserComponents) -> Result<()> {
        self.component.connect_to_browser(components)
    }
    
    /// Get DevTools server URL
    pub fn get_url(&self) -> String {
        format!("http://localhost:{}/json", self.component.get_port())
    }
    
    /// Get WebSocket debugger URL for a specific target
    pub fn get_debugger_url(&self, target_id: &str) -> String {
        format!("ws://localhost:{}/devtools/page/{}", 
                self.component.get_port(), target_id)
    }
}

/// Configuration for DevTools
pub struct DevToolsConfig {
    pub port: u16,
    pub enable_remote_debugging: bool,
    pub allowed_origins: Vec<String>,
    pub max_message_size: usize,
    pub protocol_version: String,
}

impl Default for DevToolsConfig {
    fn default() -> Self {
        Self {
            port: 9222,
            enable_remote_debugging: false,
            allowed_origins: vec!["http://localhost:*".to_string()],
            max_message_size: 100 * 1024 * 1024, // 100MB
            protocol_version: "1.3".to_string(),
        }
    }
}
```

### Component Message Interface
```rust
impl BrowserComponent for DevToolsComponent {
    fn initialize(&mut self, config: ComponentConfig) -> Result<(), ComponentError> {
        self.config = config.parse_devtools_config()?;
        self.setup_protocol_handlers()?;
        Ok(())
    }
    
    fn handle_message(&mut self, msg: ComponentMessage) -> Result<ComponentResponse, ComponentError> {
        match msg {
            ComponentMessage::DevTools(DevToolsMessage::InspectElement { node_id }) => {
                self.inspector.inspect_element(node_id)?;
                Ok(ComponentResponse::Success)
            }
            ComponentMessage::DevTools(DevToolsMessage::BreakpointHit { location }) => {
                self.debugger.handle_breakpoint(location)?;
                Ok(ComponentResponse::Success)
            }
            ComponentMessage::DevTools(DevToolsMessage::NetworkRequest { request }) => {
                self.network_monitor.track_request(request)?;
                Ok(ComponentResponse::Success)
            }
            _ => Ok(ComponentResponse::NotHandled),
        }
    }
    
    fn health_check(&self) -> ComponentHealth {
        ComponentHealth {
            status: if self.cdp_server.is_running() { 
                HealthStatus::Healthy 
            } else { 
                HealthStatus::Unhealthy 
            },
            message: format!("DevTools server on port {}", self.config.port),
            metrics: self.get_metrics(),
        }
    }
}
```

## Testing Strategy

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_cdp_server_startup() {
        let devtools = DevToolsComponent::new(Default::default()).unwrap();
        assert!(devtools.start_server(9222).await.is_ok());
        
        // Verify WebSocket endpoint is accessible
        let client = reqwest::Client::new();
        let resp = client.get("http://localhost:9222/json")
            .send().await.unwrap();
        assert_eq!(resp.status(), 200);
    }
    
    #[test]
    fn test_dom_node_mapping() {
        let mut dom_domain = DomDomain::new();
        let node = DomNode::new("div");
        let node_id = dom_domain.register_node(node.clone());
        
        assert!(dom_domain.node_map.contains_key(&node_id));
        assert_eq!(dom_domain.node_map.get(&node_id).unwrap(), &node);
    }
    
    #[test]
    fn test_breakpoint_management() {
        let mut debugger = JavaScriptDebugger::new();
        let location = Location { 
            script_id: "1".to_string(), 
            line_number: 10 
        };
        
        let result = debugger.set_breakpoint(SetBreakpointParams {
            location: location.clone(),
            condition: None,
        });
        
        assert!(result.is_ok());
        assert_eq!(debugger.breakpoints.len(), 1);
    }
}
```

### Integration Tests
```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_full_debugging_session() {
        // Setup browser with DevTools
        let browser = setup_test_browser().await;
        let devtools = browser.get_devtools();
        
        // Connect CDP client
        let client = CdpClient::connect("ws://localhost:9222").await.unwrap();
        
        // Enable domains
        client.runtime_enable().await.unwrap();
        client.debugger_enable().await.unwrap();
        
        // Set breakpoint
        let bp = client.debugger_set_breakpoint(SetBreakpointParams {
            location: Location {
                script_id: "test.js".to_string(),
                line_number: 5,
            },
            condition: None,
        }).await.unwrap();
        
        // Execute code that hits breakpoint
        browser.navigate_to("http://localhost:8080/test.html").await;
        
        // Verify paused at breakpoint
        let paused_event = client.wait_for_event::<DebuggerPaused>().await.unwrap();
        assert_eq!(paused_event.reason, "breakpoint");
        assert_eq!(paused_event.hit_breakpoints, vec![bp.breakpoint_id]);
    }
    
    #[tokio::test]
    async fn test_network_interception() {
        let browser = setup_test_browser().await;
        let devtools = browser.get_devtools();
        let client = CdpClient::connect("ws://localhost:9222").await.unwrap();
        
        // Enable network domain with interception
        client.network_enable(EnableParams {
            max_total_buffer_size: Some(10_000_000),
            max_resource_buffer_size: Some(5_000_000),
        }).await.unwrap();
        
        client.network_set_request_interception(SetRequestInterceptionParams {
            patterns: vec![InterceptionPattern {
                url_pattern: "*test*",
                resource_type: Some(ResourceType::Document),
                interception_stage: InterceptionStage::Request,
            }],
        }).await.unwrap();
        
        // Navigate and intercept
        browser.navigate_to("http://localhost:8080/test.html").await;
        
        let intercepted = client.wait_for_event::<NetworkRequestIntercepted>().await.unwrap();
        assert!(intercepted.request.url.contains("test"));
        
        // Modify and continue
        client.network_continue_intercepted_request(ContinueInterceptedRequestParams {
            interception_id: intercepted.interception_id,
            headers: Some(hashmap!{
                "X-Test-Header" => "Modified"
            }),
        }).await.unwrap();
    }
}
```

### CDP Compliance Tests
```rust
// tests/cdp_compliance/mod.rs

pub struct CdpComplianceTestSuite {
    test_cases: Vec<CdpTestCase>,
}

impl CdpComplianceTestSuite {
    pub fn load_from_chromium() -> Self {
        // Load CDP test cases from Chromium source
        let test_files = glob("chromium/src/third_party/devtools-frontend/test/e2e/**/*.ts").unwrap();
        
        let mut test_cases = Vec::new();
        for file in test_files {
            test_cases.push(CdpTestCase::from_typescript(&file));
        }
        
        Self { test_cases }
    }
    
    pub async fn run_all(&self) -> TestResults {
        let mut results = TestResults::new();
        
        for test in &self.test_cases {
            let result = self.run_single_test(test).await;
            results.record(test.name.clone(), result);
        }
        
        results
    }
}

#[tokio::test]
async fn cdp_compliance() {
    let suite = CdpComplianceTestSuite::load_from_chromium();
    let results = suite.run_all().await;
    
    // Target: 85% compliance with Chrome DevTools Protocol
    assert!(results.pass_rate() >= 0.85, 
           "CDP compliance: {:.2}% (target: 85%)", 
           results.pass_rate() * 100.0);
}
```

### Performance Benchmarks
```rust
#[bench]
fn bench_dom_traversal(b: &mut Bencher) {
    let devtools = setup_devtools();
    let large_dom = create_large_dom_tree(10000); // 10k nodes
    
    b.iter(|| {
        devtools.dom_domain.get_document();
    });
    
    // Target: < 100ms for 10k node tree
    assert!(b.ns_per_iter() < 100_000_000);
}

#[bench]
fn bench_message_throughput(b: &mut Bencher) {
    let devtools = setup_devtools();
    let messages = generate_cdp_messages(1000);
    
    b.iter(|| {
        for msg in &messages {
            devtools.handle_cdp_message(msg);
        }
    });
    
    // Target: > 10k messages/second
    assert!(b.ns_per_iter() / 1000 < 100_000);
}
```

## Build Configuration

### Cargo.toml
```toml
[package]
name = "browser-devtools"
version = "0.1.0"
edition = "2021"

[dependencies]
# Core browser dependencies
browser-interfaces = { path = "../shared/interfaces" }
browser-messages = { path = "../shared/messages" }
browser-types = { path = "../shared/types" }

# WebSocket server
tokio = { version = "1.35", features = ["full"] }
tokio-tungstenite = "0.21"
tungstenite = "0.21"

# Protocol handling
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_repr = "0.1"

# Async runtime
async-trait = "0.1"
futures = "0.3"

# Utilities
anyhow = "1.0"
thiserror = "1.0"
tracing = "0.1"
uuid = { version = "1.6", features = ["v4", "serde"] }
dashmap = "5.5"
parking_lot = "0.12"

# CDP protocol types
chrome-devtools-protocol = { version = "0.1", optional = true }

[dev-dependencies]
# Testing
reqwest = "0.11"
mockito = "1.2"
proptest = "1.4"
criterion = "0.5"

# CDP compliance testing
cdp-client = "0.5"
chromium-cdp-tests = { git = "https://github.com/chromium/chromium", optional = true }

[features]
default = ["full-cdp"]
full-cdp = ["chrome-devtools-protocol"]
minimal = []  # Minimal CDP subset for testing
standalone = []  # For testing without full browser

[build-dependencies]
# Generate CDP protocol bindings
cdp-generator = "0.1"

[[bench]]
name = "devtools_benchmarks"
harness = false
```

## Implementation Phases

### Phase 1: Core Infrastructure (Week 1-2)
**Goal**: Basic CDP server with DOM/CSS inspection

#### Tasks:
1. Implement WebSocket server
2. Create CDP message handling
3. Implement DOM domain basics
4. Implement CSS domain basics
5. Create simple Elements inspector

#### Validation:
- Connect Chrome DevTools frontend to server
- Successfully inspect DOM elements
- View computed styles
- Pass 50% of basic CDP tests

### Phase 2: Network and Console (Week 3-4)
**Goal**: Network monitoring and console functionality

#### Tasks:
1. Implement Network domain
2. Create request/response tracking
3. Implement Console domain
4. Add logging infrastructure
5. Create REPL functionality

#### Validation:
- Monitor all network requests
- View request/response details
- Execute console commands
- Log filtering and search

### Phase 3: JavaScript Debugging (Week 5-6)
**Goal**: Full JavaScript debugging capabilities

#### Tasks:
1. Implement Debugger domain
2. Create breakpoint management
3. Implement stepping controls
4. Add scope inspection
5. Support source maps

#### Validation:
- Set and hit breakpoints
- Step through code
- Inspect variables
- Evaluate expressions in scope

### Phase 4: Performance Tools (Week 7-8)
**Goal**: Profiling and performance analysis

#### Tasks:
1. Implement Profiler domain
2. Create CPU profiling
3. Implement HeapProfiler domain
4. Add memory snapshots
5. Create timeline recording

#### Validation:
- Record CPU profiles
- Analyze heap snapshots
- Timeline with all events
- Performance metrics accurate

### Phase 5: Advanced Features (Week 9-10)
**Goal**: Complete CDP implementation

#### Tasks:
1. Implement Emulation domain
2. Add Storage domain
3. Implement Security domain
4. Create extension debugging support
5. Add remote debugging

#### Validation:
- Device emulation working
- Storage inspection complete
- Security panel functional
- 85% CDP compliance achieved

## Success Metrics

### Functional Metrics
- **CDP Compliance**: ≥ 85% of Chrome DevTools Protocol
- **Feature Coverage**: All major DevTools panels functional
- **Compatibility**: Chrome DevTools frontend fully compatible

### Performance Metrics
- **Message Throughput**: > 10,000 messages/second
- **DOM Traversal**: < 100ms for 10,000 nodes
- **Memory Overhead**: < 50MB for DevTools server
- **Latency**: < 10ms message round-trip

### Quality Metrics
- **Test Coverage**: > 80% code coverage
- **CDP Test Pass Rate**: > 85%
- **Zero memory leaks in 24-hour stress test
- **Crash rate**: < 0.01% in production

## External Dependencies

### Required Libraries
```toml
[dependencies]
# Protocol and serialization
serde = "1.0"
serde_json = "1.0"
bincode = "1.3"  # For efficient binary serialization

# WebSocket
tokio-tungstenite = "0.21"
tungstenite = "0.21"

# Async runtime
tokio = { version = "1.35", features = ["full"] }
async-trait = "0.1"

# Data structures
dashmap = "5.5"  # Concurrent HashMap
parking_lot = "0.12"  # Better Mutex/RwLock
```

### Optional Dependencies
```toml
# For Chrome DevTools frontend hosting
tower-http = { version = "0.5", features = ["fs", "cors"] }
axum = "0.7"  # HTTP server for DevTools frontend

# For source map support
sourcemap = "7.0"

# For protocol generation from Chrome
cdp-definition-parser = "0.1"
```

## Component Communication Examples

### Receiving DOM Updates
```rust
// Listening for DOM changes from DOM component
ComponentMessage::Dom(DomMessage::NodeInserted { 
    parent_id, 
    previous_id, 
    node 
}) => {
    // Convert to CDP event
    let event = DomEvent::ChildNodeInserted {
        parent_node_id: self.get_cdp_node_id(parent_id),
        previous_node_id: previous_id.map(|id| self.get_cdp_node_id(id)),
        node: self.build_cdp_node(node),
    };
    
    self.broadcast_to_clients(CdpEvent::Dom(event));
}
```

### Requesting Network Interception
```rust
// Setting up network interception with network component
self.send_component_message(
    ComponentMessage::Network(NetworkMessage::EnableInterception {
        patterns: vec![
            InterceptionPattern {
                url_pattern: "*",
                resource_type: Some(ResourceType::Xhr),
                stage: InterceptionStage::Request,
            }
        ],
        callback: Box::new(move |request| {
            // Handle intercepted request
            self.handle_intercepted_request(request)
        }),
    })
)?;
```

## Security Considerations

### WebSocket Security
```rust
impl CdpWebSocketServer {
    fn validate_origin(&self, origin: &str) -> bool {
        // Only allow connections from trusted origins
        self.config.allowed_origins.iter().any(|allowed| {
            if allowed.ends_with('*') {
                origin.starts_with(&allowed[..allowed.len()-1])
            } else {
                origin == allowed
            }
        })
    }
    
    fn validate_request(&self, req: &Request<()>) -> Result<(), Error> {
        // Check Origin header
        if let Some(origin) = req.headers().get("Origin") {
            let origin_str = origin.to_str()?;
            if !self.validate_origin(origin_str) {
                return Err(Error::UnauthorizedOrigin);
            }
        }
        
        // Validate upgrade request
        if req.headers().get("Upgrade") != Some(&HeaderValue::from_static("websocket")) {
            return Err(Error::InvalidUpgrade);
        }
        
        Ok(())
    }
}
```

### Message Validation
```rust
fn validate_cdp_message(message: &str) -> Result<CdpMessage, Error> {
    // Size check
    if message.len() > MAX_MESSAGE_SIZE {
        return Err(Error::MessageTooLarge);
    }
    
    // Parse and validate structure
    let msg: CdpMessage = serde_json::from_str(message)?;
    
    // Validate method is allowed
    if DANGEROUS_METHODS.contains(&msg.method.as_str()) {
        return Err(Error::MethodNotAllowed);
    }
    
    Ok(msg)
}
```

## Error Handling

### CDP Error Responses
```rust
#[derive(Debug, Serialize)]
pub struct CdpError {
    code: i32,
    message: String,
    data: Option<serde_json::Value>,
}

impl CdpError {
    pub fn method_not_found(method: &str) -> Self {
        Self {
            code: -32601,
            message: format!("Method not found: {}", method),
            data: None,
        }
    }
    
    pub fn invalid_params(details: &str) -> Self {
        Self {
            code: -32602,
            message: "Invalid parameters".to_string(),
            data: Some(json!({ "details": details })),
        }
    }
    
    pub fn internal_error(error: &dyn std::error::Error) -> Self {
        Self {
            code: -32603,
            message: "Internal error".to_string(),
            data: Some(json!({ "error": error.to_string() })),
        }
    }
}
```

## Performance Optimizations

### Message Batching
```rust
struct MessageBatcher {
    pending: Vec<CdpEvent>,
    last_flush: Instant,
    max_batch_size: usize,
    max_latency: Duration,
}

impl MessageBatcher {
    async fn add_event(&mut self, event: CdpEvent) {
        self.pending.push(event);
        
        if self.should_flush() {
            self.flush().await;
        }
    }
    
    fn should_flush(&self) -> bool {
        self.pending.len() >= self.max_batch_size ||
        self.last_flush.elapsed() >= self.max_latency
    }
    
    async fn flush(&mut self) {
        if self.pending.is_empty() {
            return;
        }
        
        let batch = std::mem::take(&mut self.pending);
        self.send_batch(batch).await;
        self.last_flush = Instant::now();
    }
}
```

### Object Caching
```rust
struct RemoteObjectCache {
    objects: DashMap<RemoteObjectId, CachedObject>,
    max_cache_size: usize,
    ttl: Duration,
}

impl RemoteObjectCache {
    fn get_or_create(&self, value: JsValue) -> RemoteObject {
        // Check if object already cached
        if let Some(cached) = self.find_cached(&value) {
            cached.touch(); // Update last accessed
            return cached.remote_object();
        }
        
        // Create new remote object
        let remote_object = self.create_remote_object(value);
        self.cache_object(remote_object.clone());
        
        // Evict old objects if needed
        if self.objects.len() > self.max_cache_size {
            self.evict_oldest();
        }
        
        remote_object
    }
}
```

## Claude Code Implementation Guidelines

### Context Window Management
```rust
// Split large modules to fit in context window
// Each file should be < 2000 lines

// devtools/src/protocol/domains/mod.rs
pub mod browser;    // ~1500 lines
pub mod debugger;   // ~2000 lines
pub mod dom;        // ~1800 lines
pub mod css;        // ~1500 lines
pub mod network;    // ~2000 lines
pub mod runtime;    // ~1800 lines
pub mod page;       // ~1200 lines
// ... more domains
```

### Progressive Implementation
```rust
// Start with minimal viable DevTools
#[cfg(feature = "minimal")]
mod minimal {
    // Only DOM and CSS inspection
    pub use super::protocol::domains::{dom, css};
}

#[cfg(feature = "full")]
mod full {
    // All domains
    pub use super::protocol::domains::*;
}
```

### Testing in Isolation
```rust
// Standalone test mode without full browser
#[cfg(feature = "standalone")]
mod standalone {
    pub struct MockBrowser {
        dom: MockDom,
        css: MockCss,
        network: MockNetwork,
    }
    
    impl MockBrowser {
        pub fn setup_for_devtools_testing() -> Self {
            // Create mock components for testing
            Self {
                dom: MockDom::with_test_document(),
                css: MockCss::with_test_styles(),
                network: MockNetwork::with_test_requests(),
            }
        }
    }
}
```

---

This specification provides a complete blueprint for implementing the DevTools component. The modular design allows for incremental development while maintaining compatibility with the Chrome DevTools Protocol standard.
