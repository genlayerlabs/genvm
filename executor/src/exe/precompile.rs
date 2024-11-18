use std::io::{Read, Write};

use anyhow::{Context, Result};
use clap::builder::OsStr;
use genvm::caching;
use itertools::Itertools;
use once_cell::sync::Lazy;
use zip::ZipWriter;

#[derive(clap::Args, Debug)]
pub struct Args {
    #[arg(
        long,
        default_value_t = false,
        help = "instead of precompiling show information"
    )]
    info: bool,
}

fn compile_single_file(
    engines: &genvm::vm::Engines,
    runners_dir: &std::path::Path,
    zip_path: &std::path::Path,
) -> Result<()> {
    let base_path = zip_path
        .strip_prefix(&runners_dir)
        .with_context(|| format!("stripping {runners_dir:?} from {runners_dir:?}"))?;

    let mut zip_all: Option<ZipWriter<std::fs::File>> = None;

    let precompile = |arch: &mut Option<ZipWriter<std::fs::File>>,
                      zip_path: &std::path::PathBuf,
                      engine: &wasmtime::Engine,
                      path: &str,
                      path_suff: &str,
                      wasm_data: &[u8]|
     -> Result<()> {
        match arch {
            None => {
                println!("Started compiling {zip_path:?}");
                std::fs::create_dir_all(zip_path.parent().unwrap())?;
                let file = std::fs::File::create(zip_path.to_path_buf())
                    .with_context(|| format!("lazy creating {:?}", &zip_path))?;
                *arch = Some(zip::ZipWriter::new(file))
            }
            _ => {}
        }
        let arch = match arch {
            Some(arch) => arch,
            None => unreachable!(),
        };
        let precompiled = engine
            .precompile_module(&wasm_data)
            .with_context(|| "precompiling")?;
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::ZSTD);
        let mut path = String::from(path);
        path.push_str(path_suff);
        arch.start_file(&path, options)
            .with_context(|| format!("writing {path}"))?;
        arch.write_all(&precompiled)
            .with_context(|| "writing data")?;
        Ok(())
    };

    let mut result_zip_path = match Lazy::force(&caching::PRECOMPILE_DIR) {
        Some(v) => v,
        None => anyhow::bail!("cache directory is not writable"),
    }
    .clone();
    result_zip_path.push(base_path);

    let mut run = || -> Result<()> {
        let file =
            std::fs::File::open(&zip_path).with_context(|| format!("reading {:?}", &zip_path))?;
        let mut zip = zip::ZipArchive::new(file)?;
        let mut wasm_data = Vec::new();
        for entry_name in zip
            .file_names()
            .filter(|f| f.ends_with(".wasm") || f.ends_with("so"))
            .map(String::from)
            .collect_vec()
        {
            wasm_data.clear();
            zip.by_name(&entry_name)?.read_to_end(&mut wasm_data)?;
            if !wasmparser::Parser::is_core_wasm(&wasm_data) {
                continue;
            }

            precompile(
                &mut zip_all,
                &result_zip_path,
                &engines.det,
                &entry_name,
                caching::DET_NON_DET_PRECOMPILED_SUFFIX.det,
                &wasm_data,
            )
            .with_context(|| format!("processing det {entry_name}"))?;
            precompile(
                &mut zip_all,
                &result_zip_path,
                &engines.non_det,
                &entry_name,
                caching::DET_NON_DET_PRECOMPILED_SUFFIX.non_det,
                &wasm_data,
            )
            .with_context(|| format!("processing non-det {entry_name}"))?;
        }
        Ok(())
    };

    let mut run_res = run();
    let mut has_data = false;

    let mut close_zip = |zip: Option<ZipWriter<std::fs::File>>| -> Result<()> {
        let zip = match zip {
            None => return Ok(()),
            Some(zip) => zip,
        };
        has_data = true;
        let mut writer = zip.finish()?;
        writer.flush()?;
        Ok(())
    };

    run_res = run_res.and(close_zip(zip_all));

    match run_res {
        Ok(()) => {
            if has_data {
                println!("Compiled {zip_path:?}");
            }
            Ok(())
        }
        Err(e) => {
            println!("Failed {zip_path:?}\n{e:?}");
            zip_all = None;
            _ = zip_all;
            let _ = std::fs::remove_file(&result_zip_path);
            Ok(())
        }
    }
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
            if zip_path.extension() != Some(&OsStr::from("zip")) {
                continue;
            }

            compile_single_file(&engines, &runners_dir, &zip_path)
                .with_context(|| format!("processing {zip_path:?}"))?;
        }
    }

    Ok(())
}
