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
