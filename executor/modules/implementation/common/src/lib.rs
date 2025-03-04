use std::{future::Future, pin::Pin};

use anyhow::Result;
use regex::Regex;
use serde::Deserialize;

pub static CENSOR_RESPONSE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r#""(set-cookie|cf-ray|access-control[^"]*)": "[^"]*""#).unwrap()
});

pub static DURATION_REGEXP: std::sync::LazyLock<Regex> =
    std::sync::LazyLock::new(|| Regex::new(r#"^(\d)+(m?s)$"#).unwrap());

pub struct ParsedDuration(pub tokio::time::Duration);

struct ParsedDurationVisitor;

impl serde::de::Visitor<'_> for ParsedDurationVisitor {
    type Value = ParsedDuration;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("expected string | null")
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(ParsedDuration(tokio::time::Duration::ZERO))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let re = regex::Regex::new(r#"^(\d+)(m?s)$"#).unwrap();
        let caps = re
            .captures(value)
            .ok_or(E::custom("invalid duration format"))?;

        let int_str = caps.get(1).unwrap().as_str();

        let int = u64::from_str_radix(int_str, 10).map_err(E::custom)?;

        match caps.get(2).unwrap().as_str() {
            "s" => Ok(ParsedDuration(tokio::time::Duration::from_secs(int))),
            "ms" => Ok(ParsedDuration(tokio::time::Duration::from_millis(int))),
            _ => Err(E::custom("invalid duration suffix")),
        }
    }
}

impl<'de> Deserialize<'de> for ParsedDuration {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(ParsedDurationVisitor)
    }
}

pub async fn read_response(res: reqwest::Response) -> Result<String> {
    let status = res.status();
    if status != 200 {
        let debug = format!("{:?}", &res);
        let text = res.text().await?;
        return Err(anyhow::anyhow!(
            "can't read response\nresponse: {}\nbody: {}",
            CENSOR_RESPONSE.replace_all(&debug, "\"<censored>\": \"<censored>\""),
            &text
        ));
    }
    Ok(res.text().await?)
}

pub fn make_error_recoverable<T, E>(
    res: Result<T, E>,
    message: &'static str,
) -> genvm_modules_interfaces::ModuleResult<T>
where
    E: std::fmt::Debug,
{
    res.map_err(|e| {
        log::error!(original:? = e, mapped = message; "recoverable module error");
        genvm_modules_interfaces::ModuleError::Recoverable(message)
    })
}

pub trait SessionDrop
where
    Self: Sized,
{
    fn has_drop_session() -> bool {
        false
    }

    fn drop_session(
        _client: reqwest::Client,
        _data: &mut Self,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>> {
        Box::pin(async {})
    }
}

pub struct Session<T: SessionDrop> {
    pub client: reqwest::Client,
    pub data: T,
}

impl<T: SessionDrop> std::ops::Drop for Session<T> {
    fn drop(&mut self) {
        if !T::has_drop_session() {
            return;
        }
        tokio::spawn(T::drop_session(self.client.clone(), &mut self.data));
    }
}

impl<T: SessionDrop> Session<T> {
    pub fn new(data: T) -> Self {
        Session {
            client: reqwest::ClientBuilder::new()
                .cookie_store(true)
                .gzip(true)
                .build()
                .unwrap(),
            data,
        }
    }
}

pub struct SessionPool<T: SessionDrop> {
    pool: crossbeam::queue::ArrayQueue<Box<Session<T>>>,
}

impl<T: SessionDrop> SessionPool<T> {
    pub fn new() -> Self {
        Self {
            pool: crossbeam::queue::ArrayQueue::new(8),
        }
    }

    pub fn get(&self) -> Option<Box<Session<T>>> {
        self.pool.pop()
    }

    pub fn retn(&self, obj: Box<Session<T>>) {
        let _ = self.pool.push(obj);
    }
}

impl SessionDrop for () {}
