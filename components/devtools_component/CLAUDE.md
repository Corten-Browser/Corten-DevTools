# Component: devtools_component

## Component Information
- **Name**: devtools_component
- **Type**: integration
- **Level**: 3
- **Version**: 0.1.0
- **Project**: CortenBrowser DevTools (v0.1.0)
- **Project Root**: /home/user/Corten-DevTools
- **Tech Stack**: Rust 2021, BrowserComponent trait, integration

## Responsibility
Main DevTools orchestration and integration

## Dependencies
- cdp_types
- cdp_server
- protocol_handler
- dom_domain
- network_domain
- runtime_debugger
- profiler_domains
- console_storage
- browser_page_domains

## Implementation Requirements

### CRITICAL: TDD/BDD Workflow
1. **RED**: Write failing tests FIRST (before implementation)
2. **GREEN**: Implement code to make tests pass
3. **REFACTOR**: Clean up code while keeping tests green

### What to Implement

Read the specification file: `/home/user/Corten-DevTools/devtools-component-specification.md`

Extract the relevant sections for **devtools_component** and implement:

- All Rust structs, enums, and types
- All public API functions
- All required traits and trait implementations
- Integration with browser components (if applicable)
- Error handling with proper Result types
- Async/await where appropriate (tokio runtime)

### Quality Standards (MANDATORY)

- **Test Coverage**: ≥80% (target 95%)
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

**Dependencies**:
- Import dependencies as specified in tech_stack
- Add to Cargo.toml: `cargo add <dependency>`
- Use exact versions for stability

### File Structure

\`\`\`
devtools_component/
├── src/
│   ├── lib.rs           # Public API exports
│   ├── <modules>.rs     # Implementation modules
│   └── tests.rs         # Integration tests
├── tests/
│   ├── unit/            # Unit tests
│   └── integration/     # Integration tests
├── Cargo.toml
├── CLAUDE.md            # This file
└── README.md
\`\`\`

### Testing Strategy

**Unit Tests**:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_<functionality>() {
        // RED: Write failing test first
        let result = function_under_test();
        assert_eq!(result, expected_value);
    }

    #[tokio::test]
    async fn test_async_<functionality>() {
        // Test async functionality
        let result = async_function().await;
        assert!(result.is_ok());
    }
}
```

**Integration Tests** (tests/integration/):
```rust
#[cfg(test)]
mod integration {
    use devtools_component::*;

    #[tokio::test]
    async fn test_component_integration() {
        // Test component works with dependencies
    }
}
```

### Error Handling

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DevtoolscomponentError {
    #[error("description: {0}")]
    SpecificError(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, DevtoolscomponentError>;
```

### Git Workflow

Commit your work with component prefix:
```bash
# DO NOT run git commands directly - issues with commit prefix
# Just let the orchestrator handle git operations
```

### Quality Verification

Before marking complete, verify:
- [ ] All tests passing (100%)
- [ ] Test coverage ≥80%
- [ ] `cargo clippy` passes (zero warnings)
- [ ] `cargo fmt --check` passes
- [ ] All public APIs documented
- [ ] TDD commits visible in history
- [ ] No `.unwrap()` or `.expect()` in production code
- [ ] Proper error handling with Result types

### Component Boundaries

- Work ONLY in `components/devtools_component/` directory
- DO NOT modify other components
- DO NOT access `components/*/` (other components)
- Use dependencies through public APIs only

### Communication with Orchestrator

When complete, provide a summary:
```markdown
## Completion Summary

### Implemented
- Feature 1: Description
- Feature 2: Description

### Tests
- Unit tests: XX passing
- Integration tests: XX passing
- Coverage: XX%

### Quality Metrics
- Linting: ✅ Zero errors
- Formatting: ✅ Compliant
- Documentation: ✅ Complete

### Notes
- Any challenges faced
- Decisions made
- Areas that may need review
```

---

**IMPORTANT**: This component is part of a larger system. Your implementation will be integrated with other components. Follow the specification exactly and maintain API compatibility.

**START**: Read the specification, write tests (RED), implement code (GREEN), refactor (REFACTOR), verify quality.
