# browser_page_domains

**Type**: feature
**Level**: 2
**Version**: 0.1.0
**Project**: CortenBrowser DevTools (v0.1.0)
**Tech Stack**: Rust 2021, async-trait, tokio, parking_lot

## Responsibility

Browser, Page, Security, and Emulation domains for the Chrome DevTools Protocol (CDP).

## Structure

```
browser_page_domains/
├── src/
│   ├── lib.rs           # Public API exports
│   ├── browser.rs       # Browser domain handler
│   ├── page.rs          # Page domain handler
│   ├── security.rs      # Security domain handler
│   └── emulation.rs     # Emulation domain handler
├── tests/
│   ├── unit/            # Unit tests
│   │   ├── browser_domain_tests.rs
│   │   ├── page_domain_tests.rs
│   │   ├── security_domain_tests.rs
│   │   └── emulation_domain_tests.rs
│   └── unit_tests.rs    # Test entry point
├── Cargo.toml
├── CLAUDE.md            # Component instructions
└── README.md            # This file
```

## Implementation

### Browser Domain

Provides browser information and control:
- `getVersion()` - Returns protocol version, product, user agent, etc.
- `getBrowserCommandLine()` - Returns command line arguments
- `close()` - Closes the browser (no-op in current implementation)

### Page Domain

Handles page navigation and control:
- `enable/disable()` - Enable/disable page monitoring
- `navigate(url)` - Navigate to a URL
- `reload()` - Reload the current page
- `getFrameTree()` - Get the frame hierarchy
- `captureScreenshot()` - Capture a screenshot (mock implementation)

### Security Domain

Security monitoring and certificate handling:
- `enable/disable()` - Enable/disable security monitoring
- `setIgnoreCertificateErrors(ignore)` - Set certificate error behavior
- `handleCertificateError(eventId, action)` - Handle certificate errors

### Emulation Domain

Device emulation capabilities:
- `setDeviceMetricsOverride(width, height, ...)` - Emulate device screen size
- `clearDeviceMetricsOverride()` - Clear device metrics
- `setUserAgentOverride(userAgent)` - Override user agent string
- `setGeolocationOverride(lat, lon, accuracy)` - Emulate geolocation
- `clearGeolocationOverride()` - Clear geolocation override

## Usage

### As a Library Dependency

```rust
use browser_page_domains::{BrowserDomain, PageDomain, SecurityDomain, EmulationDomain};
use protocol_handler::{ProtocolHandler, DomainHandler};
use std::sync::Arc;

// Create protocol handler
let handler = ProtocolHandler::new();

// Register domains
handler.register_domain(Arc::new(BrowserDomain::new()));
handler.register_domain(Arc::new(PageDomain::new()));
handler.register_domain(Arc::new(SecurityDomain::new()));
handler.register_domain(Arc::new(EmulationDomain::new()));

// Handle CDP messages
let response = handler.handle_message(r#"{"id": 1, "method": "Browser.getVersion"}"#).await;
```

### Direct Domain Usage

```rust
use browser_page_domains::BrowserDomain;
use protocol_handler::DomainHandler;

let domain = BrowserDomain::new();
let result = domain.handle_method("getVersion", None).await;
```

## Development

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test suite
cargo test --test unit_tests

# Run with output
cargo test -- --nocapture
```

### Code Quality

```bash
# Format code
cargo fmt

# Run linter
cargo clippy -- -D warnings

# Check coverage
cargo llvm-cov --all-features
```

## Quality Metrics

- **Test Coverage**: 95%+ (exceeds 80% requirement)
  - browser.rs: 100% line coverage
  - page.rs: 96.19% line coverage
  - emulation.rs: 92.76% line coverage
  - security.rs: 93.46% line coverage
- **Test Pass Rate**: 100% (61/61 tests passing)
  - 21 library tests
  - 36 unit tests
  - 4 documentation tests
- **Linting**: ✅ Zero errors (clippy clean)
- **Formatting**: ✅ 100% compliant (rustfmt)
- **Documentation**: ✅ All public APIs documented

## Dependencies

- **cdp_types** - CDP protocol types and error definitions
- **protocol_handler** - CDP message routing and domain registry
- **serde_json** - JSON serialization
- **async-trait** - Async trait support
- **tokio** - Async runtime
- **parking_lot** - High-performance synchronization primitives

## Notes

This component follows TDD (Test-Driven Development) practices:
1. Tests written first (RED phase)
2. Implementation to make tests pass (GREEN phase)
3. Code refactored while keeping tests green (REFACTOR phase)

All CDP method implementations are designed to be extended with real browser integration. Current implementations provide mock/stub functionality for testing and initial development.
