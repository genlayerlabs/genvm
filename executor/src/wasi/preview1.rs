use anyhow::Context as _;
use std::{borrow::BorrowMut, io::Write, iter};
use tracing::instrument;
use wiggle::{GuestError, GuestMemory, GuestPtr};

use crate::ustar::SharedBytes;
use crate::wasi::base;

use super::common::*;
use std::collections::BTreeMap;

pub struct Context {
    args_buf: Vec<u8>,
    args_offsets: Vec<u32>,
    env_buf: Vec<u8>,
    env_offsets: Vec<u32>,

    fs: Box<FilesTrie>,
    unix_timestamp: u64,

    conf: base::Config,
}

pub struct ContextVFS<'a> {
    pub(super) vfs: &'a mut VFS,
    pub(super) context: &'a mut Context,
}

/// An error returned from the `proc_exit` host syscall.
///
/// Embedders can test if an error returned from wasm is this error, in which
/// case it may signal a non-fatal trap.
#[derive(Debug)]
pub struct I32Exit(pub i32);

impl I32Exit {
    /// Accessor for an exit code appropriate for calling `std::process::exit` with,
    /// when interpreting this `I32Exit` as an exit for the parent process.
    ///
    /// This method masks off exit codes which are illegal on Windows.
    pub fn process_exit_code(&self) -> i32 {
        if cfg!(windows) && self.0 >= 3 {
            1
        } else {
            self.0
        }
    }
}

impl std::fmt::Display for I32Exit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Exited with i32 exit status {}", self.0)
    }
}

impl std::error::Error for I32Exit {}

#[allow(clippy::too_many_arguments)]
pub(crate) mod generated {
    wiggle::from_witx!({
        witx: ["$CARGO_MANIFEST_DIR/src/wasi/witx/wasi_snapshot_preview1.witx"],
        errors: { errno => trappable Error },
        wasmtime: false,
        tracing: false,

        async: {
            wasi_snapshot_preview1::{
                fd_read, fd_pread,
                fd_filestat_get, fd_seek, fd_tell,
            },
        },
    });

    wiggle::wasmtime_integration!({
        witx: ["$CARGO_MANIFEST_DIR/src/wasi/witx/wasi_snapshot_preview1.witx"],
        target: super::generated,
        errors: { errno => trappable Error },
        tracing: false,

        async: {
            wasi_snapshot_preview1::{
                fd_read, fd_pread,
                fd_filestat_get, fd_seek, fd_tell,
            },
        },
    });
}

impl wiggle::GuestErrorType for generated::types::Errno {
    fn success() -> Self {
        Self::Success
    }
}

impl From<std::num::TryFromIntError> for generated::types::Error {
    fn from(_err: std::num::TryFromIntError) -> Self {
        generated::types::Errno::Overflow.into()
    }
}

impl From<GuestError> for generated::types::Error {
    fn from(err: GuestError) -> Self {
        use wiggle::GuestError::*;
        match err {
            InvalidFlagValue { .. } => generated::types::Errno::Inval.into(),
            InvalidEnumValue { .. } => generated::types::Errno::Inval.into(),
            // As per
            // https://github.com/WebAssembly/wasi/blob/main/legacy/tools/witx-docs.md#pointers
            //
            // > If a misaligned pointer is passed to a function, the function
            // > shall trap.
            // >
            // > If an out-of-bounds pointer is passed to a function and the
            // > function needs to dereference it, the function shall trap.
            //
            // so this turns OOB and misalignment errors into traps.
            PtrOverflow { .. } | PtrOutOfBounds { .. } | PtrNotAligned { .. } => {
                generated::types::Error::trap(err.into())
            }
            PtrBorrowed { .. } => generated::types::Errno::Fault.into(),
            InvalidUtf8 { .. } => generated::types::Errno::Ilseq.into(),
            TryFromIntError { .. } => generated::types::Errno::Overflow.into(),
            SliceLengthsDiffer { .. } => generated::types::Errno::Fault.into(),
            BorrowCheckerOutOfHandles { .. } => generated::types::Errno::Fault.into(),
            InFunc { err, .. } => generated::types::Error::from(*err),
        }
    }
}

enum FilesTrie {
    Dir {
        children: BTreeMap<String, Box<FilesTrie>>,
    },
    File {
        data: SharedBytes,
    },
}

impl Context {
    pub fn log(&self) -> serde_json::Value {
        serde_json::json!(
            {
                "env": String::from_utf8_lossy(&self.env_buf),
                "args": String::from_utf8_lossy(&self.args_buf),
                "datetime_timestamp": self.unix_timestamp,
                "datetime": chrono::DateTime::<chrono::Utc>::from_timestamp(self.unix_timestamp as i64, 0).map(|x| x.to_rfc3339()),
            }
        )
    }
    pub fn set_args(&mut self, args: &[String]) -> Result<(), anyhow::Error> {
        for c in args {
            let off: u32 = self
                .args_buf
                .len()
                .try_into()
                .with_context(|| "arguments offset overflow")?;
            self.args_offsets.push(off);
            self.args_buf.extend_from_slice(c.as_bytes());
            self.args_buf.push(0);
        }
        Ok(())
    }

    pub fn set_env(&mut self, env: &[(String, String)]) -> Result<(), anyhow::Error> {
        for (name, val) in env {
            let off: u32 = self
                .env_buf
                .len()
                .try_into()
                .with_context(|| "env offset overflow")?;
            self.env_offsets.push(off);
            self.env_buf.extend_from_slice(name.as_bytes());
            self.env_buf.push(b'=');
            self.env_buf.extend_from_slice(val.as_bytes());
            self.env_buf.push(0);
        }
        Ok(())
    }

