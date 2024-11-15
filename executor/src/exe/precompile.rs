use std::io::{Read, Write};

use anyhow::{Context, Result};
use clap::builder::OsStr;
use genvm::caching;
use itertools::Itertools;
use once_cell::sync::Lazy;
use zip::{read::ZipFile, ZipWriter};

#[derive(clap::Args, Debug)]
pub struct Args {}

fn compile_single_file(
    engines: &genvm::vm::Engines,
    runners_dir: &std::path::Path,
    zip_path: &std::path::Path,
) -> Result<()> {
    let base_path = zip_path
        .strip_prefix(&runners_dir)
        .with_context(|| format!("stripping {runners_dir:?} from {runners_dir:?}"))?;

    let mut zip_det: Option<ZipWriter<std::fs::File>> = None;
    let mut zip_non_det: Option<ZipWriter<std::fs::File>> = None;

    let precompile = |arch: &mut Option<ZipWriter<std::fs::File>>,
                      zip_path: &std::path::PathBuf,
                      engine: &wasmtime::Engine,
                      path: &str,
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
        arch.start_file(path, options)
            .with_context(|| format!("writing {path}"))?;
        arch.write_all(&precompiled)
            .with_context(|| "writing data")?;
        Ok(())
    };

    let (result_zip_det_path, result_zip_non_det_path) = match Lazy::force(&caching::PRECOMPILE_DIR)
    {
        Some(v) => v,
        None => anyhow::bail!("cache directory is not writable"),
    };
    let mut result_zip_det_path = result_zip_det_path.clone();
    result_zip_det_path.push(base_path);
    let mut result_zip_non_det_path = result_zip_non_det_path.clone();
    result_zip_non_det_path.push(base_path);

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
                &mut zip_det,
                &result_zip_det_path,
                &engines.det,
                &entry_name,
                &wasm_data,
            )
            .with_context(|| format!("processing det {entry_name}"))?;
            precompile(
                &mut zip_non_det,
                &result_zip_non_det_path,
                &engines.non_det,
                &entry_name,
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

    run_res = run_res.and(close_zip(zip_det));
    run_res = run_res.and(close_zip(zip_non_det));

    match run_res {
        Ok(()) => {
            if has_data {
                println!("Compiled {zip_path:?}");
            }
            Ok(())
        }
        Err(e) => {
            println!("Failed {zip_path:?}\n{e:?}");
            zip_det = None;
            zip_non_det = None;
            let _ = std::fs::remove_file(&result_zip_det_path);
            let _ = std::fs::remove_file(&result_zip_non_det_path);
            Ok(())
        }
    }
}

pub fn handle(_args: Args) -> Result<()> {
    let engines = genvm::vm::Engines::create(|_a| Ok(()))?;

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
