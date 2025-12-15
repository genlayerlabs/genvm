use arbitrary::Arbitrary;
use genvm::{rt::vm::storage::HostStorageLocking, SlotID};
use genvm_common::{calldata, sync};
use std::collections::BTreeMap;
use std::sync::Arc;

// Import the storage module we want to fuzz
use genvm::rt::vm::storage::{HostStorage, PageID, Storage};

#[derive(Debug, Clone)]
struct MockHostStorage {
    storage: BTreeMap<PageID, [u8; 32]>,
}

impl MockHostStorage {
    fn new() -> Self {
        Self {
            storage: BTreeMap::new(),
        }
    }

    fn write_storage_slice(
        &mut self,
        slot_id: SlotID,
        index: u32,
        data: &[u8],
    ) -> anyhow::Result<()> {
        if index as u64 + data.len() as u64 > u32::MAX as u64 + 1 {
            panic!("bounds check failed: {} {}", index, data.len());
        }

        let mut remaining_data = data;
        let mut current_index = index;

        while !remaining_data.is_empty() {
            let page_index = current_index / 32;
            let offset_in_page = current_index % 32;
            let bytes_to_write = std::cmp::min(32 - offset_in_page as usize, remaining_data.len());

            // Create page key: slot_id + page_index
            let page_key = PageID(slot_id, page_index);

            // Get existing page or create new zero-filled page
            let mut page = self.storage.get(&page_key).copied().unwrap_or([0u8; 32]);

            // Write data to the correct offset within the page
            let start_offset = offset_in_page as usize;
            let end_offset = start_offset + bytes_to_write;
            page[start_offset..end_offset].copy_from_slice(&remaining_data[..bytes_to_write]);

            // Store the modified page
            self.storage.insert(page_key, page);

            // Move to next chunk
            remaining_data = &remaining_data[bytes_to_write..];
            current_index = current_index.wrapping_add(bytes_to_write as u32);
        }

        Ok(())
    }
}

#[derive(Clone)]
struct MockHostStorageHolder(Arc<tokio::sync::Mutex<MockHostStorage>>);

impl HostStorageLocking for MockHostStorageHolder {
    type ReturnType<'a> = tokio::sync::MutexGuard<'a, MockHostStorage>;

    async fn lock<'a>(&'a self) -> Self::ReturnType<'a> {
        self.0.lock().await
    }
}

impl HostStorage for MockHostStorage {
    fn storage_read(&mut self, slot_id: SlotID, index: u32, buf: &mut [u8]) -> anyhow::Result<()> {
        if index as u64 + buf.len() as u64 > u32::MAX as u64 + 1 {
            panic!("bounds check failed: {} {}", index, buf.len());
        }

        let mut remaining_data = buf;
        let mut current_index = index;

        while !remaining_data.is_empty() {
            let page_index = current_index / 32;
            let offset_in_page = current_index % 32;
            let bytes_to_read = std::cmp::min(32 - offset_in_page as usize, remaining_data.len());

            // Create page key: slot_id + page_index
            let page_key = PageID(slot_id, page_index);

            // Get page or use zero-filled page if not found
            let page = self.storage.get(&page_key).copied().unwrap_or([0u8; 32]);

            // Read data from the correct offset within the page
            let start_offset = offset_in_page as usize;
            let end_offset = start_offset + bytes_to_read;
            remaining_data[..bytes_to_read].copy_from_slice(&page[start_offset..end_offset]);

            // Move to next chunk
            remaining_data = &mut remaining_data[bytes_to_read..];
            current_index = current_index.wrapping_add(bytes_to_read as u32);
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Arbitrary)]
enum StorageOp {
    Write {
        slot_id: [u8; 32],
        index: u32,
        data: Vec<u8>,
    },
    Read {
        slot_id: [u8; 32],
        index: u32,
        len: u8, // Limit to reasonable size
    },
}

#[derive(Debug, Clone, Arbitrary)]
struct FuzzInput {
    initial_data: Vec<(SlotID, u32, Vec<u8>)>,
    operations: Vec<StorageOp>,
}

async fn run_storage_fuzz(input: FuzzInput) -> anyhow::Result<()> {
    let address = calldata::Address::zero();
    let mut mock_host = MockHostStorage::new();

    // Initialize host storage with initial data
    for (slot_id, index, data) in &input.initial_data {
        if data.len() > u32::MAX as usize {
            panic!("Data length too large for write operation");
        }
        let last_index = index.saturating_add(data.len() as u32);
        let new_len = (last_index - index) as usize;
        let data = &data[..new_len];

        mock_host.write_storage_slice(*slot_id, *index, data)?;
    }

    let mut reference_host = mock_host.clone();

    let host = MockHostStorageHolder(Arc::new(tokio::sync::Mutex::new(mock_host)));
    let mut storage = Storage::new(
        address,
        genvm::rt::vm::storage::Limiter::new(sync::DArc::new(u64::MAX.into())),
        host.clone(),
    );

    // Apply operations and verify consistency
    for op in input.operations {
        println!("Executing operation: {:?}", op);
        match op {
            StorageOp::Write {
                slot_id,
                index,
                data,
            } => {
                let slot_id = SlotID::from(slot_id);

                if data.len() > u32::MAX as usize {
                    panic!("Data length too large for write operation");
                }
                let last_index = index.saturating_add(data.len() as u32);
                let new_len = (last_index - index) as usize;
                let data = &data[..new_len];

                // Apply to storage
                reference_host.write_storage_slice(slot_id, index, data)?;
                storage.write(slot_id, index, data).await?;
            }
            StorageOp::Read {
                slot_id,
                index,
                len,
            } => {
                let slot_id = SlotID::from(slot_id);
                let len = len as usize;

                let last_index = index.saturating_add(len as u32);
                let len = (last_index - index) as usize;

                let mut reference_buf = vec![0u8; len];
                reference_host.storage_read(slot_id, index, &mut reference_buf)?;

                let mut storage_buf = vec![0u8; len];
                storage.read(slot_id, index, &mut storage_buf).await?;

                if storage_buf != reference_buf {
                    panic!(
                        "Storage read mismatch at slot={:?}, index={}, len={}\nstorage: {:?}\nreference: {:?}",
                        slot_id, index, len, storage_buf, reference_buf
                    );
                }
            }
        }
    }

    // Final consistency check: read random ranges and verify
    for (slot_id, base_index, _) in &input.initial_data {
        for offset in 0..32u32 {
            for len in [1, 2, 4, 8, 16, 32, 64, 128, 256].iter().cloned() {
                let index = base_index.saturating_add(offset);
                let last_index = index.saturating_add(len as u32);
                let len = (last_index - index) as usize;

                let mut storage_buf = vec![0u8; len];
                let mut reference_buf = vec![0u8; len];

                storage.read(*slot_id, index, &mut storage_buf).await?;
                reference_host.storage_read(*slot_id, index, &mut reference_buf)?;

                if storage_buf != reference_buf {
                    panic!(
                        "Final consistency check failed at slot={:?}, index={}, len={}\nstorage: {:?}\nreference: {:?}",
                        slot_id, index, len, storage_buf, reference_buf
                    );
                }
            }
        }
    }

    Ok(())
}

fn run_fuzz(input: FuzzInput) -> anyhow::Result<()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    rt.block_on(run_storage_fuzz(input))?;

    println!("Storage fuzz test completed successfully.");

    Ok(())
}

fn main() {
    afl::fuzz!(|data: FuzzInput| {
        if let Err(err) = run_fuzz(data) {
            eprintln!("Fuzz error: {:?}", err);
            panic!("Storage fuzz test failed: {}", err);
        }
    });
}