    pub fn map_file(&mut self, location: &str, contents: SharedBytes) -> anyhow::Result<()> {
        let mut location_patched = String::new();
        location_patched.reserve(location.len());

        let mut last_slash = true;
        for c in location.chars() {
            if c == '/' {
                if !last_slash {
                    location_patched.push(c);
                }
                last_slash = true;
            } else {
                last_slash = false;
                location_patched.push(c);
            }
        }

        let mut cur_trie: &mut FilesTrie = &mut self.fs;
        let locs_arr: Vec<&str> = location_patched.split("/").collect();
        for loc in &locs_arr[0..locs_arr.len() - 1] {
            cur_trie = match cur_trie.borrow_mut() {
                FilesTrie::Dir { children } => match children.entry(String::from(*loc)) {
                    std::collections::btree_map::Entry::Occupied(entry) => {
                        Ok::<&mut FilesTrie, anyhow::Error>(entry.into_mut())
                    }
                    std::collections::btree_map::Entry::Vacant(entry) => {
                        Ok(&mut **entry.insert(Box::new(FilesTrie::Dir {
                            children: BTreeMap::new(),
                        })))
                    }
                },
                FilesTrie::File { data: _ } => {
                    return Err(anyhow::anyhow!(
                        "super path is already mapped as a file {}",
                        location_patched
                    ))
                }
            }?;
        }

        let fname = locs_arr[locs_arr.len() - 1];

        match cur_trie.borrow_mut() {
            FilesTrie::Dir { children } => match children.entry(String::from(fname)) {
                std::collections::btree_map::Entry::Occupied(_entry) => Err(anyhow::anyhow!(
                    "duplicate file mapping {}",
                    location_patched
                )),
                std::collections::btree_map::Entry::Vacant(entry) => {
                    entry.insert(Box::new(FilesTrie::File { data: contents }));
                    Ok(())
                }
            },
            FilesTrie::File { data: _ } => {
                return Err(anyhow::anyhow!("super path is already mapped as a file"))
            }
        }?;

        Ok(())
    }
}

pub trait AddToLinkerFn<T> {
    fn call<'a>(&self, arg: &'a mut T) -> ContextVFS<'a>;
}

pub(super) fn add_to_linker_sync<T: Send + 'static, F>(
    linker: &mut wasmtime::Linker<T>,
    f: F,
) -> anyhow::Result<()>
where
    F: AddToLinkerFn<T> + Copy + Send + Sync + 'static,
{
    #[derive(Clone, Copy)]
    struct Fwd<F>(F);

    impl<T, F> generated::AddWasiSnapshotPreview1ToLinkerFn<T> for Fwd<F>
    where
        F: AddToLinkerFn<T> + Copy + Send + Sync + 'static,
    {
        fn call(
            &self,
            arg: &mut T,
        ) -> impl generated::wasi_snapshot_preview1::WasiSnapshotPreview1 {
            self.0.call(arg)
        }
    }
    generated::add_wasi_snapshot_preview1_to_linker(linker, Fwd(f))?;
    Ok(())
}

impl Context {
    pub fn new(datetime: chrono::DateTime<chrono::Utc>, conf: base::Config) -> Self {
        Self {
            args_buf: Vec::new(),
            args_offsets: Vec::new(),
            env_buf: Vec::new(),
            env_offsets: Vec::new(),
            fs: Box::new(FilesTrie::Dir {
                children: BTreeMap::new(),
            }),
            unix_timestamp: datetime.timestamp() as u64 * 1_000_000_000
                + datetime.timestamp_subsec_nanos() as u64,
            conf,
        }
    }
}

fn args_env_get(
    memory: &mut GuestMemory<'_>,
    mut guest_starts: GuestPtr<GuestPtr<u8>>,
    guest_buf: GuestPtr<u8>,
    offsets: &[u32],
    buf: &[u8],
) -> Result<(), generated::types::Error> {
    {
        let len: u32 = buf.len().try_into()?;
        let guest_buf_arr = guest_buf.as_array(len);
        memory.copy_from_slice(buf, guest_buf_arr)?;
    }

    for off_absolute in offsets.iter() {
        let to_write = guest_buf.add(*off_absolute)?;
        memory.write(guest_starts, to_write)?;
        guest_starts = guest_starts.add(1)?;
    }
    Ok(())
}

