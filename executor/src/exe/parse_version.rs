use std::io::{Read, Write};

use anyhow::Result;
use genvm::config;

use genvm_common::*;

#[derive(clap::Args, Debug)]
pub struct Args {}

pub fn handle(_args: Args, _config: config::Config) -> Result<()> {
    let mut code = Vec::new();
    std::io::stdin().read_to_end(&mut code)?;

    let code = util::SharedBytes::new(code);
    let arch = genvm::runners::parse(code)?;
    let version = arch.data.get("version");

    if let Some(v) = version {
        std::io::stdout().write_all(v.as_ref())?;
    } else {
        std::io::stdout().write_all("v*.*.*".as_bytes())?;
    }

    Ok(())
}
