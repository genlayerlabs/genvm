use rustpython_vm::frozen;

pub const FROZEN_SDK: &frozen::FrozenLib =
    rustpython_derive::py_freeze!(dir = "./py", crate_name = "rustpython_compiler_core");

pub fn main() -> std::process::ExitCode {
    rustpython::run(|vm| {
        vm.add_native_module("genlayer.wasi",  Box::new(genvm_python::make_gensdk_module));
        vm.add_frozen(FROZEN_SDK);
    })
}
