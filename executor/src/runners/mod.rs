pub mod actions;
pub mod cache;

mod parse;
mod ustar;
use genvm_common::*;
use itertools::Itertools;

pub use ustar::Archive;

pub use parse::parse;

pub use actions::*;

use anyhow::Context as _;
use std::{str::FromStr as _, sync::Arc};

use crate::rt;

pub fn append_runner_subpath(id: &str, hash: &str, path: &mut std::path::PathBuf) {
    path.push(id);
    path.push(&hash[..2]);
    path.push(&hash[2..]);
}

/// A parsed runner id. Runner ids come in the following forms:
///
/// - `name:hash` — a packaged runner identified by its human readable name and
///   content hash (in debug mode `hash` may also be `test`/`latest`).
/// - `contract` — the runner of the contract that is currently being executed.
/// - `chain:<address>:<a|f>:<slot>` — read the runner code blob from a storage
///   slot of an arbitrary contract. `address` is a `0x`-prefixed 20 byte hex
///   address, `a`/`f` selects accepted (latest non final) / finalized state and
///   `slot` is a `0x`-prefixed 32 byte hex slot id.
/// - `custom:<hash>` — a runner registered at runtime, looked up by its hash.
pub enum RunnerId {
    NameHash {
        name: symbol_table::GlobalSymbol,
        hash: symbol_table::GlobalSymbol,
    },
    Contract,
    Chain {
        address: calldata::Address,
        on: crate::public_abi::StorageType,
        slot: crate::SlotID,
    },
    Custom {
        hash: symbol_table::GlobalSymbol,
    },
}

fn is_hash_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '='
}

fn parse_hex_fixed<const N: usize>(s: &str) -> Option<[u8; N]> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    if s.len() != N * 2 {
        return None;
    }
    let mut out = [0u8; N];
    hex::decode_to_slice(s, &mut out).ok()?;
    Some(out)
}

pub fn parse_runner_id(id: &str) -> Option<RunnerId> {
    if id == "contract" {
        return Some(RunnerId::Contract);
    }

    if let Some(rest) = id.strip_prefix("chain:") {
        let mut it = rest.split(':');
        let address = it.next()?;
        let on = it.next()?;
        let slot = it.next()?;
        if it.next().is_some() {
            return None;
        }

        let address = calldata::Address::from(parse_hex_fixed::<20>(address)?);
        let on = match on {
            "a" => crate::public_abi::StorageType::LatestNonFinal,
            "f" => crate::public_abi::StorageType::LatestFinal,
            _ => return None,
        };
        let slot = crate::SlotID::from_bytes(parse_hex_fixed::<32>(slot)?);

        return Some(RunnerId::Chain { address, on, slot });
    }

    if let Some(rest) = id.strip_prefix("custom:") {
        if rest.is_empty() || !rest.chars().all(is_hash_char) {
            return None;
        }
        return Some(RunnerId::Custom {
            hash: symbol_table::GlobalSymbol::new(rest),
        });
    }

    let (name, hash) = id.split_once(':')?;
    if name.is_empty() || hash.is_empty() {
        return None;
    }
    for c in name.chars() {
        if !c.is_ascii_alphanumeric() && c != '-' && c != '_' {
            log_warn!("character `{c}` is not allowed in runner id");
            return None;
        }
    }
    if !hash.chars().all(is_hash_char) {
        return None;
    }
    Some(RunnerId::NameHash {
        name: symbol_table::GlobalSymbol::new(name),
        hash: symbol_table::GlobalSymbol::new(hash),
    })
}

/// Canonical textual form of a [`RunnerId::Chain`]. Used as the cache key so
/// that an explicit `chain:` reference and the current `contract` resolve to the
/// same entry when they point at the same address/state/slot.
pub fn chain_canonical(
    address: calldata::Address,
    on: crate::public_abi::StorageType,
    slot: crate::SlotID,
) -> String {
    let on = match on {
        crate::public_abi::StorageType::LatestFinal => 'f',
        _ => 'a',
    };
    format!(
        "chain:0x{}:{}:0x{}",
        hex::encode(address.raw()),
        on,
        hex::encode(slot.raw())
    )
}

/// Derives the hash component of a `custom:<hash>` runner id from its code.
pub fn custom_runner_hash(code: &[u8]) -> symbol_table::GlobalSymbol {
    use sha3::Digest as _;
    let mut digest = sha3::Sha3_256::new();
    digest.update(code);
    symbol_table::GlobalSymbol::from(hex::encode(digest.finalize()))
}

/// Builds the canonical `custom:<hash>` runner id from its hash.
pub fn custom_runner_id(hash: symbol_table::GlobalSymbol) -> symbol_table::GlobalSymbol {
    let mut id = String::from("custom:");
    id.push_str(hash.as_str());
    symbol_table::GlobalSymbol::from(id)
}

pub fn get_runner_of_contract(
    address: calldata::Address,
    state: crate::public_abi::StorageType,
) -> symbol_table::GlobalSymbol {
    // This builds the cache key only; the canonical points at the default `code`
    // slot and intentionally ignores a `code_slot` redirect. That is sound only
    // because this is called exclusively for the *currently executing* contract,
    // whose actual code is read separately via `Storage::read_code` (which DOES
    // honor `code_slot`). Resolving another contract's `contract` runner this way
    // would read the wrong slot — use an explicit `chain:` id for that.
    let slot = crate::SlotID::ZERO.indirection(crate::public_abi::root_offsets::CODE);
    symbol_table::GlobalSymbol::from(chain_canonical(address, state, slot))
}

