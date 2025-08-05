use std::collections::BTreeMap;

use crate::{public_abi, rt};
use genvm_common::*;

pub struct FileContents {
    pub contents: util::SharedBytes,
    pub pos: usize,

    pub release_memory: bool,
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
    pub fds: BTreeMap<u32, FileDescriptor>,
    pub free_descriptors: Vec<u32>,
    pub next_free_descriptor: u32,

    pub limiter: rt::memlimiter::Limiter,
}

impl VFS {
    pub fn new(stdin: Vec<u8>, limiter: rt::memlimiter::Limiter) -> Self {
        let stdin_data = util::SharedBytes::new(stdin);

        let fds = BTreeMap::from([
            (
                0,
                FileDescriptor::File(FileContents {
                    contents: stdin_data,
                    pos: 0,
                    release_memory: true,
                }),
            ),
            (1, FileDescriptor::Stdout),
            (2, FileDescriptor::Stderr),
            (3, FileDescriptor::Dir { path: Vec::new() }),
        ]);
        let next_free_descriptor = fds.last_key_value().map(|x| *x.0).unwrap_or(0);
        Self {
            fds,
            next_free_descriptor,
            free_descriptors: Vec::new(),
            limiter,
        }
    }

    /// gives vacant fd
    pub fn alloc_fd(&mut self) -> anyhow::Result<u32> {
        match self.free_descriptors.pop() {
            Some(v) => Ok(v),
            None => {
                if !self
                    .limiter
                    .consume(public_abi::MemoryLimiterConsts::FdAllocation as u32)
                {
                    return Err(rt::errors::VMError::oom(None).into());
                }
                self.next_free_descriptor += 1;
                Ok(self.next_free_descriptor)
            }
        }
    }

    /// it must be removed from fds beforehand
    pub fn free_fd(&mut self, fd: u32) {
        self.free_descriptors.push(fd);
    }

    pub fn pop_fd(&mut self, fd: u32) -> Option<FileDescriptor> {
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

    pub fn place_content(&mut self, value: FileContents) -> anyhow::Result<u32> {
        if value.release_memory && !self.limiter.consume(value.contents.len() as u32) {
            return Err(rt::errors::VMError::oom(None).into());
        }

        let fd = self.alloc_fd()?;
        self.fds.insert(fd, FileDescriptor::File(value));
        Ok(fd)
    }
}
