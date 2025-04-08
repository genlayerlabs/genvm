use anyhow::Result;
use std::{
    collections::{BTreeMap, HashMap},
    sync::LazyLock,
};

pub static DOLLAR_UNFOLDER_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r#"\$\{([a-zA-Z0-9_]*|ENV\[[a-zA-Z0-9_]*\])\}"#).unwrap());

pub static HASH_UNFOLDER_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r#"#\{([a-zA-Z0-9_]*)\}"#).unwrap());

fn replace_all<E>(
    re: &regex::Regex,
    haystack: &str,
    replacement: impl Fn(&regex::Captures) -> Result<String, E>,
) -> Result<String, E> {
    let mut new = String::with_capacity(haystack.len());
    let mut last_match = 0;
    for caps in re.captures_iter(haystack) {
        let m = caps.get(0).unwrap();
        new.push_str(&haystack[last_match..m.start()]);
        new.push_str(&replacement(&caps)?);
        last_match = m.end();
    }
    new.push_str(&haystack[last_match..]);
    Ok(new)
}

pub trait AnyMap<K, V> {
    fn get_from_map<Q>(&self, k: &Q) -> Option<&V>
    where
        Q: ?Sized,
        K: std::borrow::Borrow<Q> + Ord,
        Q: Ord + std::hash::Hash + Eq;
}

impl<K: Eq + std::hash::Hash, V> AnyMap<K, V> for HashMap<K, V> {
    fn get_from_map<Q>(&self, k: &Q) -> Option<&V>
    where
        Q: ?Sized,
        K: std::borrow::Borrow<Q>,
        Q: Ord + std::hash::Hash + Eq,
    {
        let zelf: &HashMap<K, V> = self;
        zelf.get::<Q>(k)
    }
}

impl<K: std::cmp::Ord, V> AnyMap<K, V> for BTreeMap<K, V> {
    fn get_from_map<Q>(&self, k: &Q) -> Option<&V>
    where
        Q: ?Sized,
        K: std::borrow::Borrow<Q> + Ord,
        Q: Ord + std::hash::Hash + Eq,
    {
        let zelf: &BTreeMap<K, V> = self;
        zelf.get::<Q>(k)
    }
}

pub fn patch_str(vars: &impl AnyMap<String, String>, s: &str, re: &regex::Regex) -> Result<String> {
    replace_all(re, s, |r: &regex::Captures| {
        let r: &str = &r[1];
        vars.get_from_map(r)
            .map(|s| s.as_str())
            .or_else(|| {
                if r.starts_with("ENV[") {
                    Some("")
                } else {
                    None
                }
            })
            .ok_or_else(|| anyhow::anyhow!("unknown variable `{r}`"))
            .map(String::from)
    })
}

pub fn patch_yaml(
    vars: &HashMap<String, String>,
    v: serde_yaml::Value,
    re: &regex::Regex,
) -> Result<serde_yaml::Value> {
    Ok(match v {
        serde_yaml::Value::String(s) => serde_yaml::Value::String(patch_str(vars, &s, re)?),
        serde_yaml::Value::Sequence(a) => {
            let res: Result<Vec<serde_yaml::Value>, _> =
                a.into_iter().map(|a| patch_yaml(vars, a, re)).collect();
            serde_yaml::Value::Sequence(res?)
        }
        serde_yaml::Value::Mapping(ob) => {
            let res: Result<Vec<(serde_yaml::Value, serde_yaml::Value)>, _> = ob
                .into_iter()
                .map(|(k, v)| -> Result<(serde_yaml::Value, serde_yaml::Value)> {
                    Ok((patch_yaml(vars, k, re)?, patch_yaml(vars, v, re)?))
                })
                .collect();
            serde_yaml::Value::Mapping(serde_yaml::Mapping::from_iter(res?))
        }
        serde_yaml::Value::Tagged(t) => {
            serde_yaml::Value::Tagged(Box::new(serde_yaml::value::TaggedValue {
                tag: t.tag.clone(),
                value: patch_yaml(vars, t.value, re)?,
            }))
        }
        x => x,
    })
}

pub fn patch_json(
    vars: &HashMap<String, String>,
    v: serde_json::Value,
    re: &regex::Regex,
) -> Result<serde_json::Value> {
    Ok(match v {
        serde_json::Value::String(s) => serde_json::Value::String(patch_str(vars, &s, re)?),
        serde_json::Value::Array(a) => {
            let res: Result<Vec<serde_json::Value>, _> =
                a.into_iter().map(|a| patch_json(vars, a, re)).collect();
            serde_json::Value::Array(res?)
        }
        serde_json::Value::Object(ob) => {
            let res: Result<Vec<(String, serde_json::Value)>, _> = ob
                .into_iter()
                .map(|(k, v)| -> Result<(String, serde_json::Value)> {
                    Ok((k, patch_json(vars, v, re)?))
                })
                .collect();
            serde_json::Value::Object(serde_json::Map::from_iter(res?))
        }
        x => x,
    })
}