pub struct ArchiveCache {
    pub(super) id: symbol_table::GlobalSymbol,
    pub(super) files: Archive,
    pub(super) actions: tokio::sync::OnceCell<Arc<InitAction>>,
}

impl ArchiveCache {
    pub fn runner_id(&self) -> symbol_table::GlobalSymbol {
        self.id
    }

    pub fn new(id: symbol_table::GlobalSymbol, files: Archive) -> Self {
        Self {
            id,
            files,
            actions: tokio::sync::OnceCell::new(),
        }
    }

    pub fn get_version(&self) -> anyhow::Result<genvm_common::version::Version> {
        let contents = match self.get_file("version") {
            Ok(contents) => contents,
            Err(e) => {
                log_warn!(error:ah = e, runner = self.id; "failed to read version file for runner, using default");
                bytes::Bytes::copy_from_slice(host_fns::CURRENT_MAJOR_STR.as_bytes())
            }
        };

        let contents = std::str::from_utf8(contents.as_ref())
            .with_context(|| format!("casting version to string {}", self.id))?;

        let version = version::Version::from_str(contents).with_context(|| {
            format!(
                "parsing version '{}' for runner {}",
                contents.trim(),
                self.id
            )
        })?;

        log_trace!(from = contents, to = version; "version parsed");

        Ok(version)
    }

    pub async fn get_actions(&self) -> anyhow::Result<Arc<InitAction>> {
        self.actions
            .get_or_try_init(|| async {
                let contents = self.get_file("runner.json")?;

                let contents_str = std::str::from_utf8(contents.as_ref()).with_context(|| {
                    format!("runner.json is not valid UTF-8 for runner {}", self.id)
                })?;

                let as_init: InitAction = serde_json::from_str(contents_str)
                    .with_context(|| format!("parsing runner.json for runner {}", self.id))?;

                Ok(Arc::new(as_init))
            })
            .await
            .map(Clone::clone)
    }

    pub fn get_file(&self, name: &str) -> anyhow::Result<bytes::Bytes> {
        let contents = self
            .files
            .data
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("no file {}", name))
            .with_context(|| format!("reading runner {}", self.id))?;
        Ok(contents.clone())
    }
}

pub fn verify_runner(runner_id: &str) -> Option<(&str, &str)> {
    let (runner_id, runner_hash) = runner_id.split(":").collect_tuple()?;

    for c in runner_id.chars() {
        if !c.is_ascii_alphanumeric() && c != '-' && c != '_' {
            log_warn!("character `{c}` is not allowed in runner id");

            return None;
        }
    }

    for c in runner_hash.chars() {
        if !c.is_ascii_alphanumeric() && c != '-' && c != '_' && c != '=' {
            log_warn!("character `{c}` is not allowed in runner hash");

            return None;
        }
    }
    Some((runner_id, runner_hash))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr40() -> String {
        format!("0x{}1", "0".repeat(39))
    }
    fn slot64() -> String {
        format!("0x{}2", "0".repeat(63))
    }

    #[test]
    fn parse_runner_id_forms() {
        assert!(matches!(
            parse_runner_id("contract"),
            Some(RunnerId::Contract)
        ));
        assert!(matches!(
            parse_runner_id("py-genlayer:test"),
            Some(RunnerId::NameHash { .. })
        ));
        assert!(matches!(
            parse_runner_id("custom:deadbeef"),
            Some(RunnerId::Custom { .. })
        ));

        match parse_runner_id(&format!("chain:{}:f:{}", addr40(), slot64())) {
            Some(RunnerId::Chain { on, .. }) => {
                assert_eq!(on, crate::public_abi::StorageType::LatestFinal)
            }
            _ => panic!("expected a chain runner id"),
        }
        match parse_runner_id(&format!("chain:{}:a:{}", addr40(), slot64())) {
            Some(RunnerId::Chain { on, .. }) => {
                assert_eq!(on, crate::public_abi::StorageType::LatestNonFinal)
            }
            _ => panic!("expected a chain runner id"),
        }
    }

    #[test]
    fn parse_runner_id_rejects_malformed() {
        assert!(parse_runner_id("custom:").is_none()); // empty hash
        assert!(parse_runner_id("custom:bad/char").is_none()); // invalid char
        assert!(parse_runner_id(":hash").is_none()); // empty name
        assert!(parse_runner_id("name:").is_none()); // empty hash
        assert!(parse_runner_id("with space:hash").is_none()); // invalid name char

        // chain: closed grammar
        assert!(parse_runner_id(&format!("chain:0x01:f:{}", slot64())).is_none()); // short address
        assert!(parse_runner_id(&format!("chain:{}:f:0x02", addr40())).is_none()); // short slot
        assert!(parse_runner_id(&format!("chain:{}:x:{}", addr40(), slot64())).is_none()); // bad a|f
        assert!(parse_runner_id(&format!("chain:{}:f:{}:extra", addr40(), slot64())).is_none()); // extra segment
        assert!(parse_runner_id(&format!("chain:0x{}:f:{}", "z".repeat(40), slot64())).is_none());
        // non-hex address
    }
}
