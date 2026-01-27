use core::result::Result;

pub mod raw {
    #[link(wasm_import_module = "genlayer_sdk")]
    unsafe extern "C" {
        pub fn storage_read(slot: *const u8, index: u32, buf: *mut u8, buf_len: u32) -> u32;

        pub fn storage_write(slot: *const u8, index: i32, buf: *const u8, buf_len: u32) -> u32;

        pub fn get_balance(address: *const u8, result: *mut u8) -> u32;

        pub fn get_self_balance(result: *mut u8) -> u32;

        pub fn gl_call(request: *const u8, request_len: u32, result_fd: *mut u32) -> u32;
    }
}

pub struct WasiError(pub u32);

impl WasiError {
    pub fn from_code(code: u32) -> Result<(), WasiError> {
        if code == 0 {
            Ok(())
        } else {
            Err(WasiError(code))
        }
    }
}

pub fn storage_read(slot: &[u8; 32], index: u32, buf: &mut [u8]) -> Result<(), WasiError> {
    let ret =
        unsafe { raw::storage_read(slot.as_ptr(), index, buf.as_mut_ptr(), buf.len() as u32) };

    WasiError::from_code(ret)
}

pub fn storage_write(slot: &[u8; 32], index: i32, buf: &[u8]) -> Result<(), WasiError> {
    let ret = unsafe { raw::storage_write(slot.as_ptr(), index, buf.as_ptr(), buf.len() as u32) };

    WasiError::from_code(ret)
}

pub fn get_balance(address: &[u8; 20]) -> Result<primitive_types::U256, WasiError> {
    let mut result = [0u8; 32];
    let ret = unsafe { raw::get_balance(address.as_ptr(), result.as_mut_ptr()) };

    WasiError::from_code(ret)?;
    Ok(primitive_types::U256::from_little_endian(&result))
}

pub fn get_self_balance() -> Result<primitive_types::U256, WasiError> {
    let mut result = [0u8; 32];
    let ret = unsafe { raw::get_self_balance(result.as_mut_ptr()) };

    WasiError::from_code(ret)?;
    Ok(primitive_types::U256::from_little_endian(&result))
}

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
