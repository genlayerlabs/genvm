mod host_fns;
pub mod message;

use genvm_common::*;

use crate::public_abi::{ResultCode, StorageType};
use genvm_common::calldata::Address;
use genvm_common::calldata::ADDRESS_SIZE;
use message::root_offsets;

use core::str;
use std::collections::BTreeMap;
use std::sync::Mutex;

use anyhow::{Context, Result};

use crate::calldata;
use crate::errors::VMError;
use crate::memlimiter;
use crate::vm::{self, RunOk};
pub use message::{MessageData, SlotID};

trait Sock: std::io::Read + std::io::Write + Send + Sync {}

impl Sock for bufreaderwriter::seq::BufReaderWriterSeq<std::os::unix::net::UnixStream> {}

impl Sock for bufreaderwriter::seq::BufReaderWriterSeq<std::net::TcpStream> {}

pub struct Host {
    sock: Box<Mutex<dyn Sock>>,
}

impl Host {
    pub fn new(addr: &str) -> Result<Host> {
        const UNIX: &str = "unix://";
        let sock: Box<Mutex<dyn Sock>> = if let Some(addr_suff) = addr.strip_prefix(UNIX) {
            Box::new(Mutex::new(
                bufreaderwriter::seq::BufReaderWriterSeq::new_writer(
                    std::os::unix::net::UnixStream::connect(std::path::Path::new(addr_suff))
                        .with_context(|| format!("connecting to {addr}"))?,
                ),
            ))
        } else {
            Box::new(Mutex::new(
                bufreaderwriter::seq::BufReaderWriterSeq::new_writer(
                    std::net::TcpStream::connect(addr)
                        .with_context(|| format!("connecting to {addr}"))?,
                ),
            ))
        };
        Ok(Host { sock })
    }
}

fn read_u32(sock: &mut dyn Sock) -> Result<u32> {
    let mut int_buf = [0; 4];
    sock.read_exact(&mut int_buf)?;
    Ok(u32::from_le_bytes(int_buf))
}

fn read_bytes(sock: &mut dyn Sock) -> Result<Box<[u8]>> {
    let len = read_u32(sock)?;

    let res = Box::new_uninit_slice(len as usize);
    let mut res = unsafe { res.assume_init() };
    sock.read_exact(&mut res)?;
    Ok(res)
}

fn write_slice(sock: &mut dyn Sock, data: &[u8]) -> Result<()> {
    let len = data.len() as u32;

    sock.write_all(&len.to_le_bytes())?;
    sock.write_all(data)?;

    Ok(())
}

fn read_host_error(sock: &mut dyn Sock) -> Result<host_fns::Errors> {
    let mut has_some = [0; 1];
    sock.read_exact(&mut has_some)?;

    host_fns::Errors::try_from(has_some[0])
        .map_err(|_| anyhow::anyhow!("invalid error id {}", has_some[0]))
}

fn handle_host_error(sock: &mut dyn Sock) -> Result<()> {
    let e = read_host_error(sock)?;

    if e == host_fns::Errors::Ok {
        Ok(())
    } else {
        Err(crate::errors::VMError(e.str_snake_case().to_owned(), None).into())
    }
}

pub struct LockedSlotsSet(Box<[SlotID]>);

impl LockedSlotsSet {
    pub fn contains(&self, slot: SlotID) -> bool {
        self.0.binary_search(&slot).is_ok()
    }
}

impl Host {
    pub fn get_calldata(&mut self, calldata: &mut Vec<u8>) -> Result<()> {
        let Ok(mut sock) = (*self.sock).lock() else {
            anyhow::bail!("can't take lock")
        };
        let sock: &mut dyn Sock = &mut *sock;
        sock.write_all(&[host_fns::Methods::GetCalldata as u8])?;

        handle_host_error(sock)?;

        let len = read_u32(sock)? as usize;
        calldata.reserve(len);
        let index = calldata.len();
        unsafe {
            calldata.set_len(index + len);
        }
        sock.read_exact(&mut calldata[index..index + len])?;
        Ok(())
    }

