mod mmap;
pub mod str;

use crate::sync::DArc;

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
pub struct SharedBytes(DArc<[u8]>);

impl std::fmt::Debug for SharedBytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SharedBytes")
            .field("data", &self.as_ref())
            .finish()
    }
}

impl std::cmp::PartialEq for SharedBytes {
    fn eq(&self, other: &Self) -> bool {
        *self.0 == *other.0
    }
}

impl std::cmp::Eq for SharedBytes {}

impl std::hash::Hash for SharedBytes {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (*self.0).hash(state);
    }
}

impl AsRef<[u8]> for SharedBytes {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<&[u8]> for SharedBytes {
    fn from(value: &[u8]) -> Self {
        let data: Box<[u8]> = Box::from(value);
        Self(DArc::new(data).gep(|x| x.as_ref()))
    }
}

impl SharedBytes {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    pub fn new(value: impl AsRef<[u8]>) -> Self {
        let data: Box<[u8]> = Box::from(value.as_ref());
        Self(DArc::new(data).gep(|x| x.as_ref()))
    }

    pub fn slice(&self, begin: usize, end: usize) -> SharedBytes {
        if begin > end {
            panic!("SharedBytes::slice: begin ({}) > end ({})", begin, end);
        }
        if begin > self.0.len() || end > self.0.len() {
            panic!(
                "SharedBytes::slice: range [{}..{}] out of bounds for length {}",
                begin,
                end,
                self.0.len()
            );
        }
        Self(self.0.gep(|data| &data[begin..end]))
    }
}
