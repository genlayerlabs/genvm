use anyhow::Result;
use regex::Regex;
use std::io::Read;

static CENSOR_RESPONSE: std::sync::LazyLock<Regex> =
    std::sync::LazyLock::new(|| Regex::new(r#""set-cookie": "[^"]*""#).unwrap());

pub fn read_response(res: &mut isahc::Response<isahc::Body>) -> Result<String> {
    let mut res_buf = String::new();
    let status = res.status();
    let mut res_reader = encoding_rs_io::DecodeReaderBytesBuilder::new().build(res.body_mut());
    if status != 200 {
        let _ = res_reader.read_to_string(&mut res_buf);
        return Err(anyhow::anyhow!(
            "can't read response\nresponse: {}\nread:{}",
            CENSOR_RESPONSE.replace_all(&format!("{:?}", res), "\"set-cookie\": \"<censored>\""),
            &res_buf
        ));
    }
    res_reader.read_to_string(&mut res_buf)?;
    Ok(res_buf)
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
