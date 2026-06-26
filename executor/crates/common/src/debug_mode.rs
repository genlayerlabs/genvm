use serde::{Deserialize, Serialize};

/// How the manager captures a per-execution's logs and stdout/stderr.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Capture {
    /// Nothing is captured: stdout/stderr go to `/dev/null` and logs are routed
    /// to the manager's own log.
    #[default]
    Disabled,
    /// Captured, but bounded: the oldest log entries are dropped past a cap and
    /// stdout/stderr are truncated to a tail.
    Bounded,
    /// Captured in full.
    Unbounded,
}

/// Executor debug level. Levels are ordered and each *adds* to the previous one.
///
/// - `Disabled`: production. No capture, no debug aids.
/// - `Safe`: captures logs and stdout/stderr (bounded). Deterministic.
/// - `SafeUnbounded`: capture is unbounded and `Trace` gl_call output is emitted.
///   Deterministic.
/// - `Unsafe`: also resolves the `:latest` / `:test` runner ids. This is
///   *unsafe across machines* — different nodes may resolve different code — so
///   it can diverge consensus, but a single node stays deterministic.
/// - `UnsafeTracing`: also exposes real wall-clock time to the contract
///   (`RuntimeMicroSec`). This can break determinism even on a single machine.
///   Local debugging only.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Default,
    clap::ValueEnum,
    Serialize,
    Deserialize,
    ::genlayer_calldata::Encode,
    ::genlayer_calldata::Decode,
)]
#[clap(rename_all = "kebab-case")]
#[serde(rename_all = "kebab-case")]
pub enum DebugMode {
    #[default]
    #[calldata(rename = "disabled")]
    Disabled,
    #[calldata(rename = "safe")]
    Safe,
    #[calldata(rename = "safe-unbounded")]
    SafeUnbounded,
    #[calldata(rename = "unsafe")]
    Unsafe,
    #[calldata(rename = "unsafe-tracing")]
    UnsafeTracing,
}

impl DebugMode {
    /// How the manager should capture logs and stdout/stderr.
    pub fn capture(self) -> Capture {
        match self {
            DebugMode::Disabled => Capture::Disabled,
            DebugMode::Safe => Capture::Bounded,
            DebugMode::SafeUnbounded | DebugMode::Unsafe | DebugMode::UnsafeTracing => {
                Capture::Unbounded
            }
        }
    }

    /// `SafeUnbounded`+: emit `Trace` gl_call output instead of skipping it.
    pub fn allows_tracing(self) -> bool {
        self >= DebugMode::SafeUnbounded
    }

    /// `Unsafe`+: the `:latest` / `:test` runner ids may be resolved. Unsafe
    /// across machines (consensus may diverge).
    pub fn allows_latest_resolution(self) -> bool {
        self >= DebugMode::Unsafe
    }

    /// `UnsafeTracing`: exposes non-deterministic data to the contract, such as
    /// real wall-clock time metering. Can break determinism on a single machine.
    pub fn allows_nondeterminism(self) -> bool {
        self >= DebugMode::UnsafeTracing
    }

    /// CLI value passed to the executor's `--debug-mode` flag.
    pub fn as_arg(self) -> &'static str {
        match self {
            DebugMode::Disabled => "disabled",
            DebugMode::Safe => "safe",
            DebugMode::SafeUnbounded => "safe-unbounded",
            DebugMode::Unsafe => "unsafe",
            DebugMode::UnsafeTracing => "unsafe-tracing",
        }
    }
}
