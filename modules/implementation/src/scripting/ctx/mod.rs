use std::{collections::BTreeMap, sync::Arc};

use crate::common::ModuleError;

use genvm_common::*;

pub mod dflt;
pub mod filters;
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
    /// Client for the request itself. For the web module this is the filtering
    /// client (SSRF guard) unless the request opts out via `unfiltered`.
    pub client: reqwest::Client,
    /// Plain client used when a request sets `unfiltered` (allowlisted hosts).
    pub client_unfiltered: reqwest::Client,
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

impl<W: calldata::Writer> calldata::codec::Encode<W> for Metrics {
    type Error = W::Error;

    fn encode(&self, enc: &mut calldata::Encoder<W>) -> std::result::Result<(), Self::Error> {
        enc.start_map(2)?;
        enc.push_map_k("requests_count")?;
        calldata::codec::Encode::encode(&self.requests_count, enc)?;
        enc.push_map_k("requests_time")?;
        calldata::codec::Encode::encode(&self.requests_time, enc)?;
        Ok(())
    }
}
