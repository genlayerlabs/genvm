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
}
