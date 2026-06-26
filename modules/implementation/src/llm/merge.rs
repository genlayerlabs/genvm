use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub enum MergeStrategy {
    #[default]
    None,
    Replace,
    MergeLeft,
    MergeRight,
    Map(BTreeMap<String, MergeStrategy>),
}

impl Serialize for MergeStrategy {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            MergeStrategy::None => serializer.serialize_str("none"),
            MergeStrategy::Replace => serializer.serialize_str("replace"),
            MergeStrategy::MergeLeft => serializer.serialize_str("merge_left"),
            MergeStrategy::MergeRight => serializer.serialize_str("merge_right"),
            MergeStrategy::Map(map) => map.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for MergeStrategy {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = MergeStrategy;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(
                    f,
                    "\"none\", \"replace\", \"merge_left\", \"merge_right\", or a map"
                )
            }
            fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                Ok(MergeStrategy::None)
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
                match v {
                    "none" => Ok(MergeStrategy::None),
                    "replace" => Ok(MergeStrategy::Replace),
                    "merge_left" => Ok(MergeStrategy::MergeLeft),
                    "merge_right" => Ok(MergeStrategy::MergeRight),
                    _ => Err(E::unknown_variant(
                        v,
                        &["none", "replace", "merge_left", "merge_right"],
                    )),
                }
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                map: A,
            ) -> Result<Self::Value, A::Error> {
                let map = BTreeMap::deserialize(serde::de::value::MapAccessDeserializer::new(map))?;
                Ok(MergeStrategy::Map(map))
            }
        }
        deserializer.deserialize_any(Visitor)
    }
}

pub fn merge_extra(
    request: &mut serde_json::Value,
    extra: serde_json::Value,
    strategy: MergeStrategy,
) -> anyhow::Result<()> {
    merge_values(request, extra, strategy)
}

pub fn merge_values(
    target: &mut serde_json::Value,
    source: serde_json::Value,
    strategy: MergeStrategy,
) -> anyhow::Result<()> {
    match strategy {
        MergeStrategy::None => {
            match (target, source) {
                (serde_json::Value::Object(trg), serde_json::Value::Object(src)) => {
                    for (k, v) in src {
                        match trg.entry(k) {
                            serde_json::map::Entry::Occupied(mut e) => {
                                merge_values(e.get_mut(), v, MergeStrategy::None)?;
                            }
                            serde_json::map::Entry::Vacant(e) => {
                                e.insert(v);
                            }
                        }
                    }
                }
                (serde_json::Value::Array(_), serde_json::Value::Array(_)) => {
                    anyhow::bail!(
                        "none merge strategy cannot be used with arrays; use merge_left/merge_right or replace explicitly"
                    );
                }
                (target, source) => {
                    *target = source;
                }
            }
            Ok(())
        }
        MergeStrategy::Replace => {
            *target = source;
            Ok(())
        }
        MergeStrategy::MergeLeft | MergeStrategy::MergeRight => {
            let left_first = matches!(strategy, MergeStrategy::MergeLeft);
            match (target, source) {
                (serde_json::Value::Array(trg), serde_json::Value::Array(src)) => {
                    if left_first {
                        trg.extend(src);
                    } else {
                        let mut merged = src;
                        merged.append(trg);
                        *trg = merged;
                    }
                    Ok(())
                }
                (serde_json::Value::String(trg), serde_json::Value::String(src)) => {
                    if left_first {
                        trg.push_str(&src);
                    } else {
                        let mut merged = src;
                        merged.push_str(trg);
                        *trg = merged;
                    }
                    Ok(())
                }
                _ => {
                    anyhow::bail!(
                        "merge_left/merge_right require both values to be arrays or strings"
                    )
                }
            }
        }
        MergeStrategy::Map(mut sub) => {
            let Some(target_map) = target.as_object_mut() else {
                anyhow::bail!("map merge strategy requires target to be an object");
            };
            let serde_json::Value::Object(source_map) = source else {
                anyhow::bail!("map merge strategy requires source to be an object");
            };
            for (k, v) in source_map {
                let sub_strategy = sub.remove(&k).unwrap_or_default();
                match target_map.entry(k) {
                    serde_json::map::Entry::Occupied(mut e) => {
                        merge_values(e.get_mut(), v, sub_strategy)?;
                    }
                    serde_json::map::Entry::Vacant(e) => {
                        e.insert(v);
                    }
                }
            }
            Ok(())
        }
    }
}
