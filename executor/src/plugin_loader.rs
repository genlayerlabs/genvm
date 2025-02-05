pub fn default_plugin_path() -> std::io::Result<std::path::PathBuf> {
    let mut buf = std::env::current_exe()?;
    buf.pop();
    buf.pop();
    buf.push("lib");
    buf.push("genvm-modules");
    Ok(buf)
}
