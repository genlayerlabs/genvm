use pyo3::{
    buffer::PyBuffer,
    exceptions::{PySystemError, PyValueError},
    prelude::*,
    types::PyBytes,
};

use std::{io::Read, os::fd::FromRawFd};

fn get_addr(x: &[u8]) -> PyResult<genvm_sdk_rust::Addr> {
    if x.len() != 20 {
        return Err(PyValueError::new_err("invalid address size"));
    }
    Ok(genvm_sdk_rust::Addr { ptr: x.as_ptr() })
}

fn get_full_addr(x: &[u8]) -> PyResult<genvm_sdk_rust::FullAddr> {
    if x.len() != 32 {
        return Err(PyValueError::new_err("invalid full address size"));
    }
    Ok(genvm_sdk_rust::FullAddr { ptr: x.as_ptr() })
}

fn map_error<T>(res: Result<T, genvm_sdk_rust::Errno>) -> PyResult<T> {
    res.map_err(|e| PySystemError::new_err((e.raw() as i32, e.name())))
}

fn flush_everything() {}

#[pymodule]
#[pyo3(name = "_genlayer_wasi")]
#[allow(clippy::useless_conversion)]
fn genlayer_wasi(m: &Bound<'_, PyModule>) -> PyResult<()> {
    #[pyfn(m)]
    fn storage_read<'a>(
        py: Python<'a>,
        addr: &[u8],
        off: u32,
        len: u32,
    ) -> PyResult<Bound<'a, PyBytes>> {
        let addr = get_full_addr(addr)?;
        PyBytes::new_bound_with(py, len as usize, |byts| unsafe {
            map_error(genvm_sdk_rust::storage_read(
                addr,
                off,
                genvm_sdk_rust::MutBytes {
                    buf: byts.as_mut_ptr(),
                    buf_len: len,
                },
            ))
        })
    }

    #[pyfn(m)]
    fn storage_write(py: Python<'_>, addr: &[u8], off: u32, buf: PyBuffer<u8>) -> PyResult<()> {
        let addr = get_full_addr(addr)?;
        let buf = buf.as_slice(py).unwrap();
        let res = unsafe {
            genvm_sdk_rust::storage_write(
                addr,
                off,
                genvm_sdk_rust::Bytes {
                    buf: buf.as_ptr() as *const u8,
                    buf_len: buf.len() as u32,
                },
            )
        };
        map_error(res)
    }

    #[pyfn(m)]
    fn get_balance(addr: &[u8]) -> PyResult<num_bigint::BigUint> {
        let addr = get_addr(addr)?;
        let mut result: [u8; 32] = [0; 32];
        let res = unsafe { genvm_sdk_rust::get_balance(addr, (&mut result) as *mut u8) };
        map_error(res)?;
        Ok(num_bigint::BigUint::from_bytes_le(&result))
    }

    #[pyfn(m)]
    fn get_self_balance() -> PyResult<num_bigint::BigUint> {
        let mut result: [u8; 32] = [0; 32];
        let res = unsafe { genvm_sdk_rust::get_self_balance((&mut result) as *mut u8) };
        map_error(res)?;
        Ok(num_bigint::BigUint::from_bytes_le(&result))
    }

    #[pyfn(m)]
    fn gl_call(data: &[u8]) -> PyResult<u32> {
        let res = unsafe {
            genvm_sdk_rust::gl_call(genvm_sdk_rust::Bytes {
                buf: data.as_ptr(),
                buf_len: data.len() as u32,
            })
        };

        map_error(res)
    }

    Ok(())
}
