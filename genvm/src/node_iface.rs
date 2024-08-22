use std::sync::Arc;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, base64::Base64};

pub struct StoragePartDesc {
    pub account: Address,
    pub desc: u32
}

#[serde_as]
#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub struct Address(#[serde_as(as = "Base64")] pub [u8; 32]);

impl Address {
    pub fn raw(&self) -> [u8; 32] {
        let Address(r) = self;
        *r
    }
}

pub struct Gas(pub u64);

#[derive(Serialize, Deserialize, Clone)]
pub struct Calldata {
    pub method: String,
    pub args: Vec<serde_json::Value>
}

#[derive(Serialize, Deserialize, Clone)]
pub enum Entrypoint {
    /// See [Calldata]
    Call(String),
    Nondet { eq_principe: String, data: Vec<u8> },
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MessageData {
    pub gas: u64,
    pub contract_account: Address,
    pub sender_account: Address,
    pub value: Option<u64>,
    pub entrypoint: Entrypoint,
}

#[derive(Serialize, Deserialize)]
pub struct RunnerDescription {
    pub lang: String
}

#[derive(Serialize, Deserialize, Clone)]
pub enum InitAction {
    MapFile { to: String, contents: Vec<u8> },
    MapCode { to: String },
    AddEnv { name: String, val: String },
    SetArgs { args: Vec<String> },
    LinkWasm { contents: Vec<u8>, debug_path: Option<String> },
    StartWasm { contents: Vec<u8>, debug_path: Option<String> },
}

pub trait InitApi {
    fn get_initial_data(&mut self) -> Result<MessageData>;

    fn get_code(&mut self, account: &Address) -> Result<Arc<Vec<u8>>>;
}

pub trait RunnerApi {
    fn get_runner(&mut self, desc: RunnerDescription) -> Result<Vec<InitAction>>;
}

#[allow(dead_code)]
pub trait StorageApi {
    fn storage_part_get_size(&mut self, remaing_gas: &mut Gas, part: StoragePartDesc) -> Result<u32>;

    fn storage_part_resize(&mut self, remaing_gas: &mut Gas, part: StoragePartDesc, new_size: u32) -> Result<()>;

    fn storage_part_read(&mut self, remaing_gas: &mut Gas, part: StoragePartDesc, index: u32, size: u32, buf: &mut Vec<u8>) -> Result<()>;

    fn storage_part_write(&mut self, remaing_gas: &mut Gas, part: StoragePartDesc, index: u32, size: u32, buf: &mut Vec<u8>) -> Result<()>;
}

#[allow(dead_code)]
pub trait NondetSupportApi {
    fn equivalence_principle_fast_return(&mut self, remaing_gas: &mut Gas, call_no: u32) -> Result<Option<Vec<u8>>>;

    fn equivalence_principle(&mut self, remaing_gas: &mut Gas, call_no: u32, context: &str, current_result: Vec<u8>) -> Result<Vec<u8>>;
}

#[allow(dead_code)]
pub trait NondetFunctionsApi {
    fn get_webpage(&mut self, remaing_gas: &mut Gas, url: &str) -> Result<String>;
}

#[allow(dead_code)]
pub trait ContractsApi {
    //fn run_external_view(&mut self, remaing_gas: &mut Gas, target: Address, calldata: Calldata) -> Result<String>;
    fn post_message(&mut self, remaing_gas: &mut Gas, data: MessageData, when: ()) -> Result<()>;
}
