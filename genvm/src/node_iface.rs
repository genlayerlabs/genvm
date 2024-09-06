use std::sync::Arc;

use anyhow::Result;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_with::{base64::Base64, serde_as};

pub struct StorageSlot {
    pub account: Address,
    pub slot: Address,
}

#[serde_as]
#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Hash, Copy)]
pub struct Address(#[serde_as(as = "Base64")] pub [u8; 32]);

impl Address {
    pub fn raw(&self) -> [u8; 32] {
        let Address(r) = self;
        *r
    }

    pub fn new() -> Self {
        Self([0; 32])
    }

    pub const fn len() -> usize {
        return 32;
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct Gas(pub u64);

impl Gas {
    pub fn raw(&self) -> u64 {
        self.0
    }

    pub fn decrement_by(&mut self, by: u64) {
        if by >= self.0 {
            self.0 = 0
        } else {
            self.0 -= by
        }
    }
}

#[derive(Clone, Debug)]
pub struct DecodeUtf8<I: Iterator<Item = u8>>(std::iter::Peekable<I>);

pub fn decode_utf8<I: IntoIterator<Item = u8>>(i: I) -> DecodeUtf8<I::IntoIter> {
    DecodeUtf8(i.into_iter().peekable())
}

#[derive(PartialEq, Debug)]
pub struct InvalidSequence(pub Vec<u8>);

impl<I: Iterator<Item = u8>> Iterator for DecodeUtf8<I> {
    type Item = Result<char, InvalidSequence>;
    #[inline]
    fn next(&mut self) -> Option<Result<char, InvalidSequence>> {
        let mut on_err: Vec<u8> = Vec::new();
        self.0.next().map(|b| {
            on_err.push(b);
            if b & 0x80 == 0 {
                Ok(b as char)
            } else {
                let l = (!b).leading_zeros() as usize; // number of bytes in UTF-8 representation
                if l < 2 || l > 6 {
                    return Err(InvalidSequence(on_err));
                };
                let mut x = (b as u32) & (0x7F >> l);
                for _ in 0..l - 1 {
                    match self.0.peek() {
                        Some(&b) if b & 0xC0 == 0x80 => {
                            on_err.push(b);
                            self.0.next();
                            x = (x << 6) | (b as u32) & 0x3F;
                        }
                        _ => return Err(InvalidSequence(on_err)),
                    }
                }
                match char::from_u32(x) {
                    Some(x) if l == x.len_utf8() => Ok(x),
                    _ => Err(InvalidSequence(on_err)),
                }
            }
        })
    }
}

pub enum VMRunResult {
    Return(Vec<u8>),
    Rollback(String),
    /// TODO: should there be an error or should it be merged with rollback?
    Error(String),
}

impl std::fmt::Debug for VMRunResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Return(r) => {
                let str = decode_utf8(r.iter().cloned())
                    .map(|r| match r {
                        Ok('\\') => "\\\\".into(),
                        Ok(c) if c.is_control() || c == '\n' || c == '\x07' => {
                            if c as u32 <= 255 {
                                format!("\\x{:02x}", c as u32)
                            } else {
                                format!("\\u{:04x}", c as u32)
                            }
                        }
                        Ok(c) => c.to_string(),
                        Err(InvalidSequence(seq)) => {
                            seq.iter().map(|c| format!("\\{:02x}", *c as u32)).join("")
                        }
                    })
                    .join("");
                f.write_fmt(format_args!("Return(\"{}\")", str))
            }
            Self::Rollback(r) => f.debug_tuple("Rollback").field(r).finish(),
            Self::Error(r) => f.debug_tuple("Error").field(r).finish(),
        }
    }
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
    pub lang: String,
}

pub trait InitApi {
    fn get_initial_data(&mut self) -> Result<MessageData>;

    fn get_calldata(&mut self) -> Result<Vec<u8>>;

    fn get_code(&mut self, account: &Address) -> Result<Arc<Vec<u8>>>;
}

#[derive(Serialize, Deserialize, Clone)]
pub enum InitAction {
    MapFile {
        to: String,
        contents: Arc<Vec<u8>>,
    },
    MapCode {
        to: String,
    },
    AddEnv {
        name: String,
        val: String,
    },
    SetArgs {
        args: Vec<String>,
    },
    LinkWasm {
        contents: Arc<Vec<u8>>,
        debug_path: Option<String>,
    },
    StartWasm {
        contents: Arc<Vec<u8>>,
        debug_path: Option<String>,
    },
}

pub trait RunnerApi {
    fn get_runner(&mut self, desc: RunnerDescription) -> Result<Vec<InitAction>>;
}

#[allow(dead_code)]
pub trait StorageApi {
    fn storage_read(
        &mut self,
        remaing_gas: &mut Gas,
        slot: StorageSlot,
        index: u32,
        buf: &mut [u8],
    ) -> Result<()>;

    fn storage_write(
        &mut self,
        remaing_gas: &mut Gas,
        slot: StorageSlot,
        index: u32,
        buf: &[u8],
    ) -> Result<()>;
}

#[allow(dead_code)]
pub trait NondetSupportApi {
    fn get_leader_result(&mut self, call_no: u32) -> Result<Option<VMRunResult>>;
}

#[allow(dead_code)]
pub trait ContractsApi {
    fn post_message(&mut self, remaing_gas: &mut Gas, data: MessageData, when: ()) -> Result<()>;
}
