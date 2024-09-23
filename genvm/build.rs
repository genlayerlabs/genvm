fn main() {
    let profile = std::env::var("PROFILE").unwrap();
    println!("cargo::rustc-env=PROFILE={profile}");
    println!("cargo:rerun-if-env-changed=PROFILE");
}
