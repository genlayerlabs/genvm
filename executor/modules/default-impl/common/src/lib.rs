use anyhow::Result;
use regex::Regex;
use std::io::Read;

pub fn run_with_termination<F>(f: F, timeout_handle: *mut u32) -> Option<F::Output>
where
    F: core::future::Future + Send,
    F::Output: Send,
{
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let should_quit = unsafe { std::sync::atomic::AtomicU32::from_ptr(timeout_handle) };

    let selector = async {
        let tracker_fut = async {
            while should_quit.load(std::sync::atomic::Ordering::SeqCst) == 0 {
                tokio::time::sleep(tokio::time::Duration::new(0, 1_000_000)).await;
            }
        };
        let waiter = tokio::spawn(tracker_fut);

        loop {
            tokio::select! {
                val = f => return Some(val),
                _ = waiter => {
                    return None
                },
            }
        }
    };

    let a = rt.block_on(selector);
    rt.shutdown_background();
    a
}

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
