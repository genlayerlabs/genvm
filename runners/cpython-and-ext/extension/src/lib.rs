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

fn flush_everything() {
    //let _ = rustpython_vm::stdlib::sys::get_stdout(vm)
    //    .and_then(|f| vm.call_method(&f, "flush", ()));
    //let _ = rustpython_vm::stdlib::sys::get_stderr(vm)
    //    .and_then(|f| vm.call_method(&f, "flush", ()));
}

#[pymodule]
#[pyo3(name = "_genlayer_wasi")]
fn genlayer_wasi(m: &Bound<'_, PyModule>) -> PyResult<()> {
    #[pyfn(m)]
    fn rollback(s: &str) -> PyResult<()> {
        flush_everything();
        unsafe { genvm_sdk_rust::rollback(s.as_ref()) };
        Ok(())
    }

    #[pyfn(m)]
    fn contract_return(s: &[u8]) -> PyResult<()> {
        flush_everything();
        let s = genvm_sdk_rust::Bytes {
            buf: s.as_ptr(),
            buf_len: s.len() as u32,
        };
        unsafe { genvm_sdk_rust::contract_return(s) };
        Ok(())
    }

    #[pyfn(m)]
    fn run_nondet(leader_data: &[u8], validator_data: &[u8]) -> PyResult<u32> {
        flush_everything();
        map_error(unsafe {
            genvm_sdk_rust::run_nondet(
                genvm_sdk_rust::Bytes {
                    buf: leader_data.as_ptr(),
                    buf_len: leader_data.len() as u32,
                },
                genvm_sdk_rust::Bytes {
                    buf: validator_data.as_ptr(),
                    buf_len: validator_data.len() as u32,
                },
            )
        })
    }

    #[pyfn(m)]
    fn sandbox(data: &[u8]) -> PyResult<u32> {
        flush_everything();
        map_error(unsafe {
            genvm_sdk_rust::sandbox(genvm_sdk_rust::Bytes {
                buf: data.as_ptr(),
                buf_len: data.len() as u32,
            })
        })
    }

    #[pyfn(m)]
    fn call_contract(address: &[u8], calldata: &[u8]) -> PyResult<u32> {
        flush_everything();
        let address = get_addr(&address)?;
        map_error(unsafe {
            genvm_sdk_rust::call_contract(
                address,
                genvm_sdk_rust::Bytes {
                    buf: calldata.as_ptr(),
                    buf_len: calldata.len() as u32,
                },
            )
        })
    }

    #[pyfn(m)]
    fn get_message_data() -> PyResult<String> {
        let res = map_error(unsafe { genvm_sdk_rust::get_message_data() })?;
        let mut file = unsafe { std::fs::File::from_raw_fd(res.file as std::os::fd::RawFd) };
        let mut r = String::with_capacity(res.len as usize);
        map_error(
            file.read_to_string(&mut r)
                .map_err(|_| genvm_sdk_rust::ERRNO_IO),
        )?;
        Ok(r)
    }

    #[pyfn(m)]
    fn get_entrypoint<'a>(py: Python<'a>) -> PyResult<Bound<'a, PyBytes>> {
        let res = map_error(unsafe { genvm_sdk_rust::get_entrypoint() })?;
        let mut file = unsafe { std::fs::File::from_raw_fd(res.file as std::os::fd::RawFd) };

        PyBytes::new_bound_with(py, res.len as usize, |byts| {
            map_error(file.read_exact(byts).map_err(|_| genvm_sdk_rust::ERRNO_IO))
        })
    }

    #[pyfn(m)]
    fn get_webpage(config: &str, url: &str) -> PyResult<u32> {
        map_error(unsafe { genvm_sdk_rust::get_webpage(config, url) })
    }

    #[pyfn(m)]
    fn exec_prompt(config: &str, prompt: &str) -> PyResult<u32> {
        map_error(unsafe { genvm_sdk_rust::exec_prompt(config, prompt) })
    }

    #[pyfn(m)]
    fn exec_prompt_id(id: u8, vars: &str) -> PyResult<u32> {
        map_error(unsafe { genvm_sdk_rust::exec_prompt_id(id, vars) })
    }

    #[pyfn(m)]
    fn eq_principle_prompt(id: u8, vars: &str) -> PyResult<bool> {
        map_error(unsafe { genvm_sdk_rust::eq_principle_prompt(id, vars) }).map(|x| x.raw() != 0)
    }

    #[pyfn(m)]
    fn storage_read<'a>(
        py: Python<'a>,
        addr: &[u8],
        off: u32,
        len: u32,
    ) -> PyResult<Bound<'a, PyBytes>> {
        let addr = get_full_addr(&addr)?;
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
        let addr = get_full_addr(&addr)?;
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
    fn post_message(addr: &[u8], calldata: &[u8], data: &str) -> PyResult<()> {
        let addr = get_addr(&addr)?;
        let res = unsafe {
            genvm_sdk_rust::post_message(
                addr,
                genvm_sdk_rust::Bytes {
                    buf: calldata.as_ptr() as *const u8,
                    buf_len: calldata.len() as u32,
                },
                data,
            )
        };
        map_error(res)
    }

    #[pyfn(m)]
    fn deploy_contract(calldata: &[u8], code: &[u8], data: &str) -> PyResult<()> {
        let res = unsafe {
            genvm_sdk_rust::deploy_contract(
                genvm_sdk_rust::Bytes {
                    buf: calldata.as_ptr() as *const u8,
                    buf_len: calldata.len() as u32,
                },
                genvm_sdk_rust::Bytes {
                    buf: code.as_ptr() as *const u8,
                    buf_len: code.len() as u32,
                },
                data,
            )
        };
        map_error(res)
    }

    Ok(())
}
