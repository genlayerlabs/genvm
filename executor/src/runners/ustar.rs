use std::collections::BTreeMap;

#[derive(Clone)]
pub struct Archive {
    pub data: BTreeMap<String, bytes::Bytes>,
    pub total_size: u32,
}

fn map_try_insert<K, V>(map: &mut BTreeMap<K, V>, key: K, value: V) -> anyhow::Result<&mut V>
where
    K: Ord + std::fmt::Display,
{
    use std::collections::btree_map::Entry::*;
    match map.entry(key) {
        Occupied(entry) => Err(anyhow::anyhow!("entry {} is already occupied", entry.key())),
        Vacant(entry) => Ok(entry.insert(value)),
    }
}

fn trim_zeroes(x: &[u8]) -> &[u8] {
    let mut idx = x.len() - 1;
    while idx > 0 && x[idx - 1] == 0 {
        idx -= 1;
    }

    &x[0..idx]
}

impl Archive {
    pub fn from_ustar(original_data: bytes::Bytes) -> anyhow::Result<Self> {
        const BLOCK_SIZE: usize = 512;
        const _RECORD_SIZE: usize = BLOCK_SIZE * 20;

        if original_data.len() < BLOCK_SIZE * 2 {
            anyhow::bail!("archive is too short for tar")
        }

        if !original_data.len().is_multiple_of(BLOCK_SIZE) {
            anyhow::bail!("tar len % 512 != 0")
        }

        let mut res = BTreeMap::new();

        let mut begin = 0;
        while begin + 2 * BLOCK_SIZE <= original_data.len() {
            let data = original_data.slice(begin..original_data.len());
            let header = data.slice(0..BLOCK_SIZE);

            if data.slice(0..BLOCK_SIZE * 2).iter().all(|x| *x == 0) {
                break;
            }

            let header_signature = &header[257..265];

            if header_signature != b"ustar\x0000" {
                anyhow::bail!(
                    "invalid ustar header={:?}; offset={}",
                    header_signature,
                    begin
                )
            }

            let file_size_octal = trim_zeroes(&header[124..136]);

            let link_indicator = header[156];
            if ![b'0', b'\x00', b'5'].contains(&link_indicator) {
                anyhow::bail!("links are forbidden")
            }

            let path_and_name = trim_zeroes(&header[0..100]);
            let path_and_name_prefix = trim_zeroes(&header[345..345 + 155]);

            begin += BLOCK_SIZE;

            let mut name_vec = Vec::from(path_and_name_prefix);
            name_vec.extend_from_slice(path_and_name);

            let name = String::from_utf8(name_vec)?;

            if name.ends_with("/") {
                continue;
            }

            let mut file_size = 0_usize;
            for c in file_size_octal.iter().cloned() {
                if !(b'0'..=b'7').contains(&c) {
                    anyhow::bail!("invalid octal ascii {}", c)
                }
                file_size = file_size
                    .checked_mul(8)
                    .and_then(|v| v.checked_add((c - b'0') as usize))
                    .ok_or_else(|| anyhow::anyhow!("tar file size overflow"))?;
            }

            begin += file_size;
            begin += (BLOCK_SIZE - (begin % BLOCK_SIZE)) % BLOCK_SIZE;

            let file_end = BLOCK_SIZE
                .checked_add(file_size)
                .filter(|end| *end <= data.len())
                .ok_or_else(|| anyhow::anyhow!("tar file size exceeds archive bounds"))?;
            let file_contents = data.slice(BLOCK_SIZE..file_end);

            map_try_insert(&mut res, name, file_contents)?;
        }

        Ok(Self {
            data: res,
            total_size: original_data.len() as u32,
        })
    }

    pub fn from_zip<R: std::io::Read + std::io::Seek>(
        zip: &mut zip::ZipArchive<R>,
        bytes: bytes::Bytes,
    ) -> anyhow::Result<Self> {
        let mut res = BTreeMap::new();

        for i in 0..zip.len() {
            let file = zip.by_index(i)?;

            if file.compression() != zip::CompressionMethod::Stored {
                anyhow::bail!("unsupported compression method: {:?}", file.compression());
            }

            let start_index = file.data_start();
            let compressed_size = file.compressed_size();
            let end_index = start_index.checked_add(compressed_size).ok_or_else(|| {
                anyhow::anyhow!(
                    "file {} data_start={} compressed_size={} overflow",
                    file.name(),
                    start_index,
                    compressed_size
                )
            })?;
            if end_index > bytes.len() as u64
                || start_index > bytes.len() as u64
                || end_index < start_index
            {
                anyhow::bail!(
                    "file {} data_start={} compressed_size={} end_index={} bytes_len={}",
                    file.name(),
                    start_index,
                    compressed_size,
                    end_index,
                    bytes.len()
                );
            }
            let buf = bytes.slice(start_index as usize..end_index as usize);

            map_try_insert(&mut res, String::from(file.name()), buf)?;
        }

        Ok(Self {
            data: res,
            total_size: bytes.len() as u32,
        })
    }

    pub fn from_file_and_runner(
        file: bytes::Bytes,
        version: bytes::Bytes,
        runner_comment: bytes::Bytes,
    ) -> Self {
        let total_size = file.len() as u32;

        Self {
            data: BTreeMap::from_iter([
                ("runner.json".into(), runner_comment),
                ("version".into(), version),
                ("file".into(), file),
            ]),
            total_size,
        }
    }
}
