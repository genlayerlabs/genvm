use std::{collections::HashMap, sync::LazyLock};
use anyhow::Result;

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

pub fn patch_str(vars: &HashMap<String, String>, s: &String) -> Result<String> {
    replace_all(&JSON_UNFOLDER_RE, s, |r: &regex::Captures| {
        let r: &str = &r[1];
        vars
            .get(r)
            .ok_or(anyhow::anyhow!("error"))
            .map(|x| x.clone())
    })
}
pub fn patch_value(vars: &HashMap<String, String>, v: serde_json::Value) -> Result<serde_json::Value> {
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
                .map(|(k, v)| -> Result<(String, serde_json::Value)> { Ok((k, patch_value(vars, v)?)) })
                .collect();
            serde_json::Value::Object(serde_json::Map::from_iter(res?.into_iter()))
        }
        x => x,
    })
}
