use std::{collections::BTreeMap, mem::swap};

use wiggle::{GuestError, GuestMemory, GuestPtr};

use crate::ustar::SharedBytes;

pub struct FileContents {
    pub contents: SharedBytes,
    pub pos: usize,
}

pub struct FileContentsUnevaluated {
    pub task: Option<tokio::task::JoinHandle<anyhow::Result<Box<[u8]>>>>,
    pub cell: tokio::sync::OnceCell<anyhow::Result<FileContents>>,
}

impl FileContentsUnevaluated {
    pub fn from_task(task: tokio::task::JoinHandle<anyhow::Result<Box<[u8]>>>) -> Self {
        Self {
            task: Some(task),
            cell: tokio::sync::OnceCell::new(),
        }
    }

    pub async fn get(&mut self) -> anyhow::Result<&mut FileContents> {
        let task = &mut self.task;
        self.cell
            .get_or_init(|| async {
                let task = match task {
                    Some(task) => task,
                    None => unreachable!(),
                };
                match task.await {
                    Ok(Ok(v)) => Ok(FileContents {
                        contents: SharedBytes::new(v),
                        pos: 0,
                    }),
                    Ok(Err(v)) => Err(v),
                    Err(v) => Err(anyhow::Error::new(v)),
                }
            })
            .await;

        match self.cell.get_mut().unwrap() {
            Ok(r) => Ok(r),
            Err(e) => {
                let mut err = anyhow::anyhow!("<already consumed>");
                swap(e, &mut err);
                Err(err)
            }
        }
    }

    pub fn from_contents(contents: SharedBytes, pos: usize) -> Self {
        Self {
            cell: tokio::sync::OnceCell::new_with(Some(Ok(FileContents { contents, pos }))),
            task: None,
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

pub fn read_string(memory: &GuestMemory<'_>, ptr: GuestPtr<str>) -> Result<String, GuestError> {
    Ok(memory.as_cow_str(ptr)?.into_owned())
}

pub(super) struct VFS {
    pub fds: BTreeMap<u32, FileDescriptor>,
    pub free_descriptors: Vec<u32>,
    pub next_free_descriptor: u32,
}

impl VFS {
    pub fn new(stdin: Vec<u8>) -> Self {
        let stdin_data = SharedBytes::new(stdin);

        let fds = BTreeMap::from([
            (
                0,
                FileDescriptor::File(FileContentsUnevaluated::from_contents(stdin_data, 0)),
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
