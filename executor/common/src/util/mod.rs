mod mmap;
pub mod str;

use std::sync::Arc;

pub use mmap::mmap_file;

struct GlobalSymbolDeserializeVisitor;

impl serde::de::Visitor<'_> for GlobalSymbolDeserializeVisitor {
    type Value = symbol_table::GlobalSymbol;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("expected string")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(symbol_table::GlobalSymbol::from(value))
    }
}

pub fn global_symbol_deserialize<'de, D>(d: D) -> Result<symbol_table::GlobalSymbol, D::Error>
where
    D: serde::Deserializer<'de>,
{
    d.deserialize_str(GlobalSymbolDeserializeVisitor)
}

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

    pub fn as_slice(&self) -> &[u8] {
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
