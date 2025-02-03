use std::{collections::BTreeMap, io::Read, sync::Arc};

#[derive(Clone)]
pub struct SharedBytes {
    bytes: Arc<dyn AsRef<[u8]> + Sync + Send>,
    begin: usize,
    end: usize,
}

impl std::fmt::Debug for SharedBytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SharedBytes")
            .field("data", &self.as_ref())
            .finish()
    }
}

impl std::cmp::PartialEq for SharedBytes {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl std::cmp::Eq for SharedBytes {}

impl std::hash::Hash for SharedBytes {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_ref().hash(state);
    }
}

impl AsRef<[u8]> for SharedBytes {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl From<&[u8]> for SharedBytes {
    fn from(value: &[u8]) -> Self {
        let data: Box<[u8]> = Box::from(value);
        Self {
            begin: 0,
            end: value.len(),
            bytes: Arc::new(data),
        }
    }
}

impl SharedBytes {
    pub fn len(&self) -> usize {
        self.end - self.begin
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn as_slice(&self) -> &[u8] {
        let as_slice: &[u8] = (*self.bytes).as_ref();
        &as_slice[self.begin..self.end]
    }

    pub fn new(value: impl AsRef<[u8]> + Sync + Send + 'static) -> Self {
        let vl: &[u8] = value.as_ref();
        let len = vl.len();
        Self {
            begin: 0,
            end: len,
            bytes: Arc::new(value),
        }
    }

    pub fn slice(&self, begin: usize, end: usize) -> SharedBytes {
        if begin > end {
            panic!("INVALID");
        }
        if self.begin + begin > self.end {
            panic!("INVALID");
        }

        if self.begin + end > self.end {
            panic!("INVALID");
        }
        Self {
            bytes: self.bytes.clone(),
            begin: self.begin + begin,
            end: usize::min(self.begin + end, self.end),
        }
    }
}

pub struct Archive {
    pub data: BTreeMap<String, SharedBytes>,
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
    pub fn from_ustar(original_data: SharedBytes) -> anyhow::Result<Self> {
        const BLOCK_SIZE: usize = 512;
        const _RECORD_SIZE: usize = BLOCK_SIZE * 20;

        if original_data.len() < BLOCK_SIZE * 2 {
            anyhow::bail!("archive is too short for tar")
        }

        if original_data.len() % BLOCK_SIZE != 0 {
            anyhow::bail!("tar len % 512 != 0")
        }

        let mut res = BTreeMap::new();

        let mut begin = 0;
        while begin + 2 * BLOCK_SIZE <= original_data.len() {
            let data = original_data.slice(begin, original_data.len());
            let header = data.slice(0, BLOCK_SIZE);

            if data
                .slice(0, BLOCK_SIZE * 2)
                .as_ref()
                .iter()
                .all(|x| *x == 0)
            {
                break;
            }

            let header_signature = &header.as_ref()[257..265];

            if header_signature != b"ustar\x0000" {
                anyhow::bail!(
                    "invalid ustar header={:?}; offset={}",
                    header_signature,
                    begin
                )
            }

            let file_size_octal = trim_zeroes(&header.as_ref()[124..136]);

            let link_indicator = header.as_ref()[156];
            if link_indicator != b'0' && link_indicator != b'\x00' {
                anyhow::bail!("links are forbidden")
            }

            let path_and_name = trim_zeroes(&header.as_ref()[0..100]);
            let path_and_name_prefix = trim_zeroes(&header.as_ref()[345..345 + 155]);

            begin += BLOCK_SIZE;

            let mut name_vec = Vec::from(path_and_name_prefix);
            name_vec.extend_from_slice(path_and_name);

            let name = String::from_utf8(name_vec)?;

            if name.ends_with("/") {
                continue;
            }

            let mut file_size = 0 as usize;
            for c in file_size_octal.iter().cloned() {
                if !(b'0'..=b'7').contains(&c) {
                    anyhow::bail!("invalid octal ascii {}", c)
                }
                file_size = file_size * 8 + (c - b'0') as usize;
            }

            begin += file_size;
            begin += (BLOCK_SIZE - (begin % BLOCK_SIZE)) % BLOCK_SIZE;

            let file_contents = data.slice(BLOCK_SIZE, BLOCK_SIZE + file_size);

            map_try_insert(&mut res, name, file_contents)?;
        }

        Ok(Self { data: res })
    }

    pub fn from_zip<R: std::io::Read + std::io::Seek>(
        zip: &mut zip::ZipArchive<R>,
    ) -> anyhow::Result<Self> {
        let mut res = BTreeMap::new();

        for i in 0..zip.len() {
            let mut file = zip.by_index(i)?;

            let mut buf = Vec::new();
            file.read_to_end(&mut buf)?;

            map_try_insert(
                &mut res,
                String::from(file.name()),
                SharedBytes::from(buf.as_slice()),
            )?;
        }

        Ok(Self { data: res })
    }

    pub fn from_file_and_runner(file: SharedBytes, runner_comment: SharedBytes) -> Self {
        Self {
            data: BTreeMap::from_iter([
                ("runner.json".into(), runner_comment),
                ("file".into(), file),
            ]),
        }
    }
}
