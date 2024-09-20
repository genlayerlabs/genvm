use std::{mem::swap, sync::Arc};

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
