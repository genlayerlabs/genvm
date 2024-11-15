fn main() -> std::io::Result<()> {
    println!("cargo:rerun-if-env-changed=GENVM_PROFILE_PATH");
    let path = std::env::var("GENVM_PROFILE_PATH").unwrap();
    println!("cargo:rerun-if-changed={path}");
    let tag = std::fs::read_to_string(std::path::PathBuf::from(&path))?;
    let tag = tag.trim();
    let target = vec![
        std::env::var("CARGO_CFG_TARGET_ARCH").unwrap(),
        std::env::var("CARGO_CFG_TARGET_VENDOR").unwrap(),
        std::env::var("CARGO_CFG_TARGET_OS").unwrap(),
        std::env::var("CARGO_CFG_TARGET_ENV").unwrap(),
        std::env::var("CARGO_CFG_TARGET_ABI").unwrap(),
    ]
    .join("-");
    println!("cargo::rustc-env=GENVM_BUILD_ID={tag}_{target}");

    println!("cargo:rerun-if-env-changed=PROFILE");
    let profile = std::env::var("PROFILE").unwrap();
    println!("cargo::rustc-env=PROFILE={profile}");

    Ok(())
}
