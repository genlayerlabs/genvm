pub use genvm_common::host_fns;
pub mod message;

use genlayer_sdk::abi;
use genvm_common::*;

use crate::public_abi;
use crate::public_abi::{ResultCode, StorageType};
use genvm_common::calldata::Address;
use genvm_common::calldata::ADDRESS_SIZE;
use message::root_offsets;

use core::str;
use std::os::fd::FromRawFd;

use anyhow::{Context, Result};

use crate::{calldata, rt};
pub use message::SlotID;

pub trait Sock: std::io::Read + std::io::Write + Send + Sync {}

impl Sock for bufreaderwriter::seq::BufReaderWriterSeq<std::os::unix::net::UnixStream> {}

impl Sock for bufreaderwriter::seq::BufReaderWriterSeq<std::net::TcpStream> {}

pub struct Host {
    sock: Box<dyn Sock>,
    metrics: sync::DArc<Metrics>,
}

#[derive(Default, serde::Serialize, Debug, genlayer_calldata::Encode)]
pub struct Metrics {
    pub time: stats::metric::Time,
}

impl Host {
    pub fn new(sock: Box<dyn Sock>, metrics: sync::DArc<Metrics>) -> Host {
        Self { sock, metrics }
    }
    pub fn connect(addr: &str, metrics: sync::DArc<Metrics>) -> Result<Host> {
        const UNIX: &str = "unix://";
        let sock: Box<dyn Sock> = if let Some(addr_suff) = addr.strip_prefix(UNIX) {
            Box::new(bufreaderwriter::seq::BufReaderWriterSeq::new_writer(
                std::os::unix::net::UnixStream::connect(std::path::Path::new(addr_suff))
                    .with_context(|| format!("connecting to {addr}"))?,
            ))
        } else if let Some(fd_str) = addr.strip_prefix("fd://") {
            let fd: i32 = fd_str
                .parse()
                .with_context(|| format!("parsing fd number from '{fd_str}'"))?;
            let stream = unsafe { std::os::unix::net::UnixStream::from_raw_fd(fd) };
            Box::new(bufreaderwriter::seq::BufReaderWriterSeq::new_writer(stream))
        } else {
            Box::new(bufreaderwriter::seq::BufReaderWriterSeq::new_writer(
                std::net::TcpStream::connect(addr)
                    .with_context(|| format!("connecting to {addr}"))?,
            ))
        };
        Ok(Host { sock, metrics })
    }
}

fn read_u32(sock: &mut dyn Sock, context: &str) -> Result<u32> {
    let mut int_buf = [0; 4];
    sock.read_exact(&mut int_buf)
        .with_context(|| format!("reading u32 from host: {context}"))?;
    Ok(u32::from_le_bytes(int_buf))
}

fn read_bytes(sock: &mut dyn Sock, context: &str) -> Result<Box<[u8]>> {
    let len = read_u32(sock, context)?;

    let res = Box::new_uninit_slice(len as usize);
    let mut res = unsafe { res.assume_init() };
    sock.read_exact(&mut res)
        .with_context(|| format!("reading {} bytes from host: {context}", len))?;
    Ok(res)
}

fn write_slice(sock: &mut dyn Sock, data: &[u8]) -> Result<()> {
    let len = data.len() as u32;

    sock.write_all(&len.to_le_bytes())?;
    sock.write_all(data)?;

    Ok(())
}

fn read_host_error(sock: &mut dyn Sock, context: &str) -> Result<host_fns::Errors> {
    let mut has_some = [0; 1];
    sock.read_exact(&mut has_some)
        .with_context(|| format!("reading host error code: {context}"))?;

    host_fns::Errors::try_from(has_some[0])
        .map_err(|_| anyhow::anyhow!("invalid host error code {} for: {}", has_some[0], context))
}

fn handle_host_error(sock: &mut dyn Sock, context: &str) -> Result<()> {
    let e = read_host_error(sock, context)?;

    if e == host_fns::Errors::Ok {
        Ok(())
    } else {
        Err(rt::errors::VMError(
            abi::consts::VmError::host().val_str(e.str_snake_case()),
            None,
        )
        .into())
    }
}

pub fn encode_result(res: &Result<FullResult>) -> Result<Vec<u8>> {
    match res {
        Ok(d) => {
            let mut encoded = Vec::from([d.kind as u8]);
            let as_value = calldata::to_value(d);
            calldata::encode_to(&mut calldata::Encoder::new(&mut encoded), &as_value)?;
            Ok(encoded)
        }
        Err(e) => {
            let mut encoded = Vec::from([ResultCode::InternalError as u8]);
            let fake_res = FullResult::new_internal_error(format!("{e:?}"));
            let as_value = calldata::to_value(&fake_res);
            calldata::encode_to(&mut calldata::Encoder::new(&mut encoded), &as_value)?;
            Ok(encoded)
        }
    }
}

pub fn write_result_to_sock(sock: &mut dyn Sock, res: &Result<FullResult>) -> Result<()> {
    let data = encode_result(res)?;
    write_slice(sock, &data)?;
    sock.flush()?;
    Ok(())
}

