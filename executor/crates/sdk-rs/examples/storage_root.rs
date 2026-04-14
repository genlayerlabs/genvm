//! Example contract demonstrating Root access, code introspection, and contract data.

use genlayer_sdk::abi::entry::MessageData;
use genlayer_sdk::abi::entry::contract_def::Contract;
use genlayer_sdk::calldata::Value;
use genlayer_sdk::storage::Root;

const MAGIC: &str = "MAGIC_MARKER_42";

genlayer_sdk::record!(ContractData { counter: u32 });

#[derive(Default)]
pub struct StorageRootExample;

impl Contract for StorageRootExample {
    fn handle_main(&mut self, _message: MessageData, _data: bytes::Bytes) -> Result<Value, String> {
        let root = Root::<ContractData>::get();

        // search own code for the magic string
        let code = root.code().get();
        let code_len = code.len();
        let mut code_bytes = vec![0u8; code_len as usize];
        if code_len > 0 {
            code.slot.read(code.offset + 4, &mut code_bytes);
        }
        let found = code_bytes
            .windows(MAGIC.len())
            .any(|w| w == MAGIC.as_bytes());
        println!("contains_magic={found}");

        // write and read contract data through Root
        let data = root.contract_instance().get();
        data.counter().set(42);
        let val = data.counter().get();
        println!("counter={val}");

        Ok(Value::Null)
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
        _data: bytes::Bytes,
        _stage_data: genlayer_sdk::abi::entry::contract_def::ConsensusStageData,
    ) -> Result<Value, String> {
        unimplemented!()
    }
}

genlayer_sdk::contract_main!(StorageRootExample);
