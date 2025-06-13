use genvm_common::*;
use std::ptr::NonNull;

use anyhow::Context;

use crate::{errors::ContractError, memlimiter};

#[derive(Debug)]
struct Mmap(NonNull<[u8]>);

unsafe impl Send for Mmap {}
unsafe impl Sync for Mmap {}

impl AsRef<[u8]> for Mmap {
    fn as_ref(&self) -> &[u8] {
        unsafe { self.0.as_ref() }
    }
}

impl Drop for Mmap {
    fn drop(&mut self) {
        unsafe {
            let ptr = self.0.as_ptr().cast();
            let len = (*self.0.as_ptr()).len();
            if len == 0 {
                return;
            }
            match rustix::mm::munmap(ptr, len) {
                Ok(_) => {}
                Err(e) => {
                    log_error!(errno:? = e; "munmap failed")
                }
            }
        }
    }
}

pub fn load_file(
    path: &std::path::Path,
    limiter: Option<&memlimiter::Limiter>,
) -> anyhow::Result<impl AsRef<[u8]> + Send + Sync + 'static> {
    let file = std::fs::File::open(path).with_context(|| format!("opening {:?}", path))?;

    let file_len = file
        .metadata()
        .context("failed to get file metadata")?
        .len();
    let file_len = u32::try_from(file_len).map_err(|_| anyhow::anyhow!("file too large to map"))?;

    if let Some(limiter) = limiter {
        if !limiter.consume(file_len) {
            return Err(ContractError::oom(None).into());
        }
    }

    let file_len = file_len as usize;

    let ptr = unsafe {
        rustix::mm::mmap(
            std::ptr::null_mut(),
            file_len,
            rustix::mm::ProtFlags::READ,
            rustix::mm::MapFlags::PRIVATE,
            &file,
            0,
        )?
    };

    let memory = std::ptr::slice_from_raw_parts_mut(ptr.cast(), file_len);
    Ok(Mmap(std::ptr::NonNull::new(memory).unwrap()))
}
