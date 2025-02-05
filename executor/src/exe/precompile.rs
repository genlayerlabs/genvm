use anyhow::{Context, Result};
use clap::builder::OsStr;
use genvm::{caching, ustar::SharedBytes};
use once_cell::sync::Lazy;

#[derive(clap::Args, Debug)]
pub struct Args {
    #[arg(
        long,
        default_value_t = false,
        help = "instead of precompiling show information"
    )]
    info: bool,
}

fn compile_single_file_single_mode(
    result_path: &std::path::Path,
    engine: &wasmtime::Engine,
    wasm_data: &[u8],
    engine_type: &str,
    runner_path: &std::path::Path,
    path_in_runner: &str,
) -> Result<()> {
    let time_start = std::time::Instant::now();
    let precompiled = engine
        .precompile_module(&wasm_data)
        .with_context(|| "precompiling")?;

    log::info!(target: "precompile", event = "compiled wasm", engine = engine_type, runner:? = runner_path, runner_path:? = path_in_runner, duration:? = time_start.elapsed(); "");

    std::fs::create_dir_all(result_path.parent().unwrap())?;

    let sz = precompiled.len();

    std::fs::write(result_path, precompiled)?;

    log::info!(target: "precompile", event = "wrote wasm", "size" = sz, result:? = result_path, engine = engine_type, runner:? = runner_path, runner_path:? = path_in_runner, duration:? = time_start.elapsed(); "");

    Ok(())
}

fn compile_single_file(
    engines: &genvm::vm::Engines,
    runners_dir: &std::path::Path,
    zip_path: &std::path::Path,
) -> Result<()> {
    let base_path = zip_path
        .strip_prefix(&runners_dir)
        .with_context(|| format!("stripping {runners_dir:?} from {runners_dir:?}"))?;

    let base_path = if let Some(no_stem) = base_path.file_stem() {
        base_path.with_file_name(no_stem)
    } else {
        base_path.to_owned()
    };

    let mut result_dir_path = Lazy::force(&caching::PRECOMPILE_DIR)
        .clone()
        .ok_or(anyhow::anyhow!("cache directory is not writable"))?;
    result_dir_path.push(base_path);

    let data = genvm::mmap::load_file(zip_path)?;

    let arch = genvm::ustar::Archive::from_ustar(SharedBytes::new(data))?;

    for (entry_name, contents) in arch
        .data
        .iter()
        .filter(|(k, _v)| k.ends_with(".wasm") || k.ends_with(".so"))
    {
        if !wasmparser::Parser::is_core_wasm(contents.as_ref()) {
            continue;
        }

        let entry_name_hash = caching::path_in_zip_to_hash(&entry_name);
        let result_file = result_dir_path.join(entry_name_hash);

        compile_single_file_single_mode(
            result_file
                .with_extension(caching::DET_NON_DET_PRECOMPILED_SUFFIX.det)
                .as_path(),
            &engines.det,
            contents.as_ref(),
            caching::DET_NON_DET_PRECOMPILED_SUFFIX.det,
            zip_path,
            &entry_name,
        )
        .with_context(|| format!("processing det {entry_name}"))?;

        compile_single_file_single_mode(
            result_file
                .with_extension(caching::DET_NON_DET_PRECOMPILED_SUFFIX.non_det)
                .as_path(),
            &engines.non_det,
            contents.as_ref(),
            caching::DET_NON_DET_PRECOMPILED_SUFFIX.non_det,
            zip_path,
            &entry_name,
        )
        .with_context(|| format!("processing non-det {entry_name}"))?;
    }
    Ok(())
}

pub fn handle(args: Args) -> Result<()> {
    let cache_dir = Lazy::force(&caching::CACHE_DIR);
    let precompile_dir = Lazy::force(&caching::PRECOMPILE_DIR);
    let out = serde_json::json!({
        "cache_dir": cache_dir,
        "precompile_dir": precompile_dir,
        "build_id": env!("GENVM_BUILD_ID"),
    });
    println!("{}", serde_json::to_string(&out)?);
    if args.info {
        return Ok(());
    }
    let engines = genvm::vm::Engines::create(|conf| {
        conf.cranelift_opt_level(wasmtime::OptLevel::Speed);
        Ok(())
    })?;

    let runners_dir = genvm::runner::path()?;

    for runner_id in std::fs::read_dir(&runners_dir)? {
        let runner_id = runner_id?;
        if !runner_id.file_type()?.is_dir() {
            continue;
        }
        for zip_path in std::fs::read_dir(&runner_id.path())? {
            let zip_path = zip_path?;
            if !zip_path.file_type()?.is_file() {
                continue;
            }
            let zip_path = zip_path.path();
            if zip_path.extension() != Some(&OsStr::from("tar")) {
                continue;
            }

            compile_single_file(&engines, &runners_dir, &zip_path)
                .with_context(|| format!("processing {zip_path:?}"))?;
        }
    }

    Ok(())
}
