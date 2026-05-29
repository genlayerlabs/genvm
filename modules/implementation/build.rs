use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-env-changed=LSQLITE3_SRC");
    println!("cargo:rerun-if-env-changed=LUA_INCLUDE");
    println!("cargo:rerun-if-env-changed=LSQLITE3_PREBUILT");

    if env::var("LSQLITE3_PREBUILT").is_ok() {
        println!("cargo:rustc-link-lib=static=lsqlite3_bundle");
        return;
    }

    let lsqlite3_src = match env::var("LSQLITE3_SRC") {
        Ok(v) => PathBuf::from(v),
        Err(_) => {
            panic!("either LSQLITE3_PREBUILT or LSQLITE3_SRC must be set");
        }
    };

    let lua_include =
        env::var("LUA_INCLUDE").expect("LUA_INCLUDE must be set when LSQLITE3_SRC is set");

    cc::Build::new()
        .file(lsqlite3_src.join("sqlite3.c"))
        .define("SQLITE_CORE", None)
        .include(&lsqlite3_src)
        .opt_level(2)
        .warnings(false)
        .compile("sqlite3");

    cc::Build::new()
        .file(lsqlite3_src.join("lsqlite3.c"))
        .include(&lua_include)
        .include(&lsqlite3_src)
        .opt_level(2)
        .warnings(false)
        .compile("lsqlite3");
}
