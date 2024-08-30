use std::sync::Arc;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, base64::Base64};

pub struct StorageSlot {
    pub account: Address,
    pub desc: Address,
}

#[serde_as]
#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub struct Address(#[serde_as(as = "Base64")] pub [u8; 32]);

impl Address {
    pub fn raw(&self) -> [u8; 32] {
        let Address(r) = self;
        *r
    }

    pub fn new() -> Self {
        Self([0; 32])
    }
}

pub struct Gas(pub u64);

#[derive(Debug)]
pub enum VMRunResult {
    Return(String),
    Rollback(String),
    /// TODO: should there be an error or should it be merged with rollback?
    Error(String),
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Calldata {
    pub method: String,
    pub args: Vec<serde_json::Value>
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MessageData {
    pub gas: u64,
    pub contract_account: Address,
    pub sender_account: Address,
    pub value: Option<u64>,
    pub is_init: bool,
}

#[derive(Serialize, Deserialize)]
pub struct RunnerDescription {
    pub lang: String
}

pub trait InitApi {
    fn get_initial_data(&mut self) -> Result<MessageData>;

    fn get_calldata(&mut self) -> Result<String>;

    fn get_code(&mut self, account: &Address) -> Result<Arc<Vec<u8>>>;
}

#[derive(Serialize, Deserialize, Clone)]
pub enum InitAction {
    MapFile { to: String, contents: Arc<Vec<u8>> },
    MapCode { to: String },
    AddEnv { name: String, val: String },
    SetArgs { args: Vec<String> },
    LinkWasm { contents: Arc<Vec<u8>>, debug_path: Option<String> },
    StartWasm { contents: Arc<Vec<u8>>, debug_path: Option<String> },
}

pub trait RunnerApi {
    fn get_runner(&mut self, desc: RunnerDescription) -> Result<Vec<InitAction>>;
}

#[allow(dead_code)]
pub trait StorageApi {
    fn storage_read(&mut self, remaing_gas: &mut Gas, slot: StorageSlot, index: u32, buf: &mut [u8]) -> Result<()>;

    fn storage_write(&mut self, remaing_gas: &mut Gas, slot: StorageSlot, index: u32, buf: &[u8]) -> Result<()>;
}

#[allow(dead_code)]
pub trait NondetSupportApi {
    fn equivalence_principle_fast_return(&mut self, remaing_gas: &mut Gas, call_no: u32) -> Result<Option<VMRunResult>>;

    fn equivalence_principle(&mut self, remaing_gas: &mut Gas, call_no: u32, context: &str, current_result: Vec<u8>) -> Result<VMRunResult>;
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
