use rustpython::vm::pymodule;

pub fn make_gensdk_module(vm: &::rustpython_vm::VirtualMachine) -> rustpython_vm::PyRef<rustpython_vm::builtins::PyModule> {
    genlayer_sdk::make_module(vm)
}

#[pymodule]
pub mod genlayer_sdk {
    use rustpython::vm::{builtins::PyStrRef, PyResult, VirtualMachine};
    use rustpython_vm::builtins::{PyBytes, PyBytesRef};

    fn map_error<T>(vm: &VirtualMachine, res: Result<T, genvm_sdk_rust::Errno>) -> PyResult<T> {
        res.map_err(
            |e|
                vm.new_errno_error(e.raw() as i32, "sdk error".into())
        )
    }

    #[pyfunction]
    fn rollback(s: PyStrRef) -> PyResult<()> {
        let s = s.as_str();
        unsafe { genvm_sdk_rust::rollback(s.as_ref()) };
        Ok(())
    }

    #[pyfunction]
    fn contract_return(s: PyStrRef) -> PyResult<()> {
        unsafe { genvm_sdk_rust::contract_return(s.as_ref()) };
        Ok(())
    }

    #[pyfunction]
    fn run_nondet(eq_principle: PyStrRef, calldata: PyBytesRef, vm: &VirtualMachine) -> PyResult<String> {
        let eq_principle = eq_principle.as_str();
        let calldata: &[u8] = calldata.as_bytes();
        let len = map_error(vm, unsafe {
            genvm_sdk_rust::run_nondet(
                eq_principle.as_ref(),
                genvm_sdk_rust::Bytes {
                    buf: calldata.as_ptr(),
                    buf_len: calldata.len() as u32,
                }
            )
        })?;
        read_result_str(vm, len)
    }

    #[pyfunction]
    fn call_contract(address: PyBytesRef, calldata: PyStrRef, vm: &VirtualMachine) -> PyResult<String> {
        let len = map_error(vm, unsafe {
            genvm_sdk_rust::call_contract(
                genvm_sdk_rust::Bytes {
                    buf: address.as_ptr(),
                    buf_len: address.len() as u32,
                },
                calldata.as_str(),
            )
        })?;
        read_result_str(vm, len)
    }

    fn read_result_str(vm: &VirtualMachine, len: u32) -> PyResult<String> {
        let mut ret = Vec::<u8>::new();
        ret.resize(len as usize, 0);
        map_error(vm, unsafe { genvm_sdk_rust::read_result(ret.as_mut_ptr(), len) })?;
        String::from_utf8(ret).map_err(|_e| {
            vm.new_buffer_error(String::from("invalid utf8 seq"))
        })
    }

    fn read_result_bytes(vm: &VirtualMachine, len: u32) -> PyResult<PyBytes> {
        let mut ret = Vec::<u8>::new();
        ret.resize(len as usize, 0);
        map_error(vm, unsafe { genvm_sdk_rust::read_result(ret.as_mut_ptr(), len) })?;
        Ok(PyBytes::from(ret))
    }

    #[pyfunction]
    fn get_message_data(vm: &VirtualMachine) -> PyResult<String> {
        let len = map_error(vm, unsafe { genvm_sdk_rust::get_message_data() })?;

        read_result_str(vm, len)
    }

    #[pyfunction]
    fn get_entrypoint(vm: &VirtualMachine) -> PyResult<PyBytes> {
        let len = map_error(vm, unsafe { genvm_sdk_rust::get_entrypoint() })?;

        read_result_bytes(vm, len)
    }
}
