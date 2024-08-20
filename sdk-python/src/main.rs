pub fn main() -> std::process::ExitCode {
    rustpython::run(|vm| {
        vm.add_native_module("genlayer.wasi",  Box::new(genvm_python::make_gensdk_module))
    })
}
