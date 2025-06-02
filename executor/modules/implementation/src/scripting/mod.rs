pub mod pool;

mod ctx;

use std::{future::Future, sync::Arc};

pub struct RSContext<C> {
    pub client: reqwest::Client,
    pub data: Arc<C>,
}

pub type CtxCreator<C> =
    dyn Fn(&RSContext<C>, &mlua::Lua, &mlua::Table) -> anyhow::Result<()> + Send + Sync;

pub struct UserVM<T, C> {
    pub vm: mlua::Lua,
    pub data: T,

    ctx_creators: Vec<Box<CtxCreator<C>>>,
}

impl<T, C> UserVM<T, C> {
    pub fn add_ctx_creator(&mut self, creator: Box<CtxCreator<C>>) {
        self.ctx_creators.push(creator);
    }

    pub fn create_ctx(&self, rs_ctx: &RSContext<C>) -> anyhow::Result<mlua::Value> {
        let ctx = self.vm.create_table()?;

        for c in &self.ctx_creators {
            c(rs_ctx, &self.vm, &ctx)?;
        }

        Ok(mlua::Value::Table(ctx))
    }

    pub async fn create<F>(
        extra_lua_path: &str,
        data_getter: impl FnOnce(mlua::Lua) -> F,
    ) -> anyhow::Result<Self>
    where
        F: Future<Output = anyhow::Result<T>>,
    {
        use mlua::StdLib;

        let lua_lib_path = {
            let mut lua_lib_path = std::env::current_exe()?;
            lua_lib_path.pop();
            lua_lib_path.pop();
            lua_lib_path.push("share");
            lua_lib_path.push("lib");
            lua_lib_path.push("genvm");
            lua_lib_path.push("greyboxing");

            let mut path = lua_lib_path
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("could not detect default lib path"))?
                .to_owned();
            path.push_str("/?.lua");

            if !extra_lua_path.is_empty() {
                path.push(';');
                path.push_str(extra_lua_path);
            }

            log::info!(path = path; "lua path");

            path
        };

        std::env::set_var("LUA_PATH", &lua_lib_path);

        let lua_libs = StdLib::COROUTINE
            | StdLib::TABLE
            | StdLib::IO
            | StdLib::STRING
            | StdLib::MATH
            | StdLib::PACKAGE;

        let vm = mlua::Lua::new_with(lua_libs, mlua::LuaOptions::default())?;

        vm.load_std_libs(lua_libs)?;

        vm.globals().set("__dflt", ctx::dflt::create_global(&vm)?)?;

        let mut ctx_creators: Vec<Box<CtxCreator<C>>> = Vec::new();

        ctx_creators.push(Box::new(|rs_ctx, vm, ctx| {
            let my_ctx = vm.create_userdata(ctx::dflt::CtxPart {
                client: rs_ctx.client.clone(),
            })?;

            ctx.set("__ctx_dflt", my_ctx)?;

            Ok(())
        }));

        Ok(Self {
            data: data_getter(vm.clone()).await?,
            ctx_creators,
            vm,
        })
    }

    pub async fn call_fn<R>(
        &self,
        f: &mlua::Function,
        args: impl mlua::IntoLuaMulti,
    ) -> anyhow::Result<R>
    where
        R: mlua::FromLuaMulti,
    {
        let res = f.call_async(args).await;

        match res {
            Ok(res) => Ok(res),
            Err(mlua::Error::ExternalError(e)) => Err(anyhow::Error::from(e)),
            Err(mlua::Error::WithContext { context, cause }) => {
                Err(anyhow::Error::from(cause).context(context))
            }
            Err(e) => Err(anyhow::Error::from(e)),
        }
    }
}

pub const DEFAULT_LUA_SER_OPTIONS: mlua::SerializeOptions = mlua::SerializeOptions::new()
    .serialize_none_to_null(false)
    .serialize_unit_to_null(false);

pub async fn load_script<P>(vm: &mlua::Lua, path: P) -> anyhow::Result<()>
where
    P: AsRef<std::path::Path> + Into<String>,
{
    let script_contents = std::fs::read_to_string(&path)?;
    let chunk = vm.load(script_contents);
    let chunk = chunk.set_name(path.into());
    chunk.exec_async().await?;

    Ok(())
}
