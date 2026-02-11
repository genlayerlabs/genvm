use std::sync::Arc;

pub struct ExecutionContext {
    pub hello: Arc<genvm_modules_interfaces::GenVMHello>,
    pub llm: Option<LlmSubContext>,
    pub web: Option<WebSubContext>,
}

pub struct LlmSubContext {
    pub scripting: crate::scripting::CtxPart,
    pub module: crate::llm::ctx::CtxPart,
}

pub struct WebSubContext {
    pub scripting: crate::scripting::CtxPart,
}

impl ExecutionContext {
    pub fn collect_metrics(&self) -> serde_json::Value {
        let llm = self
            .llm
            .as_ref()
            .map(|ctx| serde_json::to_value(&*ctx.module.metrics).unwrap_or_default());
        let web = self
            .web
            .as_ref()
            .map(|ctx| serde_json::to_value(&*ctx.scripting.metrics).unwrap_or_default());
        serde_json::json!({
            "llm": llm,
            "web": web,
        })
    }
}

impl crate::common::WithGenVMId for ExecutionContext {
    fn genvm_id(&self) -> genvm_modules_interfaces::GenVMId {
        self.hello.genvm_id
    }
}

impl crate::common::WithGenVMId for LlmSubContext {
    fn genvm_id(&self) -> genvm_modules_interfaces::GenVMId {
        self.scripting.hello.genvm_id
    }
}

impl crate::common::WithGenVMId for WebSubContext {
    fn genvm_id(&self) -> genvm_modules_interfaces::GenVMId {
        self.scripting.hello.genvm_id
    }
}
