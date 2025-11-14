# DevTools Component

**Type**: Integration Component (Level 3)
**Version**: 0.1.0
**Project**: CortenBrowser DevTools

## Overview

The DevTools component is the main orchestrator that integrates all Chrome DevTools Protocol (CDP) domain handlers with the WebSocket server to provide a complete DevTools implementation for CortenBrowser.

## Features

### Domain Registration

Automatically registers and manages **13 CDP domain handlers**:

1. **Browser** - Browser-level operations
2. **Page** - Page navigation and lifecycle
3. **Security** - Security monitoring and certificates
4. **Emulation** - Device emulation
5. **DOM** - DOM tree inspection and manipulation
6. **CSS** - CSS styles and computed values
7. **Network** - Network request/response monitoring
8. **Runtime** - JavaScript runtime and evaluation
9. **Debugger** - JavaScript debugging (breakpoints, stepping)
10. **Profiler** - CPU profiling
11. **HeapProfiler** - Memory profiling
12. **Console** - Console messages and REPL
13. **Storage** - Cookies, localStorage, etc.

### Server Lifecycle Management

- **Start/Stop**: Full lifecycle control over the WebSocket server
- **Ephemeral Ports**: Support for dynamic port allocation (port 0)
- **Concurrent Safety**: Thread-safe operations for concurrent access
- **Graceful Shutdown**: Proper cleanup on server stop

### Configuration

Flexible configuration via builder pattern:
- Port selection (including ephemeral ports)
- Remote debugging enable/disable
- CORS origin allowlist
- Maximum message size
- Protocol version

## Usage

### Basic Example

```rust
use devtools_component::{DevToolsComponent, DevToolsConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create configuration
    let config = DevToolsConfig::builder()
        .port(9222)
        .enable_remote_debugging(false)
        .build();

    // Create DevTools component
    let devtools = DevToolsComponent::new(config)?;

    // Start the server
    devtools.start().await?;

    println!("DevTools server running!");
    println!("WebSocket URL: {}", devtools.get_debugger_url("page-1"));
    println!("JSON endpoint: {}", devtools.get_json_url());

    // Server runs until stopped
    // ...

    // Graceful shutdown
    devtools.stop().await?;

    Ok(())
}
```

### Advanced Configuration

```rust
use devtools_component::DevToolsConfig;

let config = DevToolsConfig::builder()
    .port(0) // Use ephemeral port
    .enable_remote_debugging(true) // Allow remote connections
    .allowed_origin("https://devtools.example.com".to_string())
    .max_message_size(50 * 1024 * 1024) // 50 MB
    .protocol_version("1.3".to_string())
    .build();
```

## Testing

### Test Coverage

- **27 unit tests** - Testing individual components and methods
- **9 integration tests** - End-to-end functionality tests
- **9 doc tests** - Examples in documentation
- **Total: 45 tests** - 100% passing

### Running Tests

```bash
# Run all tests
cargo test

# Run only unit tests
cargo test --lib

# Run only integration tests
cargo test --test integration_tests
```

## Quality Standards

✅ **Test Coverage**: 100% passing (45/45 tests)
✅ **Linting**: Zero clippy warnings
✅ **Formatting**: 100% compliant with rustfmt
✅ **Documentation**: All public APIs documented
✅ **Error Handling**: Proper Result types, no unwrap() in production
✅ **TDD**: Red-Green-Refactor cycle followed

## Dependencies

### Local Components

- `cdp_types` - CDP protocol types and messages
- `cdp_server` - WebSocket server implementation
- `protocol_handler` - Message routing and domain registry
- `browser_page_domains` - Browser/Page/Security/Emulation domains
- `dom_domain` - DOM/CSS domains
- `network_domain` - Network monitoring
- `runtime_debugger` - Runtime/Debugger domains
- `profiler_domains` - Profiler/HeapProfiler domains
- `console_storage` - Console/Storage domains

### External Dependencies

- `tokio` - Async runtime
- `async-trait` - Async trait support
- `tracing` - Logging and instrumentation
- `serde` / `serde_json` - Serialization
- `thiserror` / `anyhow` - Error handling
