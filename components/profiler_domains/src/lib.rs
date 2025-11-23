//! Performance profiling and heap analysis
//!
//! This module provides ProfilerDomain, HeapProfilerDomain, CpuProfiler,
//! MemoryProfiler, and TimelineDomain implementations for the Chrome DevTools Protocol (CDP).
//!
//! # Features
//!
//! - **ProfilerDomain**: CPU profiling and code coverage (CDP Profiler domain)
//! - **HeapProfilerDomain**: Heap profiling and memory snapshots (CDP HeapProfiler domain)
//! - **CpuProfiler**: Enhanced CPU profiler with sample-based profiling and call tree
//! - **MemoryProfiler**: Memory allocation tracking with leak detection
//! - **TimelineDomain**: Performance timeline recording (FEAT-034)

mod cpu_profiler;
mod heap_profiler_domain;
mod memory_profiler;
mod profiler_domain;
mod timeline_domain;
mod types;

pub use cpu_profiler::{CpuProfiler, ProfileStats};
pub use heap_profiler_domain::HeapProfilerDomain;
pub use memory_profiler::{MemoryProfiler, MemoryStats};
pub use profiler_domain::ProfilerDomain;
pub use timeline_domain::TimelineDomain;
pub use types::*;
