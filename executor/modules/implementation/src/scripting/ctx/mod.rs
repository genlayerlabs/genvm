use std::sync::Arc;

use crate::common::ModuleError;

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
        mlua::Error::CallbackError { cause, traceback } => try_unwrap_err(cause).map(|mut e| {
            e.causes.push(traceback.clone());

            e
        }),
        _ => None,
    }
}
