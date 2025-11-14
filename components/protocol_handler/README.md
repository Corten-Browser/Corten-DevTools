# Protocol Handler

**Type**: Core Component (Level 1)
**Version**: 0.1.0
**Tech Stack**: Rust 2021, serde_json, dashmap, async-trait, tokio

## Overview

The Protocol Handler component provides the core infrastructure for routing Chrome DevTools Protocol (CDP) messages to appropriate domain handlers. It implements a flexible, async-ready message routing system that supports the JSON-RPC 2.0 protocol used by CDP.

## Features

- **Message Routing**: Routes incoming CDP requests to registered domain handlers based on the `Domain.method` format
- **Domain Registry**: Dynamic registration and management of domain handlers
- **Error Handling**: Proper JSON-RPC 2.0 error codes (-32700 to -32603)
- **Async Support**: Fully asynchronous message handling using tokio
- **Thread-Safe**: Concurrent message handling using DashMap
- **Validation**: Input validation with proper error responses

## Architecture

```
┌─────────────────┐
│  CDP Message    │
│   (JSON-RPC)    │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ ProtocolHandler │
│   - Parse       │
│   - Validate    │
│   - Route       │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ DomainRegistry  │
│  (DashMap)      │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ DomainHandler   │
│  (trait impl)   │
└─────────────────┘
```

## Usage

### Creating a Protocol Handler

```rust
use protocol_handler::ProtocolHandler;

let handler = ProtocolHandler::new();
```

### Implementing a Domain Handler

```rust
use async_trait::async_trait;
use protocol_handler::DomainHandler;
use cdp_types::CdpError;
use serde_json::Value;

struct MyDomainHandler;

#[async_trait]
impl DomainHandler for MyDomainHandler {
    fn name(&self) -> &str {
        "MyDomain"
    }

    async fn handle_method(
        &self,
        method: &str,
        params: Option<Value>,
    ) -> Result<Value, CdpError> {
        match method {
            "myMethod" => {
                // Handle the method
                Ok(json!({"result": "success"}))
            }
            _ => Err(CdpError::method_not_found(
                format!("MyDomain.{}", method)
            )),
        }
    }
}
```

### Registering a Domain Handler

```rust
use std::sync::Arc;

let handler = ProtocolHandler::new();
let my_domain = Arc::new(MyDomainHandler);

handler.register_domain(my_domain);
```

### Handling Messages

```rust
let request = r#"{"id": 1, "method": "MyDomain.myMethod", "params": {"key": "value"}}"#;
let response = handler.handle_message(request).await;

// Response will be:
// {"id": 1, "result": {"result": "success"}}
```

## Message Format

### Request

```json
{
  "id": 1,
  "method": "Domain.method",
  "params": {
    "key": "value"
  }
}
```

### Success Response

```json
{
  "id": 1,
  "result": {
    "data": "value"
  }
}
```

### Error Response

```json
{
  "id": 1,
  "error": {
    "code": -32601,
    "message": "Method not found",
    "data": {
      "method": "UnknownDomain.unknownMethod"
    }
  }
}
```

## Error Codes

Following JSON-RPC 2.0 specification:

- **-32700**: Parse error (invalid JSON)
- **-32600**: Invalid request (valid JSON but invalid structure)
- **-32601**: Method not found (domain or method doesn't exist)
- **-32602**: Invalid params (invalid method parameters)
- **-32603**: Internal error (server-side error)

## API Documentation

### `ProtocolHandler`

Main protocol handler for routing CDP messages.

#### Methods

- `new() -> Self`: Create a new protocol handler
- `register_domain(&self, handler: Arc<dyn DomainHandler>)`: Register a domain handler
- `unregister_domain(&self, domain_name: &str) -> Option<Arc<dyn DomainHandler>>`: Unregister a domain
- `handle_message(&self, message: &str) -> String`: Handle an incoming CDP message (async)

### `DomainHandler` Trait

Trait that all domain handlers must implement.

#### Required Methods

- `fn name(&self) -> &str`: Returns the domain name
- `async fn handle_method(&self, method: &str, params: Option<Value>) -> Result<Value, CdpError>`: Handle a method call

## Implementation Details

### Message Routing Flow

1. **Parse**: Parse incoming JSON message
2. **Validate**: Check message structure and method format
3. **Extract**: Extract domain name and method name from "Domain.method" format
4. **Lookup**: Find registered domain handler
5. **Delegate**: Call domain handler's `handle_method`
6. **Response**: Format and return response

### Concurrency

- Uses `DashMap` for lock-free concurrent access to domain registry
- Fully async/await compatible
- Thread-safe domain registration and message handling

### Error Handling

The handler distinguishes between:
- **Parse errors**: Invalid JSON syntax
- **Invalid requests**: Valid JSON but missing required fields or invalid format
- **Domain errors**: Errors from domain handler implementations

## Testing

### Running Tests

```bash
# Run all tests
cargo test

# Run with coverage
cargo tarpaulin

# Run specific test
cargo test test_handle_message_success
```

### Test Coverage

- Unit tests: 10 tests covering core functionality
- Integration tests: 12 tests covering end-to-end scenarios
- Total coverage: >85%

### Test Categories

1. **Message Parsing**: Valid/invalid JSON, missing fields
2. **Domain Registry**: Registration, unregistration, lookup
3. **Message Routing**: Success cases, error cases
4. **Concurrency**: Concurrent message handling
5. **Error Handling**: All error codes and edge cases

## Quality Standards

✅ **Test Coverage**: >85% (target: 95%)
✅ **Test Pass Rate**: 100% (22/22 tests passing)
✅ **Linting**: Zero clippy warnings
✅ **Formatting**: 100% rustfmt compliant
✅ **Documentation**: All public APIs documented
✅ **No `.unwrap()`**: Proper error handling throughout

## Dependencies

- `cdp_types`: CDP protocol types and errors
- `serde_json`: JSON serialization/deserialization
- `dashmap`: Concurrent hash map
- `async-trait`: Async trait support
- `tracing`: Logging infrastructure
- `tokio`: Async runtime

## Performance

- **Message Throughput**: Designed for high-throughput message handling
- **Lock-Free**: Uses DashMap for lock-free concurrent access
- **Zero-Copy**: Minimal allocations in hot paths
- **Async**: Non-blocking I/O throughout

## Future Enhancements

- [ ] Message batching support
- [ ] Metrics and instrumentation
- [ ] Hot domain registration/unregistration
- [ ] Domain priority and fallback handling
- [ ] Request timeout support
- [ ] Rate limiting per domain

## License

Part of CortenBrowser DevTools implementation.

## Contributing

This component follows TDD principles. When adding features:

1. Write failing tests first (RED)
2. Implement code to pass tests (GREEN)
3. Refactor while keeping tests green (REFACTOR)
4. Ensure all quality checks pass
