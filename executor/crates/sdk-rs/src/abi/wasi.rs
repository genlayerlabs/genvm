//! WASI bindings for the GenLayer SDK.
//!
//! This module provides safe Rust wrappers around the raw GenLayer WASI imports.
//! These functions allow contracts to interact with blockchain state, query balances,
//! and invoke gl_call operations.

use core::result::Result;

/// Raw FFI bindings to the GenLayer WASI imports.
///
/// These are unsafe C-style function declarations that map directly to the
/// WebAssembly imports from the `genlayer_sdk` module. Prefer using the safe
/// wrapper functions in the parent module instead.
pub mod raw {
    #[link(wasm_import_module = "genlayer_sdk")]
    unsafe extern "C" {
        pub fn storage_read(slot: *const u8, index: u32, buf: *mut u8, buf_len: u32) -> u32;

        pub fn storage_write(slot: *const u8, index: u32, buf: *const u8, buf_len: u32) -> u32;

        pub fn get_balance(address: *const u8, result: *mut u8) -> u32;

        pub fn get_self_balance(result: *mut u8) -> u32;

        pub fn gl_call(request: *const u8, request_len: u32, result_fd: *mut u32) -> u32;
    }
}

/// Error type returned by GenLayer WASI functions.
///
/// Contains the error code returned by the underlying WASI call.
/// Error codes:
/// - 0: success
/// - 1: overflow
/// - 2: inval (invalid argument)
/// - 3: fault
/// - 4: ilseq (illegal sequence)
/// - 5: io
/// - 6: forbidden (permission denied)
/// - 7: inbalance (insufficient balance)
pub struct WasiError(pub u32);

const ERROR_NAMES: &[&str] = &[
    "success",
    "overflow",
    "inval",
    "fault",
    "ilseq",
    "io",
    "forbidden",
    "inbalance",
];

impl WasiError {
    /// Converts a raw error code to a Result.
    ///
    /// Returns `Ok(())` if code is 0 (success), otherwise returns `Err(WasiError(code))`.
    pub fn from_code(code: u32) -> Result<(), WasiError> {
        if code == 0 {
            Ok(())
        } else {
            Err(WasiError(code))
        }
    }

    /// Returns the human-readable name of this error code.
    pub fn name(&self) -> &'static str {
        ERROR_NAMES
            .get(self.0 as usize)
            .copied()
            .unwrap_or("unknown")
    }
}

impl std::fmt::Display for WasiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.name(), self.0)
    }
}

impl std::fmt::Debug for WasiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WasiError::{} ({})", self.name(), self.0)
    }
}

impl std::error::Error for WasiError {}

/// Reads data from contract storage at the specified slot and index.
///
/// # Arguments
/// * `slot` - 32-byte storage slot identifier
/// * `index` - Byte offset within the slot to start reading
/// * `buf` - Buffer to read data into
///
/// # Requirements
/// * Sub-VM must have read storage permission
/// * `index + buf.len()` must not overflow
#[inline(always)]
pub fn storage_read(slot: &[u8; 32], index: u32, buf: &mut [u8]) -> Result<(), WasiError> {
    let ret =
        unsafe { raw::storage_read(slot.as_ptr(), index, buf.as_mut_ptr(), buf.len() as u32) };

    WasiError::from_code(ret)
}

/// Writes data to contract storage at the specified slot and index.
///
/// # Arguments
/// * `slot` - 32-byte storage slot identifier
/// * `index` - Byte offset within the slot to start writing
/// * `buf` - Data to write
///
/// # Requirements
/// * Sub-VM must have write storage permission
/// * `index + buf.len()` must not overflow
/// * Storage slot must not be locked (unless sender is in upgraders list)
#[inline(always)]
pub fn storage_write(slot: &[u8; 32], index: u32, buf: &[u8]) -> Result<(), WasiError> {
    let ret = unsafe { raw::storage_write(slot.as_ptr(), index, buf.as_ptr(), buf.len() as u32) };

    WasiError::from_code(ret)
}

/// Queries the balance of a specified contract address.
///
/// # Arguments
/// * `address` - 20-byte address to query
///
/// # Returns
/// The balance as a 256-bit unsigned integer (little-endian).
pub fn get_balance(address: &[u8; 20]) -> Result<primitive_types::U256, WasiError> {
    let mut result = [0u8; 32];
    let ret = unsafe { raw::get_balance(address.as_ptr(), result.as_mut_ptr()) };

    WasiError::from_code(ret)?;
    Ok(primitive_types::U256::from_little_endian(&result))
}

/// Gets the current contract's balance, adjusted for the current transaction context.
///
/// The returned value is: `balance_before_transaction + message.value - value_consumed_by_current_tx`
///
/// # Returns
/// The adjusted balance as a 256-bit unsigned integer.
pub fn get_self_balance() -> Result<primitive_types::U256, WasiError> {
    let mut result = [0u8; 32];
    let ret = unsafe { raw::get_self_balance(result.as_mut_ptr()) };

    WasiError::from_code(ret)?;
    Ok(primitive_types::U256::from_little_endian(&result))
}

/// Primary GenLayer WASI SDK function for invoking blockchain operations.
///
/// Takes a calldata-encoded request and dispatches to various operations
/// based on the message type. See [`super::gl_call::Message`] for available operations.
///
/// # Arguments
/// * `request` - Calldata-encoded message specifying the operation
///
/// # Returns
/// A file descriptor that can be read to get the operation result.
pub fn gl_call(request: &[u8]) -> Result<u32, WasiError> {
    let mut result_fd: u32 = 0;
    let ret = unsafe {
        raw::gl_call(
            request.as_ptr(),
            request.len() as u32,
            &mut result_fd as *mut u32,
        )
    };

    WasiError::from_code(ret)?;
    Ok(result_fd)
}
