use std::ops::DerefMut;

use genvm_common::{calldata, sync};

use crate::{rt, SlotID};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(C)]
pub struct PageID(pub SlotID, pub u32);

impl PageID {
    pub fn to_bytes(&self) -> [u8; 36] {
        let mut res = [0u8; 36];
        res[..32].copy_from_slice(&self.0.raw());
        res[32..].copy_from_slice(&self.1.to_le_bytes());
        res
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Delta(
    #[serde(with = "serde_bytes")] [u8; 36],
    #[serde(with = "serde_bytes")] Vec<u8>,
);

pub trait HostStorage {
    fn storage_read(&mut self, slot_id: SlotID, index: u32, buf: &mut [u8]) -> anyhow::Result<()>;
}

impl<HS: HostStorage, T: DerefMut<Target = HS>> HostStorage for T {
    fn storage_read(&mut self, slot_id: SlotID, index: u32, buf: &mut [u8]) -> anyhow::Result<()> {
        self.deref_mut().storage_read(slot_id, index, buf)
    }
}

pub trait HostStorageLocking {
    type ReturnType<'a>: HostStorage
    where
        Self: 'a;

    fn lock(&self) -> impl std::future::Future<Output = Self::ReturnType<'_>> + Send;
}

#[derive(Clone, Debug)]
pub struct Limiter(sync::DArc<std::sync::atomic::AtomicU64>);

impl Limiter {
    pub fn new(storage_pages_limit: sync::DArc<std::sync::atomic::AtomicU64>) -> Self {
        Self(storage_pages_limit)
    }

    pub fn consume(&self, amount: u64) -> anyhow::Result<()> {
        self.0
            .fetch_update(
                std::sync::atomic::Ordering::SeqCst,
                std::sync::atomic::Ordering::SeqCst,
                |current| {
                    if amount > current {
                        None
                    } else {
                        Some(current - amount)
                    }
                },
            )
            .map_err(|_| rt::errors::VMError::oos(None))?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
struct StoragePagesOverride(
    rpds::RedBlackTreeMap<PageID, [u8; 32], archery::ArcTK>,
    Limiter,
);

impl StoragePagesOverride {
    fn new(storage_pages_limit: rt::vm::storage::Limiter) -> Self {
        Self(Default::default(), storage_pages_limit)
    }

    fn read_page_override(&self, key: PageID) -> Option<[u8; 32]> {
        self.0.get(&key).cloned()
    }

    fn get(&self, key: PageID) -> Option<[u8; 32]> {
        self.0.get(&key).cloned()
    }

    fn write_page(&mut self, key: PageID, value: [u8; 32]) -> anyhow::Result<()> {
        if !self.0.contains_key(&key) {
            self.1.consume(1)?;
        }
        self.0 = self.0.insert(key, value);

        Ok(())
    }
}

#[derive(Clone)]
pub struct Storage<HS: Send + Sync> {
    pub address: calldata::Address,
    host: HS,
    pages: StoragePagesOverride,
}

impl<HS: Send + Sync> Storage<HS> {
    pub fn new(address: calldata::Address, storage_pages_limit: Limiter, host: HS) -> Self {
        Self {
            address,
            host,
            pages: StoragePagesOverride::new(storage_pages_limit),
        }
    }

    #[inline(always)]
    pub fn read_page_override(&self, key: PageID) -> Option<[u8; 32]> {
        self.pages.read_page_override(key)
    }

    #[inline(always)]
    pub fn write_page(&mut self, key: PageID, value: [u8; 32]) -> anyhow::Result<()> {
        self.pages.write_page(key, value)
    }

    pub fn make_delta(&self) -> Vec<Delta> {
        let mut res = Vec::<Delta>::new();

        for (k, v) in &self.pages.0 {
            if k.1 != 0 {
                let prev_page_id = PageID(k.0, k.1 - 1);
                if self.pages.0.get(&prev_page_id).is_some() {
                    res.last_mut().unwrap().deref_mut().1.extend_from_slice(v);
                    continue;
                }
            }
            res.push(Delta(k.to_bytes(), v.to_vec()));
        }

        res
    }
}

impl<HS: HostStorageLocking + Send + Sync> Storage<HS> {
    pub async fn read(&self, slot_id: SlotID, index: u32, buf: &mut [u8]) -> anyhow::Result<()> {
        if buf.is_empty() {
            return Ok(());
        }

        let start_index = index as usize;
        let end_index = start_index + buf.len();

        // Calculate page range
        let start_page = start_index / 32;
        let end_page = (end_index - 1) / 32;

        // Multi-page case: cut known prefix and suffix
        let mut need_host_read_start = start_index;
        let mut need_host_read_end = end_index;

        // Cut known prefix
        for page_idx in start_page..=end_page {
            let page_id = PageID(slot_id, page_idx as u32);
            if self.pages.get(page_id).is_some() {
                need_host_read_start = (page_idx + 1) * 32;
            } else {
                break;
            }
        }

        // Cut known suffix
        for page_idx in (start_page..=end_page).rev() {
            let page_id = PageID(slot_id, page_idx as u32);
            if self.pages.get(page_id).is_some() {
                need_host_read_end = page_idx * 32;
            } else {
                break;
            }
        }

        // Read from host if needed
        if need_host_read_start < need_host_read_end {
            let host_read_len = need_host_read_end - need_host_read_start;
            let host_buf_offset = need_host_read_start - start_index;
            self.host.lock().await.storage_read(
                slot_id,
                need_host_read_start as u32,
                &mut buf[host_buf_offset..host_buf_offset + host_read_len],
            )?;
        }

        // Apply overrides to the buffer
        for page_idx in start_page..=end_page {
            let page_id = PageID(slot_id, page_idx as u32);
            if let Some(page_data) = self.pages.get(page_id) {
                let page_start_byte = page_idx * 32;
                let page_end_byte = page_start_byte + 32;

                // Calculate overlap between requested range and this page
                let overlap_start = start_index.max(page_start_byte);
                let overlap_end = end_index.min(page_end_byte);

                if overlap_start < overlap_end {
                    let src_offset = overlap_start - page_start_byte;
                    let dst_offset = overlap_start - start_index;
                    let copy_len = overlap_end - overlap_start;

                    buf[dst_offset..dst_offset + copy_len]
                        .copy_from_slice(&page_data[src_offset..src_offset + copy_len]);
                }
            }
        }

        Ok(())
    }

    async fn write_single_page(
        &mut self,
        page_id: PageID,
        offset_in_page: usize,
        buf: &[u8],
    ) -> anyhow::Result<()> {
        let mut page_data = [0u8; 32];

        // Check if this is a full page write
        if offset_in_page == 0 && buf.len() == 32 {
            // Full page write
            page_data.copy_from_slice(buf);
        } else {
            // Partial page write - need existing data first
            if let Some(existing_page) = self.pages.get(page_id) {
                page_data.copy_from_slice(&existing_page);
            } else {
                // Read from host
                let page_start = ((page_id.1 as usize) * 32) as u32;
                self.host
                    .lock()
                    .await
                    .storage_read(page_id.0, page_start, &mut page_data)?;
            }

            // Apply the write data
            page_data[offset_in_page..offset_in_page + buf.len()].copy_from_slice(buf);
        }

        self.write_page(page_id, page_data)?;
        Ok(())
    }

    pub async fn write(&mut self, slot_id: SlotID, index: u32, buf: &[u8]) -> anyhow::Result<()> {
        if buf.is_empty() {
            return Ok(());
        }

        let start_index = index as usize;
        let end_index = start_index + buf.len();

        // Calculate page range
        let start_page = start_index / 32;
        let end_page = (end_index - 1) / 32;

        // Handle single page case
        if start_page == end_page {
            let page_id = PageID(slot_id, start_page as u32);
            let offset_in_page = start_index % 32;
            return self.write_single_page(page_id, offset_in_page, buf).await;
        }

        // Multi-page case

        let first_page_start = start_page * 32;
        let last_page_start = end_page * 32;

        let partial_first = start_index > first_page_start;
        let partial_last = end_index < last_page_start + 32;

        if partial_first || partial_last {
            let mut host_lock = self.host.lock().await;

            if partial_first {
                let page_id = PageID(slot_id, start_page as u32);
                let mut page_data = [0u8; 32];

                if let Some(existing_page) = self.pages.get(page_id) {
                    page_data.copy_from_slice(&existing_page);
                } else {
                    host_lock.storage_read(slot_id, first_page_start as u32, &mut page_data)?;
                }

                let offset_in_page = start_index % 32;
                let copy_len = 32 - offset_in_page;
                page_data[offset_in_page..].copy_from_slice(&buf[..copy_len]);

                self.pages.write_page(page_id, page_data)?;
            }

            if partial_last {
                let page_id = PageID(slot_id, end_page as u32);
                let mut page_data = [0u8; 32];

                if let Some(existing_page) = self.pages.get(page_id) {
                    page_data.copy_from_slice(&existing_page);
                } else {
                    host_lock.storage_read(slot_id, last_page_start as u32, &mut page_data)?;
                }

                let end_offset_in_page = end_index % 32;
                let src_offset = buf.len() - end_offset_in_page;
                page_data[..end_offset_in_page].copy_from_slice(&buf[src_offset..]);

                self.pages.write_page(page_id, page_data)?;
            }
        }

        // Write all middle pages directly
        for page_idx in start_page..=end_page {
            let page_start = page_idx * 32;
            let page_end = page_start + 32;

            // Skip if this is a partial page we already handled
            if (page_idx == start_page && start_index > page_start)
                || (page_idx == end_page && end_index < page_end)
            {
                continue;
            }

            let page_id = PageID(slot_id, page_idx as u32);
            let src_offset = page_start - start_index;
            let mut page_data = [0u8; 32];
            page_data.copy_from_slice(&buf[src_offset..src_offset + 32]);
            self.write_page(page_id, page_data)?;
        }

        Ok(())
    }
}
