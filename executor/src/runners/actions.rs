use genvm_common::*;
use std::sync::Arc;

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, PartialEq, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum WasmMode {
    Det,
    Nondet,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub enum InitAction {
    MapFile {
        to: Arc<str>,
        file: Arc<str>,
    },
    AddEnv {
        name: String,
        val: String,
    },
    SetArgs(Vec<String>),
    Depends(
        #[serde(deserialize_with = "util::global_symbol_deserialize")] symbol_table::GlobalSymbol,
    ),
    LinkWasm(Arc<str>),
    StartWasm(Arc<str>),

    When {
        cond: WasmMode,
        action: Box<InitAction>,
    },
    Seq(Vec<InitAction>),

    With {
        #[serde(deserialize_with = "util::global_symbol_deserialize")]
        runner: symbol_table::GlobalSymbol,
        action: Box<InitAction>,
    },
}
