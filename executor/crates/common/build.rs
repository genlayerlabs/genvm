fn main() -> std::io::Result<()> {
    println!("cargo::rerun-if-env-changed=CARGO_LD_LIBRARY_PATH");
    if let Ok(v) = std::env::var("CARGO_LD_LIBRARY_PATH") {
        for path in v.split(':').filter(|p| !p.is_empty()) {
            println!("cargo::rustc-link-search=native={path}");
        }
    }

    println!("cargo::rerun-if-env-changed=GENVM_PROFILE");
    let tag: String = std::env::var("GENVM_PROFILE").unwrap_or("vTEST".into());

    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();

    println!("cargo::rerun-if-env-changed=PROFILE");
    let profile = std::env::var("PROFILE").unwrap();
    println!("cargo::rustc-env=PROFILE={profile}");

    // tag can have `-`, parse from the right
    let arch = arch.replace("-", "_");
    let os = os.replace("-", "_");
    let profile = profile.replace("-", "_");

    println!("cargo::rustc-env=GENVM_BUILD_ID={tag}-{arch}-{os}-{profile}");

    Ok(())
}
