//! Application Binary Interface for GenLayer contracts.
//!
//! This module provides the types, traits, and functions needed to build
//! GenLayer intelligent contracts in Rust.
//!
//! - [`consts`]: Auto-generated constants (EntryKind, ResultCode, etc.)
//! - [`entry`]: Contract entry point handling and the Contract trait
//! - [`gl_call`]: Message types for gl_call operations
//! - [`wasi`]: WASI bindings for storage, balance, and gl_call

pub mod consts;
pub mod entry;
pub mod gl_call;
#[cfg(feature = "wasi")]
pub mod wasi;
