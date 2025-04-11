fn main() -> std::io::Result<()> {
    println!("cargo:rerun-if-env-changed=GENVM_PROFILE_PATH");
    let tag: String = match std::env::var("GENVM_PROFILE_PATH") {
        Ok(path) => {
            println!("cargo:rerun-if-changed={path}");
            let tag = std::fs::read_to_string(std::path::PathBuf::from(&path))?;
            tag.trim().into()
        }
        Err(_) => "test".into(),
    };

    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();

    println!("cargo:rerun-if-env-changed=PROFILE");
    let profile = std::env::var("PROFILE").unwrap();
    println!("cargo::rustc-env=PROFILE={profile}");

    let tag = tag.replace("-", "_");
    let arch = arch.replace("-", "_");
    let os = os.replace("-", "_");
    let profile = profile.replace("-", "_");

    println!("cargo::rustc-env=GENVM_BUILD_ID={tag}-{arch}-{os}-{profile}");

    Ok(())
}
