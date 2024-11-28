mod host_fns;
mod result_codes;

pub use result_codes::ResultCode;

use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_with::{base64::Base64, serde_as};

use crate::vm;

#[serde_as]
#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Hash, Copy)]
pub struct AccountAddress(#[serde_as(as = "Base64")] pub [u8; 20]);

impl AccountAddress {
    pub fn raw(&self) -> [u8; 20] {
        let AccountAddress(r) = self;
        *r
    }

    pub fn new() -> Self {
        Self([0; 20])
    }

    pub const fn len() -> usize {
        return 20;
    }
}

#[serde_as]
#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Hash, Copy)]
pub struct GenericAddress(#[serde_as(as = "Base64")] pub [u8; 32]);

impl GenericAddress {
    pub fn raw(&self) -> [u8; 32] {
        let GenericAddress(r) = self;
        *r
    }

    pub fn new() -> Self {
        Self([0; 32])
    }

    pub const fn len() -> usize {
        return 32;
    }
}

fn default_datetime() -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339("2024-11-26T06:42:42.424242Z")
        .unwrap()
        .to_utc()
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MessageData {
    pub contract_account: crate::AccountAddress,
    pub sender_account: crate::AccountAddress,
    pub value: Option<u64>,
    pub is_init: bool,
    #[serde(default = "default_datetime")]
    pub datetime: chrono::DateTime<chrono::Utc>,
}

trait Sock: std::io::Read + std::io::Write + Send + Sync {}

impl Sock for std::os::unix::net::UnixStream {}

impl Sock for std::net::TcpStream {}

pub struct Host {
    sock: Box<Mutex<dyn Sock>>,
}

impl Host {
    pub fn new(addr: &str) -> Result<Host> {
        const UNIX: &str = "unix://";
        let sock: Box<Mutex<dyn Sock>> = if addr.starts_with(UNIX) {
            Box::new(Mutex::new(
                std::os::unix::net::UnixStream::connect(std::path::Path::new(&addr[UNIX.len()..]))
                    .with_context(|| format!("connecting to {addr}"))?,
            ))
        } else {
            Box::new(Mutex::new(
                std::net::TcpStream::connect(addr)
                    .with_context(|| format!("connecting to {addr}"))?,
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

fn write_result(sock: &mut dyn Sock, res: Result<&vm::RunOk, &anyhow::Error>) -> Result<()> {
    let str: String;
    let data = match res {
        Ok(vm::RunOk::Return(r)) => {
            sock.write_all(&[ResultCode::Return as u8])?;
            &r
        }
        Ok(vm::RunOk::Rollback(r)) => {
            sock.write_all(&[ResultCode::Rollback as u8])?;
            r.as_bytes()
        }
        Ok(vm::RunOk::ContractError(r)) => {
            sock.write_all(&[ResultCode::ContractError as u8])?;
            r.as_bytes()
        }
        Err(e) => {
            sock.write_all(&[ResultCode::Error as u8])?;
            str = format!("{}", e);
            str.as_bytes()
        }
    };
    sock.write_all(&(data.len() as u32).to_le_bytes())?;
    sock.write_all(data)?;
    Ok(())
}

impl Host {
    pub fn append_calldata(&mut self, calldata: &mut Vec<u8>) -> Result<()> {
        let Ok(mut sock) = (*self.sock).lock() else {
            anyhow::bail!("can't take lock")
        };
        let sock: &mut dyn Sock = &mut *sock;
        sock.write_all(&[host_fns::Methods::AppendCalldata as u8])?;
        let len = read_u32(sock)? as usize;
        calldata.reserve(len);
        let index = calldata.len();
        unsafe {
            calldata.set_len(index + len);
        }
        sock.read_exact(&mut calldata[index..index + len])?;
        Ok(())
    }

    pub fn get_code(&mut self, account: &AccountAddress) -> Result<Arc<[u8]>> {
        let Ok(mut sock) = (*self.sock).lock() else {
            anyhow::bail!("can't take lock")
        };
        let sock: &mut dyn Sock = &mut *sock;
        sock.write_all(&[host_fns::Methods::GetCode as u8])?;
        sock.write_all(&account.raw())?;

        let len = read_u32(sock)? as usize;
        let mut res = Vec::with_capacity(len);
        unsafe { res.set_len(len) };
        sock.read_exact(&mut res)?;
        Ok(Arc::from(res))
    }

    pub fn storage_read(
        &mut self,
        account: AccountAddress,
        slot: GenericAddress,
        index: u32,
        buf: &mut [u8],
    ) -> Result<()> {
        let Ok(mut sock) = (*self.sock).lock() else {
            anyhow::bail!("can't take lock")
        };
        let sock: &mut dyn Sock = &mut *sock;
        sock.write_all(&[host_fns::Methods::StorageRead as u8])?;
        sock.write_all(&account.raw())?;
        sock.write_all(&slot.raw())?;
        sock.write_all(&index.to_le_bytes())?;
        sock.write_all(&(buf.len() as u32).to_le_bytes())?;

        sock.read_exact(buf)?;
        Ok(())
    }

    pub fn storage_write(
        &mut self,
        account: AccountAddress,
        slot: GenericAddress,
        index: u32,
        buf: &[u8],
    ) -> Result<()> {
        let Ok(mut sock) = (*self.sock).lock() else {
            anyhow::bail!("can't take lock")
        };
        let sock: &mut dyn Sock = &mut *sock;
        sock.write_all(&[host_fns::Methods::StorageWrite as u8])?;
        sock.write_all(&account.raw())?;
        sock.write_all(&slot.raw())?;
        sock.write_all(&index.to_le_bytes())?;
        sock.write_all(&(buf.len() as u32).to_le_bytes())?;
        sock.write_all(buf)?;

        Ok(())
    }

    pub fn consume_result(&mut self, res: &vm::RunResult) -> Result<()> {
        let Ok(mut sock) = (*self.sock).lock() else {
            anyhow::bail!("can't take lock")
        };
        let sock: &mut dyn Sock = &mut *sock;
        sock.write_all(&[host_fns::Methods::ConsumeResult as u8])?;
        let res = match res {
            Ok(res) => Ok(res),
            Err(e) => Err(e),
        };
        write_result(sock, res)?;

        let mut int_buf = [0; 1];
        sock.read_exact(&mut int_buf)?;

        Ok(())
    }

    pub fn get_leader_result(&mut self, call_no: u32) -> Result<Option<vm::RunOk>> {
        let Ok(mut sock) = (*self.sock).lock() else {
            anyhow::bail!("can't take lock")
        };
        let sock: &mut dyn Sock = &mut *sock;
        sock.write_all(&[host_fns::Methods::GetLeaderNondetResult as u8])?;
        sock.write_all(&call_no.to_le_bytes())?;
        let mut has_some = [0; 1];
        sock.read_exact(&mut has_some)?;
        if has_some[0] == ResultCode::None as u8 {
            return Ok(None);
        }
        let len = read_u32(sock)?;
        let mut buf = Vec::with_capacity(len as usize);
        unsafe {
            buf.set_len(len as usize);
        }
        sock.read_exact(&mut buf)?;
        let res = match has_some[0] {
            x if x == ResultCode::Return as u8 => vm::RunOk::Return(buf),
            x if x == ResultCode::Rollback as u8 => vm::RunOk::Rollback(String::from_utf8(buf)?),
            x if x == ResultCode::ContractError as u8 => {
                vm::RunOk::ContractError(String::from_utf8(buf)?)
            }
            x => anyhow::bail!("host returned incorrect result id {}", x),
        };
        Ok(Some(res))
    }

    pub fn post_result(&mut self, call_no: u32, res: &vm::RunOk) -> Result<()> {
        let Ok(mut sock) = (*self.sock).lock() else {
            anyhow::bail!("can't take lock")
        };
        let sock: &mut dyn Sock = &mut *sock;
        sock.write_all(&[host_fns::Methods::PostNondetResult as u8])?;
        sock.write_all(&call_no.to_le_bytes())?;
        write_result(sock, Ok(res))?;
        Ok(())
    }

    pub fn post_message(
        &mut self,
        account: &AccountAddress,
        gas: u64,
        calldata: &[u8],
        code: &[u8],
    ) -> Result<()> {
        let Ok(mut sock) = (*self.sock).lock() else {
            anyhow::bail!("can't take lock")
        };
        let sock: &mut dyn Sock = &mut *sock;
        sock.write_all(&[host_fns::Methods::PostMessage as u8])?;
        sock.write_all(&account.raw())?;
        sock.write_all(&gas.to_le_bytes())?;

        sock.write_all(&(calldata.len() as u32).to_le_bytes())?;
        sock.write_all(calldata)?;

        sock.write_all(&(code.len() as u32).to_le_bytes())?;
        sock.write_all(code)?;
        Ok(())
    }

    pub fn consume_fuel(&mut self, gas: u64) -> Result<()> {
        let Ok(mut sock) = (*self.sock).lock() else {
            anyhow::bail!("can't take lock")
        };
        let sock: &mut dyn Sock = &mut *sock;
        sock.write_all(&[host_fns::Methods::ConsumeFuel as u8])?;
        sock.write_all(&gas.to_le_bytes())?;
        Ok(())
    }
}