pub struct LockedSlotsSet(Box<[SlotID]>);

impl LockedSlotsSet {
    pub fn contains(&self, slot: SlotID) -> bool {
        self.0.binary_search(&slot).is_ok()
    }
}

pub fn all_useful_work_done() {
    std::process::exit(0);
}

#[derive(Debug, Clone, serde::Serialize, genlayer_calldata::Encode)]
pub struct FullResult {
    pub execution_hash: bytes::Bytes,

    pub kind: public_abi::ResultCode,
    pub data: calldata::Value,
    pub fingerprint: Option<rt::errors::Fingerprint>,
    pub storage_changes: Vec<rt::vm::storage::Delta>,

    pub emissions: Vec<genlayer_sdk::abi::ExecutionEmission>,

    pub nondet_disagreement: Option<u32>,
    pub nondet_results: Vec<bytes::Bytes>,

    pub data_fees_remaining: Vec<primitive_types::U256>,
}

impl FullResult {
    pub fn new_internal_error(msg: String) -> Self {
        Self {
            execution_hash: bytes::Bytes::new(),
            kind: public_abi::ResultCode::InternalError,
            data: calldata::Value::Str(msg),
            fingerprint: None,
            storage_changes: Vec::new(),
            emissions: Vec::new(),
            nondet_disagreement: None,
            nondet_results: Vec::new(),
            data_fees_remaining: Vec::new(),
        }
    }
}

struct Sha3Writer(sha3::Sha3_256);

impl calldata::Writer for Sha3Writer {
    type Error = std::convert::Infallible;

    fn write_all(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        sha3::Digest::update(&mut self.0, data);
        Ok(())
    }
}

impl FullResult {
    pub fn new(
        rt_result: rt::vm::FullResult,
        nondet_results: Vec<bytes::Bytes>,
        nondet_disagreement: Option<u32>,
        data_fees_remaining: Vec<primitive_types::U256>,
    ) -> Self {
        #[derive(serde::Serialize)]
        struct Hashable<'a> {
            kind: &'a public_abi::ResultCode,
            data: &'a calldata::Value,
            fingerprint: &'a Option<rt::errors::Fingerprint>,
            storage_changes: &'a Vec<rt::vm::storage::Delta>,
            data_fees_remaining: &'a Vec<primitive_types::U256>,
        }

        impl<W: calldata::Writer> calldata::codec::Encode<W> for Hashable<'_> {
            type Error = W::Error;

            fn encode(&self, enc: &mut calldata::Encoder<W>) -> Result<(), Self::Error> {
                enc.start_map(5)?;

                enc.push_map_k("data")?;
                calldata::codec::Encode::encode(self.data, enc)?;

                enc.push_map_k("data_fees_remaining")?;
                calldata::codec::Encode::encode(self.data_fees_remaining, enc)?;

                enc.push_map_k("fingerprint")?;
                calldata::codec::Encode::encode(self.fingerprint, enc)?;

                enc.push_map_k("kind")?;
                calldata::codec::Encode::encode(self.kind, enc)?;

                enc.push_map_k("storage_changes")?;
                calldata::codec::Encode::encode(self.storage_changes, enc)?;

                Ok(())
            }
        }

        let hashable = Hashable {
            kind: &rt_result.kind,
            data: &rt_result.data,
            fingerprint: &rt_result.fingerprint,
            storage_changes: &rt_result.storage_changes,
            data_fees_remaining: &data_fees_remaining,
        };

        let as_value = calldata::to_value(&hashable);
        let mut enc = calldata::Encoder::new(Sha3Writer(sha3::Digest::new()));
        match calldata::encode_to(&mut enc, &as_value) {
            Ok(()) => {}
            Err(e) => match e {},
        }
        let execution_hash =
            bytes::Bytes::from(sha3::Digest::finalize(enc.into_inner().0).to_vec());

        Self {
            execution_hash,

            data: rt_result.data,
            fingerprint: rt_result.fingerprint,
            kind: rt_result.kind,
            storage_changes: rt_result.storage_changes,
            emissions: rt_result.emissions,
            nondet_results,
            nondet_disagreement,
            data_fees_remaining,
        }
    }
}

impl Host {
    fn lock_sock(&mut self) -> sync::Lock<&mut dyn Sock, stats::tracker::Time> {
        sync::Lock::new(
            &mut *self.sock,
            stats::tracker::Time::new(self.metrics.gep(|x| &x.time)),
        )
    }

