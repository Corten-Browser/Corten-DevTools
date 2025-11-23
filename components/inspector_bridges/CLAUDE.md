# Component: inspector_bridges

## Component Information
- **Name**: inspector_bridges
- **Type**: feature
- **Level**: 2
- **Version**: 0.1.0
- **Project**: CortenBrowser DevTools (v0.1.0)
- **Project Root**: /home/user/Corten-DevTools
- **Tech Stack**: Rust 2021, async-trait, serde, tokio

## Responsibility
DOM and Render Inspector Bridges for Chrome DevTools Protocol integration.

Implements:
- FEAT-017: DOM Inspector Bridge - Bridge between CDP DOM domain and browser DOM
- FEAT-020: Render Inspector Bridge - Bridge for render tree inspection

## Dependencies
- cdp_types
- protocol_handler
- dom_domain

## Features

### DOM Inspector Bridge
- Node tree traversal
- Node selection and highlighting
- DOM mutation tracking
- Node search functionality

### Render Inspector Bridge
- Box model inspection
- Computed styles access
- Layer tree representation

## Implementation Requirements

### CRITICAL: TDD/BDD Workflow
1. **RED**: Write failing tests FIRST (before implementation)
2. **GREEN**: Implement code to make tests pass
3. **REFACTOR**: Clean up code while keeping tests green

### Quality Standards (MANDATORY)

- **Test Coverage**: >= 80% (target 95%)
- **Test Pass Rate**: 100% - ZERO failing tests allowed
- **TDD Compliance**: Git history must show Red-Green-Refactor
- **Linting**: Zero errors (use `cargo clippy`)
- **Formatting**: 100% compliant (use `cargo fmt`)
- **Documentation**: All public APIs must have doc comments
- **Security**: Input validation, no unwrap() in production code

### Technology Guidelines

**Rust Best Practices**:
- Use `Result<T, E>` for error handling
- Use `Option<T>` for optional values
- Avoid `.unwrap()` and `.expect()` - use proper error propagation
- Use `async`/`await` with tokio for concurrent operations
- Use `Arc<T>` for shared ownership across threads
- Use `RwLock<T>` or `Mutex<T>` for thread-safe mutable state
- Implement `Debug`, `Clone` where appropriate

### File Structure

```
inspector_bridges/
├── src/
│   ├── lib.rs                    # Public API exports
│   ├── dom_inspector_bridge.rs   # DOM Inspector Bridge implementation
│   ├── render_inspector_bridge.rs # Render Inspector Bridge implementation
│   ├── types.rs                  # Shared types
│   └── mock_browser.rs           # Mock browser for testing
├── tests/
│   ├── unit/                     # Unit tests
│   │   └── mod.rs
│   └── integration/              # Integration tests
│       └── mod.rs
├── Cargo.toml
├── CLAUDE.md                     # This file
└── README.md
```

### Quality Verification

Before marking complete, verify:
- [ ] All tests passing (100%)
- [ ] Test coverage >= 80%
- [ ] `cargo clippy` passes (zero warnings)
- [ ] `cargo fmt --check` passes
- [ ] All public APIs documented
- [ ] TDD commits visible in history
- [ ] No `.unwrap()` or `.expect()` in production code
- [ ] Proper error handling with Result types

### Component Boundaries

- Work ONLY in `components/inspector_bridges/` directory
- DO NOT modify other components
- Use dependencies through public APIs only
