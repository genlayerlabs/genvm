use mlua::LuaSerdeExt;

use crate::scripting;

use super::{config, domains};

pub struct VMData {
    pub render: mlua::Function,
    pub request: mlua::Function,
}

pub struct CtxPart {
    //pub dflt_ctx: Arc<scripting::CtxPart>,
    //pub config: sync::DArc<config::Config>,
}

impl mlua::UserData for CtxPart {}
pub fn create_global(vm: &mlua::Lua, config: &config::Config) -> anyhow::Result<mlua::Value> {
    let web = vm.create_table()?;

    web.set(
        "config",
        vm.to_value_with(config, scripting::DEFAULT_LUA_SER_OPTIONS)?,
    )?;

    let tld = vm.create_table_from(domains::DOMAINS.iter().map(|k| (*k, true)))?;
    for k in &config.extra_tld {
        tld.set(&**k, true)?;
    }
    web.set("allowed_tld", tld)?;

    Ok(mlua::Value::Table(web))
}
