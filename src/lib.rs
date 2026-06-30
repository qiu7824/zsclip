//! Public reusable surfaces for ZSClip.
//!
//! The binary still owns the current Windows-first application runtime.  The
//! `zsui` module is the new framework-shaped API that other Rust application
//! code can reference without importing the ZSClip product modules.

pub mod zsui;
