use rustpython::vm::pymodule;
use rustpython_vm::{builtins::PyBytes, PyResult, VirtualMachine};

pub fn make_gensdk_module(
    vm: &::rustpython_vm::VirtualMachine,
) -> rustpython_vm::PyRef<rustpython_vm::builtins::PyModule> {
    genlayer_sdk::make_module(vm)
}

fn get_addr(x: &PyBytes, vm: &VirtualMachine) -> PyResult<genvm_sdk_rust::Addr> {
    if x.len() != 32 {
        return Err(vm.new_value_error("invalid size".into()));
    }
    Ok(genvm_sdk_rust::Addr { ptr: x.as_ptr() })
}

#[pymodule]
pub mod genlayer_sdk {
    use std::{io::Read, os::fd::FromRawFd};

    use rustpython::vm::{
        builtins::{PyBytes, PyBytesRef, PyStrRef},
        protocol::PyBuffer,
        PyResult, VirtualMachine,
    };

    fn map_error<T>(vm: &VirtualMachine, res: Result<T, genvm_sdk_rust::Errno>) -> PyResult<T> {
        res.map_err(|e| vm.new_errno_error(e.raw() as i32, "sdk error".into()))
    }

    fn flush_everything(vm: &VirtualMachine) {
        let _ = rustpython_vm::stdlib::sys::get_stdout(vm)
            .and_then(|f| vm.call_method(&f, "flush", ()));
        let _ = rustpython_vm::stdlib::sys::get_stderr(vm)
            .and_then(|f| vm.call_method(&f, "flush", ()));
    }

    #[pyfunction]
    fn rollback(s: PyStrRef, vm: &VirtualMachine) -> PyResult<()> {
        flush_everything(vm);
        let s = s.as_str();
        unsafe { genvm_sdk_rust::rollback(s.as_ref()) };
        Ok(())
    }

    #[pyfunction]
    fn contract_return(s: PyBytesRef, vm: &VirtualMachine) -> PyResult<()> {
        flush_everything(vm);
        let s = genvm_sdk_rust::Bytes {
            buf: s.as_ptr(),
            buf_len: s.len() as u32,
        };
        unsafe { genvm_sdk_rust::contract_return(s) };
        Ok(())
    }

    #[pyfunction]
    fn run_nondet(
        eq_principle: PyStrRef,
        calldata: PyBytesRef,
        vm: &VirtualMachine,
    ) -> PyResult<u32> {
        flush_everything(vm);
        let eq_principle = eq_principle.as_str();
        let calldata: &[u8] = calldata.as_bytes();
        map_error(vm, unsafe {
            genvm_sdk_rust::run_nondet(
                eq_principle.as_ref(),
                genvm_sdk_rust::Bytes {
                    buf: calldata.as_ptr(),
                    buf_len: calldata.len() as u32,
                },
            )
        })
    }

    #[pyfunction]
    fn call_contract(
        address: PyBytesRef,
        calldata: PyBytesRef,
        vm: &VirtualMachine,
    ) -> PyResult<u32> {
        flush_everything(vm);
        let address = super::get_addr(&address, vm)?;
        map_error(vm, unsafe {
            genvm_sdk_rust::call_contract(
                address,
                genvm_sdk_rust::Bytes {
                    buf: calldata.as_ptr(),
                    buf_len: calldata.len() as u32,
                },
            )
        })
    }

    #[pyfunction]
    fn get_message_data(vm: &VirtualMachine) -> PyResult<String> {
        let res = map_error(vm, unsafe { genvm_sdk_rust::get_message_data() })?;
        let mut file = unsafe { std::fs::File::from_raw_fd(res.file as std::os::fd::RawFd) };
        let mut r = String::with_capacity(res.len as usize);
        map_error(
            vm,
            file.read_to_string(&mut r)
                .map_err(|_| genvm_sdk_rust::ERRNO_IO),
        )?;
        Ok(r)
    }

    #[pyfunction]
    fn get_entrypoint(vm: &VirtualMachine) -> PyResult<PyBytes> {
        let res = map_error(vm, unsafe { genvm_sdk_rust::get_entrypoint() })?;
        let mut file = unsafe { std::fs::File::from_raw_fd(res.file as std::os::fd::RawFd) };
        let mut b = Vec::with_capacity(res.len as usize);
        unsafe {
            b.set_len(res.len as usize);
        }
        map_error(
            vm,
            file.read_exact(&mut b)
                .map_err(|_| genvm_sdk_rust::ERRNO_IO),
        )?;
        Ok(b.into())
    }

    #[pyfunction]
    fn get_webpage(config: PyStrRef, url: PyStrRef, vm: &VirtualMachine) -> PyResult<u32> {
        map_error(vm, unsafe {
            genvm_sdk_rust::get_webpage(config.as_str(), url.as_str())
        })
    }

    #[pyfunction]
    fn call_llm(config: PyStrRef, prompt: PyStrRef, vm: &VirtualMachine) -> PyResult<u32> {
        map_error(vm, unsafe {
            genvm_sdk_rust::call_llm(config.as_str(), prompt.as_str())
        })
    }

    #[pyfunction]
    fn storage_read(
        addr: PyBytesRef,
        off: u32,
        len: u32,
        vm: &VirtualMachine,
    ) -> PyResult<PyBytes> {
        let addr = super::get_addr(&addr, vm)?;
        let mut v = Vec::with_capacity(len as usize);
        let res = unsafe {
            v.set_len(len as usize);
            genvm_sdk_rust::storage_read(
                addr,
                off,
                genvm_sdk_rust::MutBytes {
                    buf: v.as_mut_ptr(),
                    buf_len: len,
                },
            )
        };
        map_error(vm, res)?;
        Ok(PyBytes::from(v))
    }

    #[pyfunction]
    fn storage_write(
        addr: PyBytesRef,
        off: u32,
        buf: PyBuffer,
        vm: &VirtualMachine,
    ) -> PyResult<()> {
        let addr = super::get_addr(&addr, vm)?;
        let buf = buf.as_contiguous().unwrap();
        let res = unsafe {
            genvm_sdk_rust::storage_write(
                addr,
                off,
                genvm_sdk_rust::Bytes {
                    buf: buf.as_ptr(),
                    buf_len: buf.len() as u32,
                },
            )
        };
        map_error(vm, res)
    }
}
