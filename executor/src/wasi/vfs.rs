use std::collections::BTreeMap;

use genlayer_sdk::abi;
use genvm_common::log_trace;

use crate::{public_abi, rt};

pub struct FileContents {
    pub contents: bytes::Bytes,
    pub pos: u32,

    pub release_memory: bool,
}

impl From<bytes::Bytes> for FileContents {
    fn from(value: bytes::Bytes) -> Self {
        Self {
            contents: value,
            pos: 0,
            release_memory: true,
        }
    }
}

pub enum FileDescriptor {
    Stdin,
    Stdout,
    Stderr,
    File(FileContents),
    Dir { path: Vec<String> },
}

#[allow(dead_code)]
const _: FileDescriptor = FileDescriptor::Stdin;

pub(crate) struct VFS {
    pub fds: BTreeMap<Fd, FileDescriptor>,
    pub free_descriptors: Vec<Fd>,
    pub next_free_descriptor: Fd,

    pub limiter: rt::memlimiter::Limiter,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Fd(u32);

impl From<Fd> for u32 {
    fn from(value: Fd) -> Self {
        value.0
    }
}

impl Fd {
    pub fn new(fd: u32) -> Self {
        Self(fd)
    }

    pub fn as_u32(&self) -> u32 {
        self.0
    }
}

impl VFS {
    pub fn new(
        stdin: Vec<u8>,
        limiter: rt::memlimiter::Limiter,
    ) -> std::result::Result<Self, rt::errors::VMError> {
        log_trace!(stdin:bytes = stdin; "creating VFS");

        if !limiter.consume(stdin.len() as u32) {
            return Err(rt::errors::VMError(
                abi::consts::VmError::oom().ram().val(),
                None,
            ));
        }

        let stdin_data = bytes::Bytes::from(stdin);

        let fds = BTreeMap::from([
            (
                Fd::new(0),
                FileDescriptor::File(FileContents {
                    contents: stdin_data,
                    pos: 0,
                    release_memory: true,
                }),
            ),
            (Fd::new(1), FileDescriptor::Stdout),
            (Fd::new(2), FileDescriptor::Stderr),
            (Fd::new(3), FileDescriptor::Dir { path: Vec::new() }),
        ]);
        let next_free_descriptor = fds.last_key_value().map(|x| *x.0).unwrap_or(Fd::default());
        Ok(Self {
            fds,
            next_free_descriptor,
            free_descriptors: Vec::new(),
            limiter,
        })
    }

    /// gives vacant fd
    pub fn alloc_fd(&mut self) -> anyhow::Result<Fd> {
        if self.fds.len() >= public_abi::top_limits::MAX_FDS as usize {
            return Err(
                rt::errors::VMError(abi::consts::VmError::oom().ram().limit(), None).into(),
            );
        }
        match self.free_descriptors.pop() {
            Some(v) => Ok(v),
            None => {
                if !self
                    .limiter
                    .consume(public_abi::memory_limiter_consts::FD_ALLOCATION)
                {
                    return Err(
                        rt::errors::VMError(abi::consts::VmError::oom().ram().val(), None).into(),
                    );
                }
                self.next_free_descriptor.0 += 1;
                Ok(self.next_free_descriptor)
            }
        }
    }

    /// it must be removed from fds beforehand
    pub fn free_fd(&mut self, fd: Fd) {
        self.free_descriptors.push(fd);
    }

    pub fn pop_fd(&mut self, fd: Fd) -> Option<FileDescriptor> {
        match self.fds.remove(&fd) {
            Some(v) => {
                if let FileDescriptor::File(v) = &v {
                    if v.release_memory {
                        self.limiter.release(v.contents.len() as u32);
                    }
                }

                self.free_fd(fd);

                Some(v)
            }
            None => None,
        }
    }

    pub fn place_content(&mut self, value: FileContents) -> anyhow::Result<Fd> {
        if value.release_memory && !self.limiter.consume(value.contents.len() as u32) {
            return Err(rt::errors::VMError(abi::consts::VmError::oom().ram().val(), None).into());
        }

        let fd = self.alloc_fd()?;
        self.fds.insert(fd, FileDescriptor::File(value));
        Ok(fd)
    }
}
