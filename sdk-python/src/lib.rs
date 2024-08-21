use rustpython::vm::pymodule;

pub fn make_gensdk_module(vm: &::rustpython_vm::VirtualMachine) -> rustpython_vm::PyRef<rustpython_vm::builtins::PyModule> {
    genlayer_sdk::make_module(vm)
}

#[pymodule]
pub mod genlayer_sdk {
    use rustpython::vm::{builtins::{PyBaseException, PyBaseExceptionRef, PyException, PyStrRef}, pymodule, PyRef, PyResult, VirtualMachine};

    fn map_error<T>(vm: &VirtualMachine, res: Result<T, genvm_sdk_rust::Errno>) -> PyResult<T> {
        res.map_err(
            |e|
                vm.new_errno_error(e.raw() as i32, "sdk internal error".into())
        )
    }

    #[pyfunction]
    fn rollback(s: PyStrRef, vm: &VirtualMachine) -> PyResult<()> {
        let s = s.as_str();
        unsafe { genvm_sdk_rust::rollback(s.as_ref()) };
        Ok(())
    }

    fn read_result(vm: &VirtualMachine, len: u32) -> PyResult<String> {
        let mut ret = Vec::<u8>::new();
        ret.resize(len as usize, 0);
        map_error(vm, unsafe { genvm_sdk_rust::read_result(ret.as_mut_ptr(), len) })?;
        map_error(vm, String::from_utf8(ret).map_err(|_e| genvm_sdk_rust::ERRNO_ILSEQ))
    }

    #[pyfunction]
    fn get_message_data(vm: &VirtualMachine) -> PyResult<String> {
        let len = map_error(vm, unsafe { genvm_sdk_rust::get_message_data() })?;

        read_result(vm, len)
    }
}
