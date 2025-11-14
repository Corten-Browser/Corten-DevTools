# devtools_api

**Type**: Application Layer (Level 4)
**Tech Stack**: Rust 2021 Edition
**Version**: 0.1.0

## Overview

`devtools_api` provides the public API for CortenBrowser DevTools. It offers a simple, ergonomic interface for integrating Chrome DevTools Protocol (CDP) support into the browser.

This component is a thin wrapper around `devtools_component`, providing a clean public API that hides implementation details and makes it easy to use DevTools in your application.

## Features

- ✅ Simple, ergonomic public API
- ✅ Full Chrome DevTools Protocol (CDP) support
- ✅ WebSocket server for debugging
- ✅ Configurable ports and settings
- ✅ Multiple debugging target support
- ✅ Lifecycle management (start/stop/restart)

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
devtools_api = { path = "components/devtools_api" }
tokio = { version = "1.35", features = ["full"] }
anyhow = "1.0"
```

## Quick Start

### Basic Usage

```rust
use devtools_api::{DevTools, DevToolsConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create DevTools with default configuration
    let config = DevToolsConfig::default();
    let devtools = DevTools::new(config)?;

    // Start the DevTools server on port 9222
    devtools.start(9222).await?;

    println!("DevTools URL: {}", devtools.get_url());
    println!("Connect Chrome DevTools to this URL");

    // ... browser runs ...

    // Stop the DevTools server
    devtools.stop().await?;

    Ok(())
}
```

### Custom Configuration

```rust
use devtools_api::{DevTools, DevToolsConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Build custom configuration
    let config = DevToolsConfig::builder()
        .port(9222)
        .enable_remote_debugging(true)
        .allowed_origin("http://localhost:3000".to_string())
        .max_message_size(100 * 1024 * 1024) // 100MB
        .build();

    let devtools = DevTools::new(config)?;
    devtools.start(9222).await?;

    println!("DevTools ready at: {}", devtools.get_url());

    devtools.stop().await?;
    Ok(())
}
```

### Ephemeral Port (Testing)

```rust
use devtools_api::{DevTools, DevToolsConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = DevToolsConfig::default();
    let devtools = DevTools::new(config)?;

    // Use port 0 to get an ephemeral port
    devtools.start(0).await?;

    // The actual port is reflected in the URLs
    println!("DevTools URL: {}", devtools.get_url());

    devtools.stop().await?;
    Ok(())
}
```

### Multiple Debugging Targets

```rust
use devtools_api::{DevTools, DevToolsConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = DevToolsConfig::default();
    let devtools = DevTools::new(config)?;

    devtools.start(9222).await?;

    // Get debugger URLs for different targets
    let page_url = devtools.get_debugger_url("page-1");
    let worker_url = devtools.get_debugger_url("worker-1");

    println!("Page debugger: {}", page_url);
    println!("Worker debugger: {}", worker_url);

    devtools.stop().await?;
    Ok(())
}
```

## API Reference

### `DevTools`

Main public API for DevTools.

#### Methods

##### `new(config: DevToolsConfig) -> Result<Self>`

Create a new DevTools instance with the given configuration.

**Example:**
```rust
let config = DevToolsConfig::default();
let devtools = DevTools::new(config)?;
```

##### `async fn start(&self, port: u16) -> Result<()`

Start the DevTools server on the specified port.

**Parameters:**
- `port`: Port number to bind to (use 0 for ephemeral port)

**Example:**
```rust
devtools.start(9222).await?;
```

##### `async fn stop(&self) -> Result<()`

Stop the DevTools server.

**Example:**
```rust
devtools.stop().await?;
```

##### `fn get_url(&self) -> String`

Get the DevTools HTTP endpoint URL.

**Returns:** URL string (e.g., `http://localhost:9222/json`)

**Example:**
```rust
let url = devtools.get_url();
println!("DevTools at: {}", url);
```

##### `fn get_debugger_url(&self, target_id: &str) -> String`

Get the WebSocket debugger URL for a specific target.

**Parameters:**
- `target_id`: ID of the debugging target

**Returns:** WebSocket URL string

**Example:**
```rust
let url = devtools.get_debugger_url("page-123");
// Returns: ws://localhost:9222/devtools/page/page-123
```

### `DevToolsConfig`

Configuration for DevTools server.

#### Builder Pattern

```rust
let config = DevToolsConfig::builder()
    .port(9222)
    .enable_remote_debugging(true)
    .allowed_origin("http://localhost:3000".to_string())
    .max_message_size(100 * 1024 * 1024)
    .build();
```

#### Default Configuration

```rust
let config = DevToolsConfig::default();
// port: 9222
// enable_remote_debugging: false
// allowed_origins: ["http://localhost:*"]
// max_message_size: 100MB
// protocol_version: "1.3"
```

## Error Handling

All async operations return `Result<T, DevToolsError>`:

```rust
use devtools_api::{DevTools, DevToolsConfig, DevToolsError};

match devtools.start(9222).await {
    Ok(()) => println!("DevTools started successfully"),
    Err(DevToolsError::ServerAlreadyRunning) => {
        println!("Server is already running");
    }
    Err(e) => println!("Error starting DevTools: {}", e),
}
```

### Common Errors

- `ServerAlreadyRunning` - Attempted to start an already running server
- `ServerNotRunning` - Attempted to stop a server that isn't running
- `ServerStartFailed` - Failed to start the server
- `ServerStopFailed` - Failed to stop the server
- `InvalidConfiguration` - Invalid configuration provided

## Testing

Run the test suite:

```bash
cargo test
```

Run with coverage:

```bash
cargo tarpaulin --out Html
```

## Architecture

```
devtools_api (Level 4 - Application)
    ├── DevTools (public API)
    └── Re-exports
        ├── DevToolsConfig
        ├── DevToolsError
        └── Result

Dependencies:
    └── devtools_component (Level 3 - Integration)
        ├── DevToolsComponent
        └── Full CDP implementation
```

## Development

See [CLAUDE.md](CLAUDE.md) for component-specific development instructions.

### Quality Standards

- ✅ Test Coverage: ≥80%
- ✅ Test Pass Rate: 100%
- ✅ TDD Compliance: Red-Green-Refactor pattern
- ✅ Linting: Zero warnings (cargo clippy)
- ✅ Formatting: 100% compliant (cargo fmt)
- ✅ Documentation: All public APIs documented

### Current Status

- ✅ **Implemented**: Full public API
- ✅ **Tests**: 26 tests (13 unit + 7 integration + 6 doc)
- ✅ **Quality**: All quality gates passing
- ✅ **Documentation**: Complete API documentation

## Examples

See the `examples/` directory for more usage examples (coming soon).

## License

Part of CortenBrowser DevTools implementation.

## Contributing

This component follows TDD practices:
1. Write tests first (RED)
2. Implement to make tests pass (GREEN)
3. Refactor while keeping tests green (REFACTOR)

All contributions must maintain 100% test pass rate and ≥80% coverage.
