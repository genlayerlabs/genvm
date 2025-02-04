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

    let target = [
        std::env::var("CARGO_CFG_TARGET_ARCH").unwrap(),
        std::env::var("CARGO_CFG_TARGET_VENDOR").unwrap(),
        std::env::var("CARGO_CFG_TARGET_OS").unwrap(),
        std::env::var("CARGO_CFG_TARGET_ENV").unwrap(),
    ]
    .join("-");
    println!("cargo::rustc-env=GENVM_BUILD_ID={tag}_{target}");

    println!("cargo:rerun-if-env-changed=PROFILE");
    let profile = std::env::var("PROFILE").unwrap();
    println!("cargo::rustc-env=PROFILE={profile}");

    Ok(())
}
