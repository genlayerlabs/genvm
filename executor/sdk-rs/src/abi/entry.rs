use crate::abi::consts::EntryKind;
use crate::calldata::{Address, Value};
use serde::{Deserialize, Serialize};

fn entry_kind_as_int<S>(data: &EntryKind, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    s.serialize_u8(*data as u8)
}

fn default_datetime() -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339("2024-11-26T06:42:42.424242Z")
        .unwrap()
        .to_utc()
}

/// Core message data that represents the transaction context.
/// This is the minimal set of information needed to process a contract call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageData {
    pub contract_address: Address,
    pub sender_address: Address,
    pub origin_address: Address,
    pub chain_id: std::sync::Arc<str>,
    pub value: Option<u64>,
    pub is_init: bool,
    #[serde(default = "default_datetime")]
    pub datetime: chrono::DateTime<chrono::Utc>,
}

#[cfg(feature = "arbitrary")]
impl<'a> arbitrary::Arbitrary<'a> for MessageData {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        use arbitrary::Arbitrary;

        let ts = u32::arbitrary(u)?;
        let Some(datetime) = chrono::DateTime::<chrono::Utc>::from_timestamp_secs(ts as i64) else {
            return Err(arbitrary::Error::NotEnoughData);
        };

        let chain_id_bytes: [u8; 32] = Arbitrary::arbitrary(u)?;
        let chain_id = primitive_types::U256::from_big_endian(&chain_id_bytes);

        Ok(Self {
            contract_address: Arbitrary::arbitrary(u)?,
            sender_address: Arbitrary::arbitrary(u)?,
            origin_address: Arbitrary::arbitrary(u)?,
            chain_id: std::sync::Arc::from(chain_id.to_string()),
            value: Option::<u64>::arbitrary(u)?,
            is_init: bool::arbitrary(u)?,
            datetime,
        })
    }
}

/// Extended message that includes entry point information.
/// This is the full message passed to WebAssembly contracts via stdin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtendedMessage {
    pub contract_address: Address,
    pub sender_address: Address,
    pub origin_address: Address,
    /// View methods call chain.
    /// It is empty for entrypoint (refer to [`contract_address`])
    pub stack: Vec<Address>,

    pub chain_id: num_bigint::BigInt,
    pub value: num_bigint::BigInt,
    pub is_init: bool,
    /// Transaction timestamp
    pub datetime: chrono::DateTime<chrono::Utc>,

    #[serde(serialize_with = "entry_kind_as_int")]
    pub entry_kind: EntryKind,
    #[serde(with = "serde_bytes")]
    pub entry_data: Vec<u8>,

    pub entry_stage_data: Value,
}

impl ExtendedMessage {
    /// Get the entry point data based on the entry kind
    pub fn get_entry_point_data(&self) -> EntryPointData<'_> {
        match self.entry_kind {
            EntryKind::Main => EntryPointData::Main(MainEntryData {
                data: &self.entry_data,
            }),
            EntryKind::Sandbox => EntryPointData::Sandbox(SandboxEntryData {
                data: &self.entry_data,
            }),
            EntryKind::ConsensusStage => EntryPointData::ConsensusStage(ConsensusStageEntryData {
                data: &self.entry_data,
                stage_data: &self.entry_stage_data,
            }),
        }
    }
}

/// Entry data for Main entry kind - regular contract method calls
#[derive(Debug, Clone)]
pub struct MainEntryData<'a> {
    /// Method call information encoded as calldata
    pub data: &'a [u8],
}

/// Entry data for Sandbox entry kind - contract decides how to handle
#[derive(Debug, Clone)]
pub struct SandboxEntryData<'a> {
    /// Arbitrary payload for the contract to handle
    pub data: &'a [u8],
}

/// Entry data for ConsensusStage entry kind - validator consensus functions
#[derive(Debug, Clone)]
pub struct ConsensusStageEntryData<'a> {
    /// Entry data payload
    pub data: &'a [u8],
    /// Consensus stage data:
    /// - `null` for leader nodes
    /// - `{leaders_result: <calldata>}` for validator nodes
    pub stage_data: &'a Value,
}

/// Enum representing the entry point data for different entry kinds
#[derive(Debug)]
pub enum EntryPointData<'a> {
    Main(MainEntryData<'a>),
    Sandbox(SandboxEntryData<'a>),
    ConsensusStage(ConsensusStageEntryData<'a>),
}

/// Trait for handling different entry kinds in a WASI context.
/// Implement this trait to define how your contract handles each entry type.
#[cfg(feature = "wasi")]
pub trait EntryHandler {
    /// The result type for entry handling
    type Output;
    /// The error type for entry handling
    type Error;

    /// Handle a Main entry - regular contract method calls.
    /// The entry_data contains method call information as described in contract call conventions.
    fn handle_main(&mut self, entry_data: &[u8]) -> Result<Self::Output, Self::Error>;

    /// Handle a Sandbox entry - contract decides how to handle the payload.
    /// The entry_data contains arbitrary data for the contract to process.
    fn handle_sandbox(&mut self, entry_data: &[u8]) -> Result<Self::Output, Self::Error>;

    /// Handle a ConsensusStage entry - validator consensus functions.
    /// - For leader nodes: stage_data is null
    /// - For validator nodes: stage_data contains {leaders_result: <calldata>}
    fn handle_consensus_stage(
        &mut self,
        entry_data: &[u8],
        stage_data: &Value,
    ) -> Result<Self::Output, Self::Error>;

    /// Dispatch to the appropriate handler based on entry kind.
    fn dispatch(&mut self, message: &ExtendedMessage) -> Result<Self::Output, Self::Error> {
        match message.entry_kind {
            EntryKind::Main => self.handle_main(&message.entry_data),
            EntryKind::Sandbox => self.handle_sandbox(&message.entry_data),
            EntryKind::ConsensusStage => {
                self.handle_consensus_stage(&message.entry_data, &message.entry_stage_data)
            }
        }
    }
}
