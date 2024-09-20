use std::io::Read;

use anyhow::Result;

pub fn read(res: &mut isahc::Response<isahc::Body>) -> Result<String> {
    let mut res_buf = String::new();
    let status = res.status();
    let mut res_reader = encoding_rs_io::DecodeReaderBytesBuilder::new()
        .build(res.body_mut());
    if status != 200 {
        let _ =res_reader.read_to_string(&mut res_buf);
        return Err(anyhow::anyhow!("can't get webpage contents {:?}\n{}", res, &res_buf));
    }
    res_reader.read_to_string(&mut res_buf)?;
    Ok(res_buf)
}
