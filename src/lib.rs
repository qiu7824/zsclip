//! Public reusable surfaces for ZSClip.
//!
//! The binary still owns the current Windows-first application runtime.  The
//! `zsui` is re-exported from the standalone framework crate so other Rust
//! application code can reference it without importing ZSClip product modules.

pub use zsui;
