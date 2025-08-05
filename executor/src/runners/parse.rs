use crate::{public_abi, rt};
use genvm_common::*;

fn detect_version_from_wasm(code: &[u8]) -> anyhow::Result<String> {
    let parser = wasmparser::Parser::new(0);

    for payload in parser.parse_all(code) {
        match payload? {
            wasmparser::Payload::CustomSection(section) if section.name() == "genvm.version" => {
                let version = section.data().to_vec();
                if let Ok(version_str) = std::str::from_utf8(&version) {
                    return Ok(version_str.to_string());
                } else {
                    return Err(anyhow::anyhow!("Invalid UTF-8 in version section"));
                }
            }
            _ => {}
        }
    }

    Err(anyhow::anyhow!("version section not found"))
}

pub fn parse(code: util::SharedBytes) -> anyhow::Result<super::Archive> {
    if let Ok(mut as_zip) = zip::ZipArchive::new(std::io::Cursor::new(code.clone())) {
        return super::Archive::from_zip(&mut as_zip, code);
    }

    if wasmparser::Parser::is_core_wasm(code.as_ref()) {
        let version = match detect_version_from_wasm(code.as_ref()) {
            Ok(v) => v,
            Err(e) => {
                log_warn!(default = public_abi::ABSENT_VERSION, error = e; "could not detect version from wasm");
                public_abi::ABSENT_VERSION.to_string()
            }
        };
        return Ok(super::Archive::from_file_and_runner(
            code,
            util::SharedBytes::from(version.as_bytes()),
            util::SharedBytes::from(b"{ \"StartWasm\": \"file\" }".as_ref()),
        ));
    }

    code_to_archive_from_text(code)
}

fn code_to_archive_from_text(code: util::SharedBytes) -> anyhow::Result<super::Archive> {
    let code_str = std::str::from_utf8(code.as_ref()).map_err(|e| {
        rt::errors::VMError(
            format!(
                "{} not_utf8_text",
                public_abi::VmError::InvalidContract.value()
            ),
            Some(anyhow::Error::from(e)),
        )
    })?;

    let code_start = (|| {
        for c in ["//", "#", "--"] {
            if code_str.starts_with(c) {
                return Ok(c);
            }
        }
        Err(rt::errors::VMError(
            format!(
                "{} absent_runner_comment",
                public_abi::VmError::InvalidContract.value()
            ),
            None,
        ))
    })()?;

    let mut version_string = String::new();
    let mut code_comment = String::new();
    let mut first = true;
    for l in code_str.lines() {
        if !l.starts_with(code_start) {
            break;
        }

        let l = &l[code_start.len()..];

        if first {
            first = false;
            if l.trim().starts_with("v") {
                version_string.push_str(l);
            } else {
                log_warn!(default = public_abi::ABSENT_VERSION; "runner comment does not start with version, using default");
                version_string.push_str(public_abi::ABSENT_VERSION);

                code_comment.push_str(l)
            }
        } else {
            code_comment.push_str(l)
        }
    }

    Ok(super::Archive::from_file_and_runner(
        code,
        util::SharedBytes::from(version_string.as_bytes()),
        util::SharedBytes::from(code_comment.as_bytes()),
    ))
}
