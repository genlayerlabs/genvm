use std::{collections::BTreeMap, sync::LazyLock};

use anyhow::{anyhow, Result};
use rustpython_vm::{bytecode::CodeObject, frozen};

struct CompiledModule {
    code: CodeObject,
    package: bool,
}

static CARGO_MANIFEST_DIR: LazyLock<std::path::PathBuf> =
    LazyLock::new(|| std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")));

fn compile(
    source: &str,
    mode: rustpython_compiler_core::Mode,
    module_name: String,
) -> Result<rustpython_compiler::CodeObject, Box<dyn std::error::Error>> {
    use rustpython_compiler::{compile, CompileOpts};
    Ok(compile(source, mode, module_name, CompileOpts::default())?)
}

fn compile_string<D: std::fmt::Display, F: FnOnce() -> D>(
    source: &str,
    module_name: String,
    origin: F,
) -> Result<CodeObject> {
    compile(source, rustpython_compiler_core::Mode::Exec, module_name)
        .map_err(|err| anyhow!("Python compile error from {}: {}", origin(), err))
}

fn compile_dir(path: &std::path::Path, parent: String) -> Result<BTreeMap<String, CompiledModule>> {
    let mut code_map = BTreeMap::new();
    let paths = std::fs::read_dir(path)
        .or_else(|e| {
            if cfg!(windows) {
                if let Ok(real_path) = std::fs::read_to_string(path.canonicalize().unwrap()) {
                    return std::fs::read_dir(real_path.trim());
                }
            }
            Err(e)
        })
        .map_err(|err| anyhow!("Error listing dir {path:?}: {err}"))?;
    let mut paths: Vec<std::fs::DirEntry> = paths.flat_map(|x| x.into_iter()).collect();
    paths.sort_by_key(|d| d.file_name());
    for path in paths {
        let path = path.path();
        let file_name = path
            .file_name()
            .unwrap()
            .to_str()
            .ok_or_else(|| anyhow!("Invalid UTF-8 in file name {path:?}"))?;
        if path.is_dir() {
            code_map.extend(compile_dir(
                &path,
                if parent.is_empty() {
                    file_name.to_string()
                } else {
                    format!("{parent}.{file_name}")
                },
            )?);
        } else if file_name.ends_with(".py") {
            let stem = path.file_stem().unwrap().to_str().unwrap();
            let is_init = stem == "__init__";
            let module_name = if is_init {
                parent.clone()
            } else if parent.is_empty() {
                stem.to_owned()
            } else {
                format!("{parent}.{stem}")
            };

            let compile_path = |src_path: &std::path::Path| {
                let source = std::fs::read_to_string(src_path)
                    .map_err(|err| anyhow!("Error reading file {path:?}: {err}"))?;
                compile_string(&source, module_name.clone(), || {
                    path.strip_prefix(&*CARGO_MANIFEST_DIR)
                        .ok()
                        .unwrap_or(&path)
                        .display()
                })
            };
            let code = compile_path(&path).or_else(|e| {
                if cfg!(windows) {
                    if let Ok(real_path) = std::fs::read_to_string(path.canonicalize().unwrap()) {
                        let joined = path.parent().unwrap().join(real_path.trim());
                        if joined.exists() {
                            return compile_path(&joined);
                        } else {
                            return Err(e);
                        }
                    }
                }
                Err(e)
            });

            let code = match code {
                Ok(code) => code,
                Err(_) if stem.starts_with("badsyntax_") | parent.ends_with(".encoded_modules") => {
                    // TODO: handle with macro arg rather than hard-coded path
                    continue;
                }
                Err(e) => return Err(e),
            };

            code_map.insert(
                module_name,
                CompiledModule {
                    code,
                    package: is_init,
                },
            );
        }
    }
    Ok(code_map)
}

fn main() -> Result<()> {
    let path = CARGO_MANIFEST_DIR.join("py");
    let code_map = compile_dir(&path, String::new())?;

    let data = frozen::FrozenLib::encode(code_map.iter().map(|(k, v)| {
        let v = frozen::FrozenModule {
            code: frozen::FrozenCodeObject::encode(&v.code),
            package: v.package,
        };
        (&**k, v)
    }));

    let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target");
    p.push("sdk.frozen");
    std::fs::write(p, &data.bytes.as_slice())?;
    Ok(())
}
