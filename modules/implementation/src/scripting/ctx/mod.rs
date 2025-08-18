use std::{collections::BTreeMap, sync::Arc};

use crate::common::ModuleError;

use genvm_common::*;

pub mod dflt;
pub mod req;
mod signing;

fn arc_to_ref<T>(x: &Arc<T>) -> &T
where
    T: ?Sized,
{
    x
}

pub(super) fn try_unwrap_err(err: &mlua::Error) -> Option<ModuleError> {
    match err {
        mlua::Error::ExternalError(e) => ModuleError::try_unwrap_dyn(arc_to_ref(e)),
        mlua::Error::CallbackError { cause, traceback } => try_unwrap_err(cause).inspect(|_e| {
            let _ = traceback;
            // I wonder if we should keep it...
            //e.causes.push(traceback.clone());
        }),
        _ => None,
    }
}

pub struct CtxPart {
    pub client: reqwest::Client,
    pub sign_url: Arc<str>,
    pub sign_headers: Arc<BTreeMap<String, String>>,
    pub sign_vars: BTreeMap<String, String>,
    pub node_address: String,
    pub metrics: sync::DArc<Metrics>,
    pub hello: Arc<genvm_modules_interfaces::GenVMHello>,
}

impl mlua::UserData for CtxPart {}

#[derive(Debug, serde::Serialize, Default)]
pub struct Metrics {
    pub requests_count: stats::metric::Count,
    pub requests_time: stats::metric::Time,
}
