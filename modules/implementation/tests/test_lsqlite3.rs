use mlua::prelude::*;

fn make_vm() -> Lua {
    let libs = mlua::StdLib::COROUTINE
        | mlua::StdLib::TABLE
        | mlua::StdLib::IO
        | mlua::StdLib::STRING
        | mlua::StdLib::MATH
        | mlua::StdLib::PACKAGE;
    let vm = Lua::new_with(libs, mlua::LuaOptions::default()).unwrap();
    vm.load_std_libs(libs).unwrap();
    genvm_modules::scripting::preload_lsqlite3(&vm).unwrap();
    vm
}

fn eval<T: mlua::FromLua>(vm: &Lua, code: &str) -> T {
    vm.load(code).eval::<T>().unwrap()
}

#[test]
fn test_require_lsqlite3() {
    let vm = make_vm();
    let _: LuaTable = eval(&vm, "return require('lsqlite3')");
}

#[test]
fn test_open_memory_db() {
    let vm = make_vm();
    let status: i64 = eval(
        &vm,
        r#"
        local sqlite3 = require('lsqlite3')
        local db = sqlite3.open_memory()
        local rc = db:exec("CREATE TABLE t(x INTEGER)")
        db:close()
        return rc
        "#,
    );
    assert_eq!(status, 0);
}

#[test]
fn test_insert_and_select() {
    let vm = make_vm();
    let result: String = eval(
        &vm,
        r#"
        local sqlite3 = require('lsqlite3')
        local db = sqlite3.open_memory()
        db:exec("CREATE TABLE t(x TEXT)")
        db:exec("INSERT INTO t VALUES('hello')")
        local val
        for row in db:nrows("SELECT x FROM t") do
            val = row.x
        end
        db:close()
        return val
        "#,
    );
    assert_eq!(result, "hello");
}
