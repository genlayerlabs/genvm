use anyhow::Context;

use crate::rt;
use genvm_common::*;

impl super::Supervisor {
    pub fn validate_wasm(&self, wasm: &[u8]) -> anyhow::Result<()> {
        use wasmparser::*;

        let add_features = WasmFeatures::REFERENCE_TYPES.bits() | WasmFeatures::FLOATS.bits();

        let det_features = self.engines.det.config().get_features().bits() | add_features;

        let non_det_features = self.engines.non_det.config().get_features().bits() | add_features;

        let mut det_validator = wasmparser::Validator::new_with_features(
            WasmFeatures::from_bits(det_features).unwrap(),
        );
        let mut non_det_validator = wasmparser::Validator::new_with_features(
            WasmFeatures::from_bits(non_det_features).unwrap(),
        );
        det_validator.validate_all(wasm).with_context(|| {
            format!(
                "validating {}",
                &String::from_utf8_lossy(&wasm[..10.min(wasm.len())])
            )
        })?;
        non_det_validator.validate_all(wasm).with_context(|| {
            format!(
                "validating {}",
                &String::from_utf8_lossy(&wasm[..10.min(wasm.len())])
            )
        })?;

        Ok(())
    }

    pub async fn compile_wasm(
        &self,
        wasm: &[u8],
        debug_path: &str,
    ) -> anyhow::Result<rt::DetNondet<wasmtime::Module>> {
        log_debug!(path = debug_path; "compilation");
        self.shared_data
            .metrics
            .supervisor
            .compiled_modules
            .increment();
        let tok = stats::tracker::Time::new(
            self.shared_data
                .gep(|x| &x.metrics.supervisor.compilation_time),
        );

        self.validate_wasm(wasm)?;

        let start_time = std::time::Instant::now();
        let module_det = wasmtime::CodeBuilder::new(&self.engines.det)
            .wasm_binary(
                std::borrow::Cow::Borrowed(wasm),
                Some(std::path::Path::new(debug_path)),
            )?
            .compile_module()?;

        let module_non_det = wasmtime::CodeBuilder::new(&self.engines.non_det)
            .wasm_binary(
                std::borrow::Cow::Borrowed(wasm),
                Some(std::path::Path::new(debug_path)),
            )?
            .compile_module()?;
        log_info!(status = "done", duration:? = start_time.elapsed(), path = debug_path; "cache compiling");

        std::mem::drop(tok);
        Ok(rt::DetNondet {
            det: module_det,
            non_det: module_non_det,
        })
    }
}