#[allow(unused_variables)]
#[async_trait::async_trait]
impl generated::wasi_snapshot_preview1::WasiSnapshotPreview1 for ContextVFS<'_> {
    #[instrument(skip(self, memory))]
    fn args_get(
        &mut self,
        memory: &mut GuestMemory<'_>,
        argv: GuestPtr<GuestPtr<u8>>,
        argv_buf: GuestPtr<u8>,
    ) -> Result<(), generated::types::Error> {
        args_env_get(
            memory,
            argv,
            argv_buf,
            &self.context.args_offsets,
            &self.context.args_buf,
        )
    }

    fn args_sizes_get(
        &mut self,
        _memory: &mut GuestMemory<'_>,
    ) -> Result<(generated::types::Size, generated::types::Size), generated::types::Error> {
        let count: u32 = self.context.args_offsets.len().try_into()?;
        let len: u32 = self.context.args_buf.len().try_into()?;
        Ok((count, len))
    }

    #[instrument(skip(self, memory))]
    fn environ_get(
        &mut self,
        memory: &mut GuestMemory<'_>,
        environ: GuestPtr<GuestPtr<u8>>,
        environ_buf: GuestPtr<u8>,
    ) -> Result<(), generated::types::Error> {
        args_env_get(
            memory,
            environ,
            environ_buf,
            &self.context.env_offsets,
            &self.context.env_buf,
        )
    }

    fn environ_sizes_get(
        &mut self,
        _memory: &mut GuestMemory<'_>,
    ) -> Result<(generated::types::Size, generated::types::Size), generated::types::Error> {
        let count: u32 = self.context.env_offsets.len().try_into()?;
        let len: u32 = self.context.env_buf.len().try_into()?;
        Ok((count, len))
    }

    fn clock_res_get(
        &mut self,
        _memory: &mut GuestMemory<'_>,
        id: generated::types::Clockid,
    ) -> Result<generated::types::Timestamp, generated::types::Error> {
        panic!("todo")
    }

    fn clock_time_get(
        &mut self,
        _memory: &mut GuestMemory<'_>,
        id: generated::types::Clockid,
        _precision: generated::types::Timestamp,
    ) -> Result<generated::types::Timestamp, generated::types::Error> {
        Ok(self.context.unix_timestamp)
    }

    fn fd_advise(
        &mut self,
        _memory: &mut GuestMemory<'_>,
        fd: generated::types::Fd,
        offset: generated::types::Filesize,
        len: generated::types::Filesize,
        advice: generated::types::Advice,
    ) -> Result<(), generated::types::Error> {
        Ok(())
    }

    /// Force the allocation of space in a file.
    /// NOTE: This is similar to `posix_fallocate` in POSIX.
    fn fd_allocate(
        &mut self,
        _memory: &mut GuestMemory<'_>,
        fd: generated::types::Fd,
        _offset: generated::types::Filesize,
        _len: generated::types::Filesize,
    ) -> Result<(), generated::types::Error> {
        Err(generated::types::Errno::Rofs.into())
    }

    /// Close a file descriptor.
    /// NOTE: This is similar to `close` in POSIX.
    fn fd_close(
        &mut self,
        _memory: &mut GuestMemory<'_>,
        fd: generated::types::Fd,
    ) -> Result<(), generated::types::Error> {
        let fdi: u32 = fd.into();
        match self.vfs.fds.remove(&fdi) {
            Some(_) => {
                self.vfs.free_fd(fdi);
                Ok(())
            }
            None => Err(generated::types::Errno::Badf.into()),
        }
    }

    /// Synchronize the data of a file to disk.
    /// NOTE: This is similar to `fdatasync` in POSIX.
    fn fd_datasync(
        &mut self,
        _memory: &mut GuestMemory<'_>,
        fd: generated::types::Fd,
    ) -> Result<(), generated::types::Error> {
        Ok(())
    }

    /// Get the attributes of a file descriptor.
    /// NOTE: This returns similar flags to `fsync(fd, F_GETFL)` in POSIX, as well as additional fields.
    fn fd_fdstat_get(
        &mut self,
        _memory: &mut GuestMemory<'_>,
        fd: generated::types::Fd,
    ) -> Result<generated::types::Fdstat, generated::types::Error> {
        match self.get_fd_desc(fd)? {
            FileDescriptor::Stdin => Ok(generated::types::Fdstat {
                fs_filetype: generated::types::Filetype::Unknown,
                fs_flags: generated::types::Fdflags::empty(),
                fs_rights_base: generated::types::Rights::FD_READ,
                fs_rights_inheriting: generated::types::Rights::FD_READ,
            }),
            FileDescriptor::Stdout | FileDescriptor::Stderr => Ok(generated::types::Fdstat {
                fs_filetype: generated::types::Filetype::Unknown,
                fs_flags: generated::types::Fdflags::empty(),
                fs_rights_base: generated::types::Rights::FD_WRITE,
                fs_rights_inheriting: generated::types::Rights::FD_WRITE,
            }),
            FileDescriptor::File { .. } => {
                let rights = generated::types::Rights::FD_DATASYNC
                    | generated::types::Rights::FD_READ
                    | generated::types::Rights::FD_SEEK
                    | generated::types::Rights::FD_SYNC
                    | generated::types::Rights::FD_TELL
                    | generated::types::Rights::FD_ADVISE
                    | generated::types::Rights::PATH_OPEN
                    | generated::types::Rights::FD_READDIR
                    | generated::types::Rights::PATH_READLINK
                    | generated::types::Rights::PATH_FILESTAT_GET
                    | generated::types::Rights::FD_FILESTAT_GET;
                Ok(generated::types::Fdstat {
                    fs_filetype: generated::types::Filetype::RegularFile,
                    fs_flags: generated::types::Fdflags::empty(),
                    fs_rights_base: rights,
                    fs_rights_inheriting: rights,
                })
            }
            FileDescriptor::Dir { .. } => {
                let rights = generated::types::Rights::FD_READ
                    | generated::types::Rights::PATH_OPEN
                    | generated::types::Rights::FD_READDIR
                    | generated::types::Rights::PATH_READLINK
                    | generated::types::Rights::PATH_FILESTAT_GET
                    | generated::types::Rights::PATH_FILESTAT_GET
                    | generated::types::Rights::FD_READ
                    | generated::types::Rights::FD_FILESTAT_GET
                    | generated::types::Rights::FD_FILESTAT_GET;
                Ok(generated::types::Fdstat {
                    fs_filetype: generated::types::Filetype::Directory,
                    fs_flags: generated::types::Fdflags::empty(),
                    fs_rights_base: rights,
                    fs_rights_inheriting: rights,
                })
            }
        }
    }

    /// Adjust the flags associated with a file descriptor.
    /// NOTE: This is similar to `fcntl(fd, F_SETFL, flags)` in POSIX.
    fn fd_fdstat_set_flags(
        &mut self,
        _memory: &mut GuestMemory<'_>,
        fd: generated::types::Fd,
        flags: generated::types::Fdflags,
    ) -> Result<(), generated::types::Error> {
        Err(generated::types::Errno::Rofs.into())
    }

    /// Does not do anything if `fd` corresponds to a valid descriptor and returns `[stub::types::Errno::Badf]` error otherwise.
    fn fd_fdstat_set_rights(
        &mut self,
        _memory: &mut GuestMemory<'_>,
        fd: generated::types::Fd,
        _fs_rights_base: generated::types::Rights,
        _fs_rights_inheriting: generated::types::Rights,
    ) -> Result<(), generated::types::Error> {
        Err(generated::types::Errno::Rofs.into())
    }

    /// Return the attributes of an open file.
    async fn fd_filestat_get(
        &mut self,
        _memory: &mut GuestMemory<'_>,
        fd: generated::types::Fd,
    ) -> Result<generated::types::Filestat, generated::types::Error> {
        match self.get_fd_desc_mut(fd)? {
            FileDescriptor::Stdin | FileDescriptor::Stdout | FileDescriptor::Stderr => {
                Ok(generated::types::Filestat {
                    dev: 0,
                    ino: 0,
                    filetype: generated::types::Filetype::CharacterDevice,
                    nlink: 1,
                    size: 0,
                    atim: 0,
                    mtim: 0,
                    ctim: 0,
                })
            }
            FileDescriptor::File(file) => {
                let contents = file.get().await.map_err(generated::types::Error::trap)?;
                Ok(generated::types::Filestat {
                    dev: 0,
                    ino: 0,
                    filetype: generated::types::Filetype::RegularFile,
                    nlink: 1,
                    size: contents.contents.len().try_into()?,
                    atim: 0,
                    mtim: 0,
                    ctim: 0,
                })
            }
            FileDescriptor::Dir { .. } => Ok(generated::types::Filestat {
                dev: 0,
                ino: 0,
                filetype: generated::types::Filetype::Directory,
                nlink: 1,
                size: 0,
                atim: 0,
                mtim: 0,
                ctim: 0,
            }),
        }
    }

    /// Adjust the size of an open file. If this increases the file's size, the extra bytes are filled with zeros.
    /// NOTE: This is similar to `ftruncate` in POSIX.
    fn fd_filestat_set_size(
        &mut self,
        _memory: &mut GuestMemory<'_>,
        fd: generated::types::Fd,
        size: generated::types::Filesize,
    ) -> Result<(), generated::types::Error> {
        self.get_fd_desc(fd)?;
        Err(generated::types::Errno::Rofs.into())
    }

    /// Adjust the timestamps of an open file or directory.
    /// NOTE: This is similar to `futimens` in POSIX.
    fn fd_filestat_set_times(
        &mut self,
        _memory: &mut GuestMemory<'_>,
        fd: generated::types::Fd,
        atim: generated::types::Timestamp,
        mtim: generated::types::Timestamp,
        fst_flags: generated::types::Fstflags,
    ) -> Result<(), generated::types::Error> {
        self.get_fd_desc(fd)?;
        Err(generated::types::Errno::Rofs.into())
    }

    /// Read from a file descriptor.
    /// NOTE: This is similar to `readv` in POSIX.
    #[instrument(skip(self, memory))]
    async fn fd_read(
        &mut self,
        memory: &mut GuestMemory<'_>,
        fd: generated::types::Fd,
        iovs: generated::types::IovecArray,
    ) -> Result<generated::types::Size, generated::types::Error> {
        match self.get_fd_desc_mut(fd)? {
            FileDescriptor::Stdin => Ok(0),
            FileDescriptor::Stdout | FileDescriptor::Stderr => {
                Err(generated::types::Errno::Acces.into())
            }
            FileDescriptor::File(file) => {
                let FileContents { contents, pos } =
                    file.get().await.map_err(generated::types::Error::trap)?;
                let mut written: u32 = 0;
                for iov in iovs.iter() {
                    let iov = iov?;
                    let iov = memory.read(iov)?;
                    let remaining_len = contents.len() - *pos;
                    let mut len = iov.buf_len;
                    let len_usize: usize = len.try_into()?;
                    if remaining_len < len_usize {
                        len = remaining_len.try_into().unwrap();
                    }
                    let len_usize: usize = len.try_into()?;
                    let cont_slice: &[u8] = &contents.as_ref()[*pos..(*pos + len_usize)];
                    memory.copy_from_slice(cont_slice, iov.buf.as_array(len))?;
                    *pos += len_usize;
                    written += len;
                }
                Ok(written)
            }
            FileDescriptor::Dir { .. } => Err(generated::types::Errno::Isdir.into()),
        }
    }

    /// Read from a file descriptor, without using and updating the file descriptor's offset.
    /// NOTE: This is similar to `preadv` in POSIX.
    #[instrument(skip(self, memory))]
    async fn fd_pread(
        &mut self,
        memory: &mut GuestMemory<'_>,
        fd: generated::types::Fd,
        iovs: generated::types::IovecArray,
        offset: generated::types::Filesize,
    ) -> Result<generated::types::Size, generated::types::Error> {
        match self.get_fd_desc(fd)? {
            FileDescriptor::Stdin => Ok(0),
            FileDescriptor::Stdout | FileDescriptor::Stderr => {
                Err(generated::types::Errno::Acces.into())
            }
            FileDescriptor::File(_) => {
                todo!()
            }
            FileDescriptor::Dir { .. } => Err(generated::types::Errno::Isdir.into()),
        }
    }

    /// Write to a file descriptor.
    /// NOTE: This is similar to `writev` in POSIX.
    #[instrument(skip(self, memory))]
    fn fd_write(
        &mut self,
        memory: &mut GuestMemory<'_>,
        fd: generated::types::Fd,
        ciovs: generated::types::CiovecArray,
    ) -> Result<generated::types::Size, generated::types::Error> {
        let mut stream: Box<dyn Write> = match self.get_fd_desc(fd)? {
            FileDescriptor::Stdout => Box::new(std::io::stdout().lock()),
            FileDescriptor::Stderr => Box::new(std::io::stderr().lock()),
            FileDescriptor::Stdin => return Err(generated::types::Errno::Notsup.into()),
            _ => return Err(generated::types::Errno::Rofs.into()),
        };
        let mut size: u32 = 0;
        for ciov in ciovs.iter() {
            let ciov_read = memory.read(ciov?)?;
            if ciov_read.buf_len == 0 {
                continue;
            }
            let buf_to_rewrite = ciov_read.buf.as_array(ciov_read.buf_len);
            let cow = memory.as_cow(buf_to_rewrite)?;
            let add_size: u32 = cow.len().try_into()?;
            size += add_size;
            stream
                .write_all(&cow)
                .unwrap_or_else(|_| panic!("Can't print to terminal"));
        }
        stream
            .flush()
            .unwrap_or_else(|_| panic!("Can't print to terminal"));
        Ok(size)
    }

    /// Write to a file descriptor, without using and updating the file descriptor's offset.
    /// NOTE: This is similar to `pwritev` in POSIX.
    #[instrument(skip(self, memory))]
    fn fd_pwrite(
        &mut self,
        memory: &mut GuestMemory<'_>,
        fd: generated::types::Fd,
        ciovs: generated::types::CiovecArray,
        offset: generated::types::Filesize,
    ) -> Result<generated::types::Size, generated::types::Error> {
        self.get_fd_desc(fd)?;
        Err(generated::types::Errno::Notsup.into())
    }

    /// Return a description of the given preopened file descriptor.
    fn fd_prestat_get(
        &mut self,
        _memory: &mut GuestMemory<'_>,
        fd: generated::types::Fd,
    ) -> Result<generated::types::Prestat, generated::types::Error> {
        return match self.get_fd_desc(fd)? {
            FileDescriptor::Dir { path } => {
                let path_last = if path.is_empty() {
                    "/"
                } else {
                    &path[path.len() - 1]
                };
                Ok(generated::types::Prestat::Dir(
                    generated::types::PrestatDir {
                        pr_name_len: path_last.len().try_into()?,
                    },
                ))
            }
            _ => Err(generated::types::Errno::Badf.into()),
        };
    }

    /// Return a description of the given preopened file descriptor.
    #[instrument(skip(self, memory))]
    fn fd_prestat_dir_name(
        &mut self,
        memory: &mut GuestMemory<'_>,
        fd: generated::types::Fd,
        path: GuestPtr<u8>,
        path_max_len: generated::types::Size,
    ) -> Result<(), generated::types::Error> {
        match self.get_fd_desc(fd)? {
            FileDescriptor::Dir { path: dir_path } => {
                let path_last = if dir_path.is_empty() {
                    "/"
                } else {
                    &dir_path[dir_path.len() - 1]
                };
                let name_len: u32 = path_last.len().try_into()?;
                if path_max_len < name_len {
                    return Err(generated::types::Errno::Overflow.into());
                }
                let arr = path.as_array(name_len);
                memory.copy_from_slice(path_last.as_bytes(), arr)?;
                Ok(())
            }
            _ => Err(generated::types::Errno::Badf.into()),
        }
    }

    /// Atomically replace a file descriptor by renumbering another file descriptor.
    fn fd_renumber(
        &mut self,
        _memory: &mut GuestMemory<'_>,
        from: generated::types::Fd,
        to: generated::types::Fd,
    ) -> Result<(), generated::types::Error> {
        todo!()
    }

    /// Move the offset of a file descriptor.
    /// NOTE: This is similar to `lseek` in POSIX.
    async fn fd_seek(
        &mut self,
        _memory: &mut GuestMemory<'_>,
        fd: generated::types::Fd,
        offset: generated::types::Filedelta,
        whence: generated::types::Whence,
    ) -> Result<generated::types::Filesize, generated::types::Error> {
        match self.get_fd_desc_mut(fd)? {
            FileDescriptor::Stdin | FileDescriptor::Stderr | FileDescriptor::Stdout => {
                Err(generated::types::Errno::Spipe.into())
            }
            FileDescriptor::File(file) => {
                const {
                    assert!(std::mem::size_of::<usize>() <= std::mem::size_of::<u64>());
                }
                let FileContents { contents, pos } =
                    file.get().await.map_err(generated::types::Error::trap)?;
                match whence {
                    generated::types::Whence::Cur => {
                        if offset < 0 {
                            let offset = -offset as u64;
                            if offset > *pos as u64 {
                                *pos = 0;
                            } else {
                                *pos -= offset as usize;
                            }
                        } else {
                            let offset = offset as u64;
                            let rem = contents.len() - *pos;
                            if offset > rem as u64 {
                                *pos = contents.len();
                            } else {
                                *pos += offset as usize;
                            }
                        }
                    }
                    generated::types::Whence::End => {
                        return Err(generated::types::Errno::Notsup.into())
                    }
                    generated::types::Whence::Set => {
                        let offset = if offset < 0 { 0 } else { offset as u64 };
                        if offset > contents.len() as u64 {
                            *pos = contents.len();
                        } else {
                            *pos = offset as usize;
                        }
                    }
                };
                return u64::try_from(*pos).map_err(|_e| generated::types::Errno::Overflow.into());
            }
            FileDescriptor::Dir { .. } => Err(generated::types::Errno::Notsup.into()),
        }
    }

    /// Synchronize the data and metadata of a file to disk.
    /// NOTE: This is similar to `fsync` in POSIX.
    fn fd_sync(
        &mut self,
        _memory: &mut GuestMemory<'_>,
        fd: generated::types::Fd,
    ) -> Result<(), generated::types::Error> {
        Ok(())
    }

    /// Return the current offset of a file descriptor.
    /// NOTE: This is similar to `lseek(fd, 0, SEEK_CUR)` in POSIX.
    async fn fd_tell(
        &mut self,
        _memory: &mut GuestMemory<'_>,
        fd: generated::types::Fd,
    ) -> Result<generated::types::Filesize, generated::types::Error> {
        match self.get_fd_desc_mut(fd)? {
            FileDescriptor::Stdin | FileDescriptor::Stderr | FileDescriptor::Stdout => {
                Err(generated::types::Errno::Spipe.into())
            }
            FileDescriptor::File(file) => {
                let file = file.get().await.map_err(generated::types::Error::trap)?;
                Ok(file.pos.try_into()?)
            }
            FileDescriptor::Dir { .. } => Err(generated::types::Errno::Notsup.into()),
        }
    }

    #[instrument(skip(self, memory))]
    fn fd_readdir(
        &mut self,
        memory: &mut GuestMemory<'_>,
        fd: generated::types::Fd,
        buf: GuestPtr<u8>,
        buf_len: generated::types::Size,
        cookie: generated::types::Dircookie,
    ) -> Result<generated::types::Size, generated::types::Error> {
        let FileDescriptor::Dir { path: dir_path } = self.get_fd_desc(fd)? else {
            return Err(generated::types::Errno::Badf.into());
        };

        let dirent = self.dir_fd_follow_trie(dir_path, &self.context.fs, None)?;
        let FilesTrie::Dir {
            children: direntries,
        } = dirent
        else {
            return Err(generated::types::Errno::Badf.into());
        };

        let head = [
            (
                generated::types::Dirent {
                    d_next: 1u64.to_le(),
                    d_ino: 0,
                    d_type: generated::types::Filetype::Directory,
                    d_namlen: 1u32.to_le(),
                },
                ".".into(),
            ),
            (
                generated::types::Dirent {
                    d_next: 2u64.to_le(),
                    d_ino: 0,
                    d_type: generated::types::Filetype::Directory,
                    d_namlen: 2u32.to_le(),
                },
                "..".into(),
            ),
        ];

        const DIRENT_SIZE: u32 = size_of::<generated::types::Dirent>() as _;

        let dirent_actual_iter = direntries.iter().zip(3u64..).map(|(x, idx)| {
            let name_len: u32 = x.0.len().try_into().unwrap();
            (
                generated::types::Dirent {
                    d_next: idx.to_le(),
                    d_ino: 0,
                    d_type: match **x.1 {
                        FilesTrie::Dir { .. } => generated::types::Filetype::Directory,
                        FilesTrie::File { .. } => generated::types::Filetype::RegularFile,
                    },
                    d_namlen: name_len.to_le(),
                },
                x.0.clone(),
            )
        });

        let mut buf = buf;
        let mut cap = buf_len;
        let cookie = cookie.try_into()?;
        for (ref entry, path) in head.into_iter().chain(dirent_actual_iter).skip(cookie) {
            let mut path = path.into_bytes();
            assert_eq!(
                1,
                size_of_val(&entry.d_type),
                "Dirent member d_type should be endian-invariant"
            );
            let entry_len = cap.min(DIRENT_SIZE);
            let entry = entry as *const _ as _;
            let entry = unsafe { std::slice::from_raw_parts(entry, entry_len as _) };
            cap = cap.checked_sub(entry_len).unwrap();
            buf = write_bytes(memory, buf, entry)?;
            if cap == 0 {
                return Ok(buf_len);
            }

            if let Ok(cap) = cap.try_into() {
                // `path` cannot be longer than `usize`, only truncate if `cap` fits in `usize`
                path.truncate(cap);
            }
            cap = cap.checked_sub(path.len() as _).unwrap();
            buf = write_bytes(memory, buf, &path)?;
            if cap == 0 {
                return Ok(buf_len);
            }
        }
        Ok(buf_len.checked_sub(cap).unwrap())
    }

    #[instrument(skip(self, memory))]
    fn path_create_directory(
        &mut self,
        memory: &mut GuestMemory<'_>,
        dirfd: generated::types::Fd,
        path: GuestPtr<str>,
    ) -> Result<(), generated::types::Error> {
        Err(generated::types::Errno::Rofs.into())
    }

    /// Return the attributes of a file or directory.
    /// NOTE: This is similar to `stat` in POSIX.
    #[instrument(skip(self, memory))]
    fn path_filestat_get(
        &mut self,
        memory: &mut GuestMemory<'_>,
        dirfd: generated::types::Fd,
        flags: generated::types::Lookupflags,
        path: GuestPtr<str>,
    ) -> Result<generated::types::Filestat, generated::types::Error> {
        let fdi: u32 = dirfd.into();
        let path = super::common::read_string(memory, path)?;
        let Some(FileDescriptor::Dir { path: dir_path }) = self.vfs.fds.get(&fdi) else {
            return Err(generated::types::Errno::Badf.into());
        };
        let mut result_path = dir_path.clone();
        let mut cur_trie =
            self.dir_fd_follow_trie(dir_path, &self.context.fs, Some(&mut result_path))?;
        for fname in path.split("/") {
            cur_trie = self.dir_fd_get_trie(fname, cur_trie, &mut Some(&mut result_path))?;
        }
        match cur_trie {
            FilesTrie::File { data } => Ok(generated::types::Filestat {
                dev: 0,
                ino: 0,
                filetype: generated::types::Filetype::RegularFile,
                nlink: 0,
                size: data.len().try_into()?,
                atim: 0,
                mtim: 0,
                ctim: 0,
            }),
            FilesTrie::Dir { .. } => Ok(generated::types::Filestat {
                dev: 0,
                ino: 0,
                filetype: generated::types::Filetype::Directory,
                nlink: 0,
                size: 0,
                atim: 0,
                mtim: 0,
                ctim: 0,
            }),
        }
    }

    /// Adjust the timestamps of a file or directory.
    /// NOTE: This is similar to `utimensat` in POSIX.
    #[instrument(skip(self, memory))]
    fn path_filestat_set_times(
        &mut self,
        memory: &mut GuestMemory<'_>,
        dirfd: generated::types::Fd,
        flags: generated::types::Lookupflags,
        path: GuestPtr<str>,
        atim: generated::types::Timestamp,
        mtim: generated::types::Timestamp,
        fst_flags: generated::types::Fstflags,
    ) -> Result<(), generated::types::Error> {
        Err(generated::types::Errno::Rofs.into())
    }

    /// Create a hard link.
    /// NOTE: This is similar to `linkat` in POSIX.
    #[instrument(skip(self, memory))]
    fn path_link(
        &mut self,
        memory: &mut GuestMemory<'_>,
        src_fd: generated::types::Fd,
        src_flags: generated::types::Lookupflags,
        src_path: GuestPtr<str>,
        target_fd: generated::types::Fd,
        target_path: GuestPtr<str>,
    ) -> Result<(), generated::types::Error> {
        Err(generated::types::Errno::Rofs.into())
    }

    /// Open a file or directory.
    /// NOTE: This is similar to `openat` in POSIX.
    #[instrument(skip(self, memory))]
    fn path_open(
        &mut self,
        memory: &mut GuestMemory<'_>,
        dirfd: generated::types::Fd,
        dirflags: generated::types::Lookupflags,
        path: GuestPtr<str>,
        oflags: generated::types::Oflags,
        fs_rights_base: generated::types::Rights,
        _fs_rights_inheriting: generated::types::Rights,
        fdflags: generated::types::Fdflags,
    ) -> Result<generated::types::Fd, generated::types::Error> {
        let file_path = super::common::read_string(memory, path)?;
        let fdi: u32 = dirfd.into();
        let fdi: u32 = dirfd.into();
        let new_fd = self.vfs.alloc_fd();
        {
            let Some(FileDescriptor::Dir { path: dir_path }) = self.vfs.fds.get(&fdi) else {
                return Err(generated::types::Errno::Badf.into());
            };
            let mut resulting_path = dir_path.clone();
            let mut cur_trie =
                self.dir_fd_follow_trie(dir_path, &self.context.fs, Some(&mut resulting_path))?;
            for fname in file_path.split("/") {
                cur_trie = self.dir_fd_get_trie(fname, cur_trie, &mut Some(&mut resulting_path))?;
            }
            match cur_trie {
                FilesTrie::File { data } => {
                    let f = FileDescriptor::File(
                        super::common::FileContentsUnevaluated::from_contents(data.clone(), 0),
                    );
                    self.vfs.fds.insert(new_fd, f);
                    Ok(new_fd.into())
                }
                FilesTrie::Dir { .. } => {
                    let f = FileDescriptor::Dir {
                        path: resulting_path,
                    };
                    self.vfs.fds.insert(new_fd, f);
                    Ok(new_fd.into())
                }
            }
        }
        .inspect_err(|e| {
            self.vfs.free_fd(new_fd);
        })
    }

    /// Read the contents of a symbolic link.
    /// NOTE: This is similar to `readlinkat` in POSIX.
    #[instrument(skip(self, memory))]
    fn path_readlink(
        &mut self,
        memory: &mut GuestMemory<'_>,
        dirfd: generated::types::Fd,
        path: GuestPtr<str>,
        buf: GuestPtr<u8>,
        buf_len: generated::types::Size,
    ) -> Result<generated::types::Size, generated::types::Error> {
        Err(generated::types::Errno::Badf.into())
    }

    #[instrument(skip(self, memory))]
    fn path_remove_directory(
        &mut self,
        memory: &mut GuestMemory<'_>,
        dirfd: generated::types::Fd,
        path: GuestPtr<str>,
    ) -> Result<(), generated::types::Error> {
        Err(generated::types::Errno::Rofs.into())
    }

    /// Rename a file or directory.
    /// NOTE: This is similar to `renameat` in POSIX.
    #[instrument(skip(self, memory))]
    fn path_rename(
        &mut self,
        memory: &mut GuestMemory<'_>,
        src_fd: generated::types::Fd,
        src_path: GuestPtr<str>,
        dest_fd: generated::types::Fd,
        dest_path: GuestPtr<str>,
    ) -> Result<(), generated::types::Error> {
        Err(generated::types::Errno::Rofs.into())
    }

    #[instrument(skip(self, memory))]
    fn path_symlink(
        &mut self,
        memory: &mut GuestMemory<'_>,
        src_path: GuestPtr<str>,
        dirfd: generated::types::Fd,
        dest_path: GuestPtr<str>,
    ) -> Result<(), generated::types::Error> {
        Err(generated::types::Errno::Rofs.into())
    }

    #[instrument(skip(self, memory))]
    fn path_unlink_file(
        &mut self,
        memory: &mut GuestMemory<'_>,
        dirfd: generated::types::Fd,
        path: GuestPtr<str>,
    ) -> Result<(), generated::types::Error> {
        Err(generated::types::Errno::Rofs.into())
    }

    #[instrument(skip(self, memory))]
    fn poll_oneoff(
        &mut self,
        memory: &mut GuestMemory<'_>,
        subs: GuestPtr<generated::types::Subscription>,
        events: GuestPtr<generated::types::Event>,
        nsubscriptions: generated::types::Size,
    ) -> Result<generated::types::Size, generated::types::Error> {
        Err(generated::types::Errno::Io.into())
    }

    fn proc_exit(
        &mut self,
        _memory: &mut GuestMemory<'_>,
        status: generated::types::Exitcode,
    ) -> anyhow::Error {
        // Check that the status is within WASI's range.
        if status >= 126 {
            return anyhow::Error::msg("exit with invalid exit status outside of [0..126)");
        }
        I32Exit(status as i32).into()
    }

    fn proc_raise(
        &mut self,
        _memory: &mut GuestMemory<'_>,
        _sig: generated::types::Signal,
    ) -> Result<(), generated::types::Error> {
        Err(generated::types::Errno::Notsup.into())
    }

    fn sched_yield(
        &mut self,
        _memory: &mut GuestMemory<'_>,
    ) -> Result<(), generated::types::Error> {
        // No such thing in preview 2. Intentionally left empty.
        //Ok(())

        Err(generated::types::Errno::Notsup.into())
    }

    #[instrument(skip(self, memory))]
    fn random_get(
        &mut self,
        memory: &mut GuestMemory<'_>,
        buf: GuestPtr<u8>,
        buf_len: generated::types::Size,
    ) -> Result<(), generated::types::Error> {
        let mut mem: Vec<u8> = iter::repeat(0)
            .take(usize::try_from(buf_len).unwrap())
            .collect();

        if !self.context.conf.is_deterministic {
            if let Err(e) = getrandom::fill(&mut mem) {
                log::error!(error:err = e; "random failed");
            }
        }

        memory.copy_from_slice(&mem, buf.as_array(buf_len))?;
        Ok(())
    }

    #[allow(unused_variables)]
    fn sock_accept(
        &mut self,
        _memory: &mut GuestMemory<'_>,
        fd: generated::types::Fd,
        flags: generated::types::Fdflags,
    ) -> Result<generated::types::Fd, generated::types::Error> {
        Err(generated::types::Errno::Acces.into())
    }

    #[allow(unused_variables)]
    fn sock_recv(
        &mut self,
        _memory: &mut GuestMemory<'_>,
        fd: generated::types::Fd,
        ri_data: generated::types::IovecArray,
        ri_flags: generated::types::Riflags,
    ) -> Result<(generated::types::Size, generated::types::Roflags), generated::types::Error> {
        Err(generated::types::Errno::Acces.into())
    }

    #[allow(unused_variables)]
    fn sock_send(
        &mut self,
        _memory: &mut GuestMemory<'_>,
        fd: generated::types::Fd,
        si_data: generated::types::CiovecArray,
        _si_flags: generated::types::Siflags,
    ) -> Result<generated::types::Size, generated::types::Error> {
        Err(generated::types::Errno::Acces.into())
    }

    #[allow(unused_variables)]
    fn sock_shutdown(
        &mut self,
        _memory: &mut GuestMemory<'_>,
        fd: generated::types::Fd,
        how: generated::types::Sdflags,
    ) -> Result<(), generated::types::Error> {
        Err(generated::types::Errno::Acces.into())
    }
}

