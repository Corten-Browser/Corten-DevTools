//! Browser, Page, Security, and Emulation domains
//!
//! This module implements CDP domains for browser control, page management,
//! security monitoring, and device emulation.

mod browser;
mod emulation;
mod page;
mod security;

pub use browser::BrowserDomain;
pub use emulation::EmulationDomain;
pub use page::PageDomain;
pub use security::SecurityDomain;
