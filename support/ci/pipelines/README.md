```mermaid
graph TD
PreCommit -.-> BuildUniversal --> RustTest
PreCommit -.-> PythonTest
PreCommit -.-> BuildAmd64
BuildAmd64 --> NodePipeline
BuildUniversal --> NodePipeline
RustTest -.-> NodePipeline
```
