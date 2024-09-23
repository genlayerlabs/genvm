use std::{collections::BTreeMap, mem::swap, sync::Arc};

use wiggle::{GuestError, GuestMemory, GuestPtr};

pub struct FileContents {
    pub contents: Arc<[u8]>,
    pub pos: usize,
}

pub type FileEvalError = Option<anyhow::Error>;

pub struct FileContentsUnevaluated {
    data: Result<FileContents, std::sync::OnceLock<Result<Arc<[u8]>, FileEvalError>>>,
}

impl FileContentsUnevaluated {
    pub fn get(&mut self) -> Result<&mut FileContents, FileEvalError> {
        match &mut self.data {
            Ok(x) => return Ok(x),
            placed_data => {
                let mut old_data = Ok(FileContents {
                    contents: Arc::new([]),
                    pos: 0,
                });
                swap(placed_data, &mut old_data);
                match old_data {
                    // old data
                    Ok(_) => unreachable!(),
                    Err(fut) => {
                        fut.wait();
                        let val = fut.into_inner().unwrap();
                        match val {
                            Ok(val) => {
                                *placed_data = Ok(FileContents {
                                    contents: val,
                                    pos: 0,
                                });
                                match placed_data {
                                    Ok(x) => Ok(x),
                                    _ => unreachable!(),
                                }
                            }
                            Err(e) => {
                                // data is already nullified
                                Err(e)
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn from_contents(contents: Arc<[u8]>, pos: usize) -> Self {
        Self {
            data: Ok(FileContents { contents, pos }),
        }
    }
}

pub enum FileDescriptor {
    Stdin,
    Stdout,
    Stderr,
    File(FileContentsUnevaluated),
    Dir { path: Vec<String> },
}

pub fn read_string<'a>(
    memory: &'a GuestMemory<'_>,
    ptr: GuestPtr<str>,
) -> Result<String, GuestError> {
    Ok(memory.as_cow_str(ptr)?.into_owned())
}

pub(super) struct VFS {
    pub fds: BTreeMap<u32, FileDescriptor>,
    pub free_descriptors: Vec<u32>,
    pub next_free_descriptor: u32,
}

impl VFS {
    pub fn new() -> Self {
        let fds = BTreeMap::from([
            (0, FileDescriptor::Stdin),
            (1, FileDescriptor::Stdout),
            (2, FileDescriptor::Stderr),
            (3, FileDescriptor::Dir { path: Vec::new() }),
        ]);
        let next_free_descriptor = fds.last_key_value().map(|x| *x.0).unwrap_or(0);
        Self {
            fds,
            next_free_descriptor,
            free_descriptors: Vec::new(),
        }
    }

    /// gives vacant fd
    pub fn alloc_fd(&mut self) -> u32 {
        match self.free_descriptors.pop() {
            Some(v) => v,
            None => {
                self.next_free_descriptor += 1;
                self.next_free_descriptor
            }
        }
    }

    /// it must be removed from fds beforehand
    pub fn free_fd(&mut self, fd: u32) {
        self.free_descriptors.push(fd);
    }

    pub fn place_content(&mut self, value: FileContentsUnevaluated) -> u32 {
        let fd = self.alloc_fd();
        self.fds.insert(fd, FileDescriptor::File(value));
        fd
    }
}
