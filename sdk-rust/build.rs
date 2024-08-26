use std::{fs, process::Command};

fn main() -> anyhow::Result<()> {
    let cargo_dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);

    let repo_dir = cargo_dir.parent().ok_or(anyhow::anyhow!("no parent"))?;

    let mut cwd = std::path::PathBuf::from(repo_dir);
    cwd.extend("third-party/wasi-rs/crates/witx-bindgen/".split("/"));

    let mut file = std::path::PathBuf::from(repo_dir);
    file.extend("genvm/src/wasi/witx/genlayer_sdk.witx".split("/"));

    let out =
        Command::new(std::env::var("CARGO")?)
            .current_dir(cwd)
            .args(["run", file.to_str().ok_or(anyhow::anyhow!("file isn't path"))?])
            .output()?;

    if !out.status.success() {
        String::from_utf8(out.stderr).iter().chain(String::from_utf8(out.stdout).iter()).for_each(|x| {
            eprintln!("{}", x);
        });

        return Err(anyhow::anyhow!("couldn't compile"));
    }

    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR")?);
    fs::write(out_dir.join("generated.rs"), out.stdout)?;

    println!("cargo::rerun-if-changed={}", file.display());

    Ok(())
}
