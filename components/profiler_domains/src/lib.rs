//! Performance profiling and heap analysis
//!
//! This module provides ProfilerDomain and HeapProfilerDomain implementations
//! for the Chrome DevTools Protocol (CDP).

mod heap_profiler_domain;
mod profiler_domain;
mod types;

pub use heap_profiler_domain::HeapProfilerDomain;
pub use profiler_domain::ProfilerDomain;
pub use types::*;
