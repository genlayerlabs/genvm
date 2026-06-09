use super::Value;

#[derive(Debug, Clone)]
pub struct Raw(pub bytes::Bytes);

#[derive(Debug, Clone)]
pub enum Maybe<T> {
    Materialized(T),
    Checked(Raw),
    CheckedValue(Value),
}