    fn get_locked_slots(
        &mut self,
        contract_address: calldata::Address,
        limiter: &rt::memlimiter::Limiter,
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

        if len > abi::consts::top_limits::LOCKED_SLOTS {
            return Err(
                rt::errors::VMError(abi::consts::VmError::oom().ram().limit(), None).into(),
            );
        }

        if !limiter.consume_mul(len, SlotID::SIZE) {
            return Err(rt::errors::VMError(abi::consts::VmError::oom().ram().val(), None).into());
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
        limiter: &rt::memlimiter::Limiter,
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

        if len > abi::consts::top_limits::UPGRADERS {
            return Err(
                rt::errors::VMError(abi::consts::VmError::oom().ram().limit(), None).into(),
            );
        }

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

    pub fn storage_read(
        &mut self,
        mode: StorageType,
        account: calldata::Address,
        slot: SlotID,
        index: u32,
        buf: &mut [u8],
    ) -> Result<()> {
        let mut sock = self.lock_sock();

        sock.write_all(&[host_fns::Methods::StorageRead as u8])?;
        sock.write_all(&[mode as u8; 1])?;
        sock.write_all(&account.raw())?;
        sock.write_all(&slot.raw())?;
        sock.write_all(&index.to_le_bytes())?;
        sock.write_all(&(buf.len() as u32).to_le_bytes())?;

        handle_host_error(&mut **sock, "storage_read")?;

        sock.read_exact(buf)
            .with_context(|| format!("reading {} bytes from storage slot {:?}", buf.len(), slot))?;

        log_trace!(slot:bytes = slot.0, index = index, data:bytes = buf; "read");

        Ok(())
    }

    pub fn consume_result(&mut self, res: &Result<FullResult>) -> Result<()> {
        log_trace!("consume_result");

        let data = encode_result(res)?;

        let mut sock = self.lock_sock();

        sock.write_all(&[host_fns::Methods::ConsumeResult as u8])?;
        write_slice(&mut **sock, &data)?;

        log_debug!("wrote consumed result to host");

        let mut int_buf = [0; 1];
        sock.read_exact(&mut int_buf)?;

        log_debug!("consume_result: ACK");

        Ok(())
    }

    pub fn notify_finished(&mut self) -> Result<()> {
        log_trace!("notify_finished");

        let mut sock = self.lock_sock();
        sock.write_all(&[host_fns::Methods::NotifyFinished as u8])?;
        sock.flush()?;

        let mut int_buf = [0; 1];
        sock.read_exact(&mut int_buf)?;

        log_debug!("notify_finished: ACK");

        Ok(())
    }

    pub fn consume_fuel(&mut self, gas: primitive_types::U256) -> Result<()> {
        log_trace!("consume_fuel");

        let mut sock = self.lock_sock();
        sock.write_all(&[host_fns::Methods::ConsumeFuel as u8])?;
        let buf = gas.to_little_endian();
        sock.write_all(&buf)?;

        sock.flush()?;
        Ok(())
    }

    pub fn eth_call(&mut self, address: calldata::Address, calldata: &[u8]) -> Result<Box<[u8]>> {
        log_trace!("eth_call");

        let mut sock = self.lock_sock();
        sock.write_all(&[host_fns::Methods::EthCall as u8])?;

        sock.write_all(&address.raw())?;

        sock.write_all(&(calldata.len() as u32).to_le_bytes())?;
        sock.write_all(calldata)?;

        handle_host_error(&mut **sock, "eth_call")?;

        read_bytes(&mut **sock, "eth_call result")
    }

    pub fn get_balance(&mut self, address: calldata::Address) -> Result<primitive_types::U256> {
        log_trace!("get_balance");

        let mut sock = self.lock_sock();
        sock.write_all(&[host_fns::Methods::GetBalance as u8])?;

        sock.write_all(&address.raw())?;

        handle_host_error(&mut **sock, "get_balance")?;

        let mut buf: [u8; 32] = [0; 32];
        sock.read_exact(&mut buf)
            .with_context(|| format!("reading balance for address {:?}", address))?;
        Ok(primitive_types::U256::from_little_endian(&buf))
    }

    pub fn remaining_fuel_as_gen(&mut self) -> Result<primitive_types::U256> {
        log_trace!("remaining_fuel_as_gen");

        let mut sock = self.lock_sock();
        sock.write_all(&[host_fns::Methods::RemainingFuelAsGen as u8])?;

        handle_host_error(&mut **sock, "remaining_fuel_as_gen")?;

        let mut buf: [u8; 32] = [0; 32];
        sock.read_exact(&mut buf)
            .with_context(|| "reading remaining fuel")?;
        Ok(primitive_types::U256::from_little_endian(&buf))
    }

    pub fn notify_nondet_disagreement(&mut self, call_no: u32) -> Result<()> {
        log_trace!(call_no = call_no; "notify_nondet_disagreement");

        let mut sock = self.lock_sock();
        sock.write_all(&[host_fns::Methods::NotifyNondetDisagreement as u8])?;
        sock.write_all(&call_no.to_le_bytes())?;

        sock.flush()?;

        Ok(())
    }
}

pub struct MultiHost {
    hosts: Vec<tokio::sync::Mutex<Host>>,
    method_hosts: Vec<u8>,
}

impl MultiHost {
    pub fn new(hosts: Vec<Host>, method_hosts: Vec<u8>) -> Self {
        Self {
            hosts: hosts.into_iter().map(tokio::sync::Mutex::new).collect(),
            method_hosts,
        }
    }

    pub async fn lock_for(&self, method: host_fns::Methods) -> tokio::sync::MutexGuard<'_, Host> {
        let idx = if (method as usize) < self.method_hosts.len() {
            self.method_hosts[method as usize] as usize
        } else {
            0
        };
        self.hosts[idx].lock().await
    }
}