impl ContextVFS<'_> {
    fn dir_fd_get_trie<'a>(
        &self,
        dir_path: &str,
        cur_trie: &'a FilesTrie,
        path: &mut Option<&mut Vec<String>>,
    ) -> Result<&'a FilesTrie, generated::types::Error> {
        if dir_path == "." || dir_path.is_empty() {
            return Ok(cur_trie);
        }
        if dir_path == ".." {
            match path {
                None => return Err(generated::types::Errno::Noent.into()),
                Some(rf) => {
                    let _ = rf.pop();
                    let goto = rf.clone();
                    self.dir_fd_follow_trie(&goto, &self.context.fs, Some(rf))?;
                }
            }
        }
        match cur_trie {
            FilesTrie::File { .. } => Err(generated::types::Errno::Badf.into()),
            FilesTrie::Dir { children } => match children.get(dir_path) {
                Some(new_trie) => {
                    if let Some(rf) = path {
                        rf.push(dir_path.into());
                    }
                    Ok(new_trie)
                }
                None => Err(generated::types::Errno::Noent.into()),
            },
        }
    }

    fn dir_fd_follow_trie<'a>(
        &self,
        dir_path: &Vec<String>,
        mut cur_trie: &'a FilesTrie,
        mut path: Option<&mut Vec<String>>,
    ) -> Result<&'a FilesTrie, generated::types::Error> {
        for dir in dir_path {
            cur_trie = self.dir_fd_get_trie(dir, cur_trie, &mut path)?;
        }
        Ok(cur_trie)
    }

    fn get_fd_desc(
        &self,
        fd: generated::types::Fd,
    ) -> Result<&FileDescriptor, generated::types::Error> {
        let fdi: u32 = fd.into();
        match self.vfs.fds.get(&fdi) {
            Some(x) => Ok(x),
            None => Err(generated::types::Errno::Badf.into()),
        }
    }

    fn get_fd_desc_mut(
        &mut self,
        fd: generated::types::Fd,
    ) -> Result<&mut FileDescriptor, generated::types::Error> {
        let fdi: u32 = fd.into();
        match self.vfs.fds.get_mut(&fdi) {
            Some(x) => Ok(x),
            None => Err(generated::types::Errno::Badf.into()),
        }
    }
}

fn write_bytes(
    memory: &mut GuestMemory<'_>,
    ptr: GuestPtr<u8>,
    buf: &[u8],
) -> Result<GuestPtr<u8>, generated::types::Error> {
    let len = u32::try_from(buf.len())?;

    memory.copy_from_slice(buf, ptr.as_array(len))?;
    let next = ptr.add(len)?;
    Ok(next)
}
