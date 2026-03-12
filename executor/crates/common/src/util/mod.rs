mod mmap;
pub mod str;

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