    fn get_locked_slots(
        &mut self,
        contract_address: calldata::Address,
        limiter: &memlimiter::Limiter,
    ) -> Result<LockedSlotsSet> {
        let locked_slot = SlotID::ZERO.indirection(root_offsets::LOCKED_SLOTS);

        let mut len_buf = [0; 4];
        self.storage_read(
            StorageType::Default,
            contract_address,
            locked_slot,
            0,
            &mut len_buf,
        )?;
        let len = u32::from_le_bytes(len_buf);

        if !limiter.consume_mul(len, SlotID::SIZE) {
            return Err(VMError::oom(None).into());
        }

        let res = Box::new_uninit_slice(len as usize);
        let mut res = unsafe { res.assume_init() };

        let read_to = unsafe {
            std::slice::from_raw_parts_mut(
                res.as_mut_ptr() as *mut u8,
                (len * SlotID::SIZE) as usize,
            )
        };
        self.storage_read(
            StorageType::Default,
            contract_address,
            locked_slot,
            4,
            read_to,
        )?;

        res.sort();

        Ok(LockedSlotsSet(res))
    }

    pub fn get_locked_slots_for_sender(
        &mut self,
        contract_address: calldata::Address,
        sender: calldata::Address,
        limiter: &memlimiter::Limiter,
    ) -> Result<LockedSlotsSet> {
        let upgraders_slot = SlotID::ZERO.indirection(root_offsets::UPGRADERS);

        let mut len_buf = [0; 4];
        self.storage_read(
            StorageType::Default,
            contract_address,
            upgraders_slot,
            0,
            &mut len_buf,
        )?;
        let len = u32::from_le_bytes(len_buf);

        for i in 0..len {
            let mut read_sender = [0; ADDRESS_SIZE];

            self.storage_read(
                StorageType::Default,
                contract_address,
                upgraders_slot,
                4 + i * Address::SIZE,
                &mut read_sender,
            )?;

            if read_sender == sender.raw() {
                return Ok(LockedSlotsSet(Box::from([])));
            }
        }

        self.get_locked_slots(contract_address, limiter)
    }

    pub fn get_code(
        &mut self,
        mode: StorageType,
        account: calldata::Address,
        limiter: &memlimiter::Limiter,
    ) -> Result<Box<[u8]>> {
        let code_slot = SlotID::ZERO.indirection(root_offsets::CODE);

        let mut len_buf = [0; 4];
        self.storage_read(mode, account, code_slot, 0, &mut len_buf)?;
        let code_size = u32::from_le_bytes(len_buf);

        if !limiter.consume(code_size) {
            return Err(VMError::oom(None).into());
        }

        let res = Box::new_uninit_slice(code_size as usize);
        let mut res = unsafe { res.assume_init() };

        self.storage_read(mode, account, code_slot, 4, &mut res)?;

        Ok(res)
    }

    pub fn storage_read(
        &mut self,
        mode: StorageType,
        account: calldata::Address,
        slot: SlotID,
        index: u32,
        buf: &mut [u8],
    ) -> Result<()> {
        let Ok(mut sock) = (*self.sock).lock() else {
            anyhow::bail!("can't take lock")
        };
        let sock: &mut dyn Sock = &mut *sock;
        sock.write_all(&[host_fns::Methods::StorageRead as u8])?;
        sock.write_all(&[mode as u8; 1])?;
        sock.write_all(&account.raw())?;
        sock.write_all(&slot.raw())?;
        sock.write_all(&index.to_le_bytes())?;
        sock.write_all(&(buf.len() as u32).to_le_bytes())?;

        handle_host_error(sock)?;

        sock.read_exact(buf)?;

        log_trace!(slot:? = slot.0, index = index, data:serde = buf; "read");

        Ok(())
    }

    pub fn storage_write(&mut self, slot: SlotID, index: u32, buf: &[u8]) -> Result<()> {
        let Ok(mut sock) = (*self.sock).lock() else {
            anyhow::bail!("can't take lock")
        };
        let sock: &mut dyn Sock = &mut *sock;
        sock.write_all(&[host_fns::Methods::StorageWrite as u8])?;
        sock.write_all(&slot.raw())?;
        sock.write_all(&index.to_le_bytes())?;
        write_slice(sock, buf)?;

        sock.flush()?;

        handle_host_error(sock)?;

        Ok(())
    }

