//! Example contract demonstrating DynArray storage.
//!
//! On deploy (is_init=true), populates a DynArray<u32> and prints its contents.

use genlayer_sdk::abi::entry::MessageData;
use genlayer_sdk::abi::entry::contract_def::Contract;
use genlayer_sdk::calldata::Value;
use genlayer_sdk::storage::{DynArray, Slot, StorageType};

const ROOT_SLOT: Slot = Slot([0u8; 32]);

#[derive(Default)]
pub struct StorageDynArrayExample;

impl Contract for StorageDynArrayExample {
    fn handle_main(&mut self, _message: MessageData, _data: bytes::Bytes) -> Result<Value, String> {
        let arr = <DynArray<u32>>::handle_at(ROOT_SLOT, 0);

        // populate
        arr.append_slot().set(10);
        arr.append_slot().set(20);
        arr.append_slot().set(30);

        // iterate and print
        let len = arr.len();
        println!("len={len}");
        for i in 0..len {
            let val = arr.index(i).get();
            println!("[{i}]={val}");
        }

        // pop last, print again
        arr.pop();
        let len = arr.len();
        println!("after pop len={len}");
        for i in 0..len {
            let val = arr.index(i).get();
            println!("[{i}]={val}");
        }

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

genlayer_sdk::contract_main!(StorageDynArrayExample);
