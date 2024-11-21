use anyhow::Result;
use std::{
    collections::{BTreeMap, HashMap},
    sync::LazyLock,
};

static JSON_UNFOLDER_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r#"\$\{([a-zA-Z0-9_]*)\}"#).unwrap());

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

pub fn patch_str(vars: &impl AnyMap<String, String>, s: &str) -> Result<String> {
    replace_all(&JSON_UNFOLDER_RE, s, |r: &regex::Captures| {
        let r: &str = &r[1];
        vars.get_from_map(r)
            .ok_or(anyhow::anyhow!("error"))
            .map(|x| x.clone())
    })
}
pub fn patch_value(
    vars: &HashMap<String, String>,
    v: serde_json::Value,
) -> Result<serde_json::Value> {
    Ok(match v {
        serde_json::Value::String(s) => serde_json::Value::String(patch_str(vars, &s)?),
        serde_json::Value::Array(a) => {
            let res: Result<Vec<serde_json::Value>, _> =
                a.into_iter().map(|a| patch_value(vars, a)).collect();
            serde_json::Value::Array(res?)
        }
        serde_json::Value::Object(ob) => {
            let res: Result<Vec<(String, serde_json::Value)>, _> = ob
                .into_iter()
                .map(|(k, v)| -> Result<(String, serde_json::Value)> {
                    Ok((k, patch_value(vars, v)?))
                })
                .collect();
            serde_json::Value::Object(serde_json::Map::from_iter(res?.into_iter()))
        }
        x => x,
    })
}