    pub fn consume_result(&mut self, res: &Result<vm::FullRunOk>) -> Result<()> {
        log_trace!("consume_result");

        let Ok(mut sock) = (*self.sock).lock() else {
            anyhow::bail!("can't take lock")
        };
        let sock: &mut dyn Sock = &mut *sock;
        let data = match res {
            Ok((RunOk::Return(data), _)) => {
                let mut encoded = Vec::from([ResultCode::Return as u8]);
                encoded.extend_from_slice(data);

                encoded
            }
            Ok((RunOk::UserError(data), fp)) => {
                let fp = calldata::to_value(fp)?;
                let val = calldata::Value::Map(BTreeMap::from([
                    ("message".to_owned(), data.as_str().into()),
                    ("fingerprint".to_owned(), fp),
                ]));

                let mut encoded = Vec::from([ResultCode::UserError as u8]);
                calldata::encode_to(&mut encoded, &val);

                encoded
            }
            Ok((RunOk::VMError(data, _), fp)) => {
                let mut encoded = Vec::from([ResultCode::VmError as u8]);

                let fp = calldata::to_value(fp)?;
                let val = calldata::Value::Map(BTreeMap::from([
                    ("message".to_owned(), data.as_str().into()),
                    ("fingerprint".to_owned(), fp),
                ]));

                calldata::encode_to(&mut encoded, &val);

                encoded
            }
            Err(e) => {
                let mut encoded = Vec::from([ResultCode::VmError as u8]);
                encoded.extend_from_slice(format!("{e:?}").as_bytes());

                encoded
            }
        };

        sock.write_all(&[host_fns::Methods::ConsumeResult as u8])?;
        write_slice(sock, &data)?;

        log_debug!("wrote consumed result to host");

        let mut int_buf = [0; 1];
        sock.read_exact(&mut int_buf)?;

        log_debug!("consume_result: ACK");

        Ok(())
    }

    pub fn get_leader_result(&mut self, call_no: u32) -> Result<Option<vm::RunOk>> {
        log_trace!("get_leader_result");

        let Ok(mut sock) = (*self.sock).lock() else {
            anyhow::bail!("can't take lock")
        };
        let sock: &mut dyn Sock = &mut *sock;
        sock.write_all(&[host_fns::Methods::GetLeaderNondetResult as u8])?;
        sock.write_all(&call_no.to_le_bytes())?;

        match read_host_error(sock)? {
            host_fns::Errors::Ok => {}
            host_fns::Errors::IAmLeader => {
                return Ok(None);
            }
            e => return Err(crate::errors::VMError(e.str_snake_case().to_owned(), None).into()),
        }

        let leaders_result = read_bytes(sock)?;

        let rest = &leaders_result[1..];

        let res = match leaders_result[0] {
            x if x == ResultCode::Return as u8 => vm::RunOk::Return(rest.into()),
            x if x == ResultCode::UserError as u8 => {
                vm::RunOk::UserError(String::from(str::from_utf8(rest)?))
            }
            x if x == ResultCode::VmError as u8 => {
                vm::RunOk::VMError(String::from(str::from_utf8(rest)?), None)
            }
            x => anyhow::bail!("host returned incorrect result id {}", x),
        };
        Ok(Some(res))
    }

    pub fn post_nondet_result(&mut self, call_no: u32, res: &vm::RunOk) -> Result<()> {
        log_trace!(call_no = call_no; "post_nondet_result");

        let Ok(mut sock) = (*self.sock).lock() else {
            anyhow::bail!("can't take lock")
        };
        let sock: &mut dyn Sock = &mut *sock;
        sock.write_all(&[host_fns::Methods::PostNondetResult as u8])?;
        sock.write_all(&call_no.to_le_bytes())?;

        write_slice(sock, &Vec::from_iter(res.as_bytes_iter()))?;

        sock.flush()?;

        handle_host_error(sock)?;

        Ok(())
    }

