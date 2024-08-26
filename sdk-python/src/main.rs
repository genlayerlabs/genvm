use rustpython_vm::frozen;

#[cfg(not(feature = "sdk-debug"))]
pub const FROZEN_SDK: &frozen::FrozenLib =
    rustpython_derive::py_freeze!(dir = "./py", crate_name = "rustpython_compiler_core");

#[cfg(not(feature = "sdk-debug"))]
pub const FROZEN_SDK_REWRAP: frozen::FrozenLib<&[u8]> = frozen::FrozenLib {
    bytes: &FROZEN_SDK.bytes,
};

#[cfg(feature = "sdk-debug")]
static FROZEN_SDK_DEBUG_BYTES: std::sync::LazyLock<Vec<u8>> = std::sync::LazyLock::new(|| {
    let p = std::path::PathBuf::from("/sdk.frozen");
    std::fs::read(p).unwrap()
});

#[cfg(feature = "sdk-debug")]
static FROZEN_SDK_DEBUG: std::sync::LazyLock<frozen::FrozenLib<&[u8]>> = std::sync::LazyLock::new(|| {
    frozen::FrozenLib {
        bytes: FROZEN_SDK_DEBUG_BYTES.as_slice(),
    }
});

#[cfg(feature = "sdk-debug")]
pub fn add_frozen_sdk(vm: &mut rustpython_vm::VirtualMachine) {
    vm.add_frozen(&*FROZEN_SDK_DEBUG);
}

#[cfg(not(feature = "sdk-debug"))]
pub fn add_frozen_sdk(vm: &mut rustpython_vm::VirtualMachine) {
    vm.add_frozen(FROZEN_SDK);
}

pub const FROZEN_LIBS: &frozen::FrozenLib =
    rustpython_derive::py_freeze!(dir = "./libs", crate_name = "rustpython_compiler_core");

pub fn main() -> std::process::ExitCode {
    rustpython::run(|vm| {
        vm.add_native_module("genlayer.wasi",  Box::new(genvm_python::make_gensdk_module));
        vm.add_frozen(FROZEN_LIBS);
        add_frozen_sdk(vm);
    })
}
