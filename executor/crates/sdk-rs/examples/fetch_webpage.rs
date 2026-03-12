//! Example contract that fetches a webpage using WebRender.
//!
//! This demonstrates how to use the genlayer_sdk to make web requests
//! in a WASI contract, using nondet mode for consensus.

use std::collections::BTreeMap;
use std::io::Read;
use std::os::fd::FromRawFd;

use genlayer_sdk::abi::entry::MessageData;
use genlayer_sdk::abi::entry::contract_def::{
    ConsensusStageData, ConsensusStageParseError, Contract, LeaderResult,
};
use genlayer_sdk::abi::wasi;
use genlayer_sdk::calldata::{self, Value};
use serde::{Deserialize, Serialize};

const TARGET_URL: &str = "https://test-server.genlayer.com/static/genvm/hello.html";

/// Request to run a non-deterministic operation with leader/validator consensus
#[derive(Serialize)]
struct RunNondet {
    #[serde(with = "serde_bytes")]
    data_leader: Vec<u8>,
    #[serde(with = "serde_bytes")]
    data_validator: Vec<u8>,
}

/// Operations that can be performed in consensus stage
#[derive(Serialize, Deserialize)]
enum NondetOp {
    FetchWebpage,
}

#[derive(Debug)]
pub enum ContractError {
    Wasi(wasi::WasiError),
    IoError(std::io::Error),
    DecodeError(String),
    InvalidResponse(String),
}

impl std::fmt::Display for ContractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContractError::Wasi(e) => write!(f, "wasi error: {}", e),
            ContractError::IoError(e) => write!(f, "IO error: {}", e),
            ContractError::DecodeError(e) => write!(f, "decode error: {}", e),
            ContractError::InvalidResponse(e) => write!(f, "invalid response: {}", e),
        }
    }
}

impl From<wasi::WasiError> for ContractError {
    fn from(e: wasi::WasiError) -> Self {
        ContractError::Wasi(e)
    }
}

impl From<ConsensusStageParseError> for ContractError {
    fn from(e: ConsensusStageParseError) -> Self {
        ContractError::InvalidResponse(e.to_string())
    }
}

/// Helper to make a gl_call and read the response
fn gl_call_with_response(message: &Value) -> Result<Value, ContractError> {
    let encoded = calldata::encode(message);
    let fd = wasi::gl_call(&encoded)?;

    // fd of u32::MAX means no response data
    if fd == u32::MAX {
        return Ok(Value::Null);
    }

    // Read from the file descriptor
    let mut file = unsafe { std::fs::File::from_raw_fd(fd as i32) };
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .map_err(ContractError::IoError)?;

    // Decode the response
    let value = calldata::decode(&buffer).map_err(|e| ContractError::DecodeError(e.to_string()))?;

    // Check for ok/error wrapper
    if let Value::Map(ref map) = value {
        if let Some(ok_value) = map.get("ok") {
            return Ok(ok_value.clone());
        }
        if let Some(err_value) = map.get("error") {
            return Err(ContractError::InvalidResponse(format!("{:?}", err_value)));
        }
    }

    Ok(value)
}

/// Response from WebRender
#[derive(Deserialize)]
struct WebRenderResponse {
    text: String,
}

/// Fetch a webpage using WebRender (must be called in nondet context)
fn fetch_webpage(url: &str, mode: &str) -> Result<String, ContractError> {
    // Construct the WebRender message as a Value
    let message = Value::Map(BTreeMap::from([(
        "WebRender".to_owned(),
        Value::Map(BTreeMap::from([
            ("mode".to_owned(), Value::Str(mode.to_owned())),
            ("url".to_owned(), Value::Str(url.to_owned())),
            ("wait_after_loaded".to_owned(), Value::Str("0ms".to_owned())),
        ])),
    )]));

    let response = gl_call_with_response(&message)?;

    // Deserialize the response
    let web_response: WebRenderResponse =
        calldata::from_value(response).map_err(|e| ContractError::DecodeError(e.to_string()))?;

    Ok(web_response.text)
}

/// Run a nondet operation with leader/validator consensus.
/// The entry_data is passed back to handle_consensus_stage.
/// Returns the leader's result after consensus is reached.
fn run_nondet(entry_data: &[u8]) -> Result<Value, ContractError> {
    let request = RunNondet {
        data_leader: entry_data.to_vec(),
        data_validator: entry_data.to_vec(),
    };

    let message =
        calldata::to_value(&request).map_err(|e| ContractError::DecodeError(e.to_string()))?;
    let wrapped = Value::Map(std::collections::BTreeMap::from([(
        "RunNondet".to_owned(),
        message,
    )]));

    // Make the gl_call
    let encoded = calldata::encode(&wrapped);
    let fd = wasi::gl_call(&encoded)?;

    if fd == u32::MAX {
        return Err(ContractError::InvalidResponse(
            "no response from RunNondet".to_owned(),
        ));
    }

    // Read the response - it has format [ResultCode byte][calldata encoded value]
    let mut file = unsafe { std::fs::File::from_raw_fd(fd as i32) };
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .map_err(ContractError::IoError)?;

    let result = LeaderResult::parse(&buffer)?;

    match result {
        LeaderResult::Return(value) => Ok(value),
        LeaderResult::UserError(msg) => {
            genlayer_sdk::abi::entry::user_error_immediately(msg);
        }
        LeaderResult::VmError(msg) => {
            Err(ContractError::InvalidResponse(format!("vm error: {}", msg)))
        }
    }
}

/// Contract handler that fetches a webpage
#[derive(Default)]
pub struct FetchWebpageHandler;

impl Contract for FetchWebpageHandler {
    fn handle_main(&mut self, _message: MessageData, _data: bytes::Bytes) -> Result<Value, String> {
        let op = NondetOp::FetchWebpage;
        let entry_data = calldata::encode(&calldata::to_value(&op).map_err(|e| e.to_string())?);

        run_nondet(&entry_data).map_err(|e| e.to_string())
    }

    fn handle_sandbox(
        &mut self,
        _message: MessageData,
        _data: bytes::Bytes,
    ) -> Result<Vec<u8>, String> {
        unimplemented!()
    }

    fn handle_consensus_stage(
        &mut self,
        _message: MessageData,
        data: bytes::Bytes,
        stage_data: ConsensusStageData,
    ) -> Result<Value, String> {
        let op_value = calldata::decode(&data).map_err(|e| e.to_string())?;
        println!("op_value={:?}", op_value);
        let _op: NondetOp = calldata::from_value(op_value).map_err(|e| e.to_string())?;

        match stage_data {
            ConsensusStageData::Leader => {
                println!("I am leader!");
                let content = fetch_webpage(TARGET_URL, "text").map_err(|e| e.to_string())?;
                Ok(Value::Str(content))
            }
            ConsensusStageData::Validator { leaders_result } => {
                println!("I am validator! leaders_result={:?}", leaders_result);

                let LeaderResult::Return(leader_value) = leaders_result else {
                    // Leader had an error - disagree
                    return Ok(Value::Bool(false));
                };

                let Value::Str(leader_content) = leader_value else {
                    return Err("leader result not string".to_owned());
                };

                // Fetch our own copy and compare
                let our_content = fetch_webpage(TARGET_URL, "text").map_err(|e| e.to_string())?;

                if leader_content == our_content {
                    // Agreement - return true (as bool value)
                    Ok(Value::Bool(true))
                } else {
                    // Disagreement
                    Ok(Value::Bool(false))
                }
            }
        }
    }
}

// Define the main entry point using the macro
genlayer_sdk::contract_main!(FetchWebpageHandler);