    pub fn post_message(
        &mut self,
        account: &calldata::Address,
        calldata: &[u8],
        data: &str,
    ) -> Result<()> {
        log_trace!("post_message");

        let Ok(mut sock) = (*self.sock).lock() else {
            anyhow::bail!("can't take lock")
        };
        let sock: &mut dyn Sock = &mut *sock;
        sock.write_all(&[host_fns::Methods::PostMessage as u8])?;
        sock.write_all(&account.raw())?;

        write_slice(sock, calldata)?;
        write_slice(sock, data.as_bytes())?;

        sock.flush()?;

        handle_host_error(sock)?;

        Ok(())
    }

    pub fn deploy_contract(&mut self, calldata: &[u8], code: &[u8], data: &str) -> Result<()> {
        log_trace!("deploy_contract");

        let Ok(mut sock) = (*self.sock).lock() else {
            anyhow::bail!("can't take lock")
        };
        let sock: &mut dyn Sock = &mut *sock;
        sock.write_all(&[host_fns::Methods::DeployContract as u8])?;

        write_slice(sock, calldata)?;
        write_slice(sock, code)?;
        write_slice(sock, data.as_bytes())?;

        sock.flush()?;

        handle_host_error(sock)?;

        Ok(())
    }

    pub fn consume_fuel(&mut self, gas: u64) -> Result<()> {
        log_trace!("consume_fuel");

        let Ok(mut sock) = (*self.sock).lock() else {
            anyhow::bail!("can't take lock")
        };
        let sock: &mut dyn Sock = &mut *sock;
        sock.write_all(&[host_fns::Methods::ConsumeFuel as u8])?;
        sock.write_all(&gas.to_le_bytes())?;

        sock.flush()?;
        Ok(())
    }

    pub fn eth_call(&mut self, address: calldata::Address, calldata: &[u8]) -> Result<Box<[u8]>> {
        log_trace!("eth_call");

        let Ok(mut sock) = (*self.sock).lock() else {
            anyhow::bail!("can't take lock")
        };
        let sock: &mut dyn Sock = &mut *sock;
        sock.write_all(&[host_fns::Methods::EthCall as u8])?;

        sock.write_all(&address.raw())?;

        sock.write_all(&(calldata.len() as u32).to_le_bytes())?;
        sock.write_all(calldata)?;

        handle_host_error(sock)?;

        read_bytes(sock)
    }

    pub fn eth_send(
        &mut self,
        address: calldata::Address,
        calldata: &[u8],
        data: &str,
    ) -> Result<()> {
        log_trace!("eth_send");

        let Ok(mut sock) = (*self.sock).lock() else {
            anyhow::bail!("can't take lock")
        };
        let sock: &mut dyn Sock = &mut *sock;
        sock.write_all(&[host_fns::Methods::EthSend as u8])?;

        sock.write_all(&address.raw())?;

        sock.write_all(&(calldata.len() as u32).to_le_bytes())?;
        sock.write_all(calldata)?;

        sock.write_all(&(data.len() as u32).to_le_bytes())?;
        sock.write_all(data.as_bytes())?;

        sock.flush()?;

        handle_host_error(sock)?;

        Ok(())
    }

    pub fn get_balance(&mut self, address: calldata::Address) -> Result<primitive_types::U256> {
        log_trace!("get_balance");

        let Ok(mut sock) = (*self.sock).lock() else {
            anyhow::bail!("can't take lock")
        };
        let sock: &mut dyn Sock = &mut *sock;
        sock.write_all(&[host_fns::Methods::GetBalance as u8])?;

        sock.write_all(&address.raw())?;

        handle_host_error(sock)?;

        let mut buf: [u8; 32] = [0; 32];
        sock.read_exact(&mut buf)?;
        Ok(primitive_types::U256::from_little_endian(&buf))
    }

    pub fn remaining_fuel_as_gen(&mut self) -> Result<u64> {
        log_trace!("remaining_fuel_as_gen");

        let Ok(mut sock) = (*self.sock).lock() else {
            anyhow::bail!("can't take lock")
        };
        let sock: &mut dyn Sock = &mut *sock;
        sock.write_all(&[host_fns::Methods::RemainingFuelAsGen as u8])?;

        handle_host_error(sock)?;

        let mut buf: [u8; 8] = [0; 8];
        sock.read_exact(&mut buf)?;
        Ok(u64::from_le_bytes(buf))
    }
}
