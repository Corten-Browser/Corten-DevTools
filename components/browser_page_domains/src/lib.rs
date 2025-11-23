//! Browser, Page, Security, and Emulation domains
//!
//! This module implements CDP domains for browser control, page management,
//! security monitoring, and device emulation.
//!
//! # Features
//! - **BrowserDomain**: Browser information and control
//! - **PageDomain**: Page navigation and screenshot capture
//! - **SecurityDomain**: Security state tracking and certificate handling
//! - **EmulationDomain**: Device emulation (viewport, user agent, etc.)

mod browser;
mod emulation;
mod page;
mod security;

pub use browser::BrowserDomain;
pub use emulation::EmulationDomain;
pub use page::PageDomain;
pub use security::{
    CertificateDetails, CertificateError, CertificateErrorAction, CertificateSecurityState,
    InsecureContentStatus, MixedContentType, SafeBrowsingState, SecurityDomain, SecurityState,
    SecurityStateExplanation, VisibleSecurityState,
};
