use std::sync::Arc;

use crate::rt;
use genlayer_sdk::abi;

/// Builds a numeric expression value.
pub fn num(v: impl Into<num_bigint::BigInt>) -> genvm_common::expr::Value {
    genvm_common::expr::Value::Rational(num_rational::BigRational::from_integer(v.into()))
}

/// Builds a numeric expression value from a `U256`.
pub fn num_u256(v: primitive_types::U256) -> genvm_common::expr::Value {
    let bytes = v.to_big_endian();
    num(num_bigint::BigInt::from_bytes_be(
        num_bigint::Sign::Plus,
        &bytes,
    ))
}

/// Resolver passed to `apply_with` once `node` is baked into the closure:
/// there are no remaining free variables, so any lookup is a bug.
fn no_free_vars(name: &str) -> Result<genvm_common::expr::Value, genvm_common::expr::EvalError> {
    Err(genvm_common::expr::EvalError::UndefinedVariable(
        name.to_owned(),
    ))
}

/// Builds the attrs object passed to a `delta` function.
fn attrs_object(vars: &[(&str, genvm_common::expr::Value)]) -> genvm_common::expr::Value {
    genvm_common::expr::Value::Object(Arc::new(
        vars.iter()
            .map(|(k, v)| ((*k).to_owned(), v.clone()))
            .collect(),
    ))
}

/// Builds the `feeParams` object consumed by the `messageFeeFloor` prelude fn.
pub fn fee_params_value(p: &genvm_common::domain::MessageFeeParams) -> genvm_common::expr::Value {
    use genvm_common::expr::Value;
    let mut m = std::collections::BTreeMap::new();
    m.insert("appealRounds".to_owned(), num_u256(p.appeal_rounds));
    m.insert(
        "leaderTimeunitsAllocation".to_owned(),
        num_u256(p.leader_timeunits_allocation),
    );
    m.insert(
        "validatorTimeunitsAllocation".to_owned(),
        num_u256(p.validator_timeunits_allocation),
    );
    m.insert(
        "executionBudgetPerRound".to_owned(),
        num_u256(p.execution_budget_per_round),
    );
    let rotations: Vec<Value> = p.rotations.iter().map(|r| num_u256(*r)).collect();
    m.insert("rotations".to_owned(), Value::Array(Arc::new(rotations)));
    Value::Object(Arc::new(m))
}

fn value_to_u256(value: genvm_common::expr::Value) -> anyhow::Result<primitive_types::U256> {
    let rational = value
        .into_rational()
        .map_err(|e| anyhow::anyhow!("fee expression must yield a number: {e}"))?;
    anyhow::ensure!(
        rational.is_integer(),
        "fee expression must yield an integer, got {rational}"
    );
    let int = rational.to_integer();
    let (sign, bytes) = int.to_bytes_be();
    anyhow::ensure!(
        sign != num_bigint::Sign::Minus,
        "fee cost must be non-negative"
    );
    anyhow::ensure!(bytes.len() <= 32, "fee cost exceeds U256 range");
    let mut buf = [0u8; 32];
    buf[32 - bytes.len()..].copy_from_slice(&bytes);
    Ok(primitive_types::U256::from_big_endian(&buf))
}

/// Parses `\node = <prelude> <code>` and applies it to `node`, so the result
/// closes over `node` (and the prelude). The `delta` form additionally
/// returns a `\attrs = ...` function ready to be applied per charge.
fn eval_with_node(
    prelude: &str,
    label: &str,
    code: &str,
    node: &genvm_common::expr::Value,
) -> anyhow::Result<genvm_common::expr::Value> {
    let src = format!("\\node = {prelude}\n{code}");
    let lambda = genvm_common::expr::Expr::parse(&src)
        .map_err(|e| anyhow::anyhow!("parsing {label} fee expression `{code}`: {e}"))?
        .evaluate()
        .map_err(|e| anyhow::anyhow!("evaluating {label} fee expression: {e}"))?;
    lambda
        .apply_with(node.clone(), no_free_vars)
        .map_err(|e| anyhow::anyhow!("binding `node` in {label} fee expression: {e}"))
}

/// A fee bucket: total = `subtract_on_start` (charged once, applied above) +
/// Σ `delta(attrs)`. `delta` is a function closing over `node`/the prelude.
struct Bucket {
    bucket_no: u8,
    delta: genvm_common::expr::Value,
}

impl std::fmt::Debug for Bucket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Bucket")
            .field("bucket_no", &self.bucket_no)
            .finish()
    }
}

fn build_bucket(
    prelude: &str,
    cfg: &crate::config::FeesBucketConfig,
    node: &genvm_common::expr::Value,
    bucket_totals: &mut [primitive_types::U256],
) -> anyhow::Result<Bucket> {
    // Up-front cost: a plain number; charge it now.
    let start = value_to_u256(eval_with_node(
        prelude,
        "subtract_on_start",
        &cfg.subtract_on_start_expr,
        node,
    )?)?;
    let num_buckets = bucket_totals.len();
    let total = bucket_totals
        .get_mut(cfg.bucket_no as usize)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "bucket_no {} is out of range (have {} buckets)",
                cfg.bucket_no,
                num_buckets
            )
        })?;
    *total = total.checked_sub(start).ok_or_else(|| {
        rt::errors::VMError(
            abi::consts::VmError::oom().receipt(),
            Some(anyhow::anyhow!(
                "subtract_on_start exceeds bucket {} total",
                cfg.bucket_no
            )),
        )
    })?;

    // Per-change cost: a `\attrs = ...` function.
    let delta = eval_with_node(prelude, "delta", &cfg.delta_expr, node)?;
    Ok(Bucket {
        bucket_no: cfg.bucket_no,
        delta,
    })
}

#[derive(Debug)]
pub struct DataLimit {
    buckets: Vec<tokio::sync::Mutex<primitive_types::U256>>,
    storage: Bucket,
    message_receipt: Bucket,
    nondet_output: Bucket,
    message_fee: Bucket,
}

impl DataLimit {
    pub fn new(
        mut bucket_totals: Vec<primitive_types::U256>,
        fees: crate::config::FeesConfig,
        gas_data: std::collections::BTreeMap<String, String>,
    ) -> anyhow::Result<Self> {
        let prelude = &fees.expr_prelude;

        // Host-provided values are exposed under a single `node` object, e.g.
        // `node.gasPerChangedSlot`.
        let mut node = std::collections::BTreeMap::new();
        for (name, raw) in gas_data {
            let src = format!("{prelude}\n{raw}");
            let value = genvm_common::expr::Expr::parse(&src)
                .map_err(|e| anyhow::anyhow!("parsing gas_data constant `{name}`: {e}"))?
                .evaluate()
                .map_err(|e| anyhow::anyhow!("evaluating gas_data constant `{name}`: {e}"))?;
            node.insert(name, value);
        }
        let node = genvm_common::expr::Value::Object(Arc::new(node));

        let storage = build_bucket(prelude, &fees.storage, &node, &mut bucket_totals)?;
        let message_receipt =
            build_bucket(prelude, &fees.message_receipt, &node, &mut bucket_totals)?;
        let nondet_output = build_bucket(prelude, &fees.nondet_output, &node, &mut bucket_totals)?;
        let message_fee = build_bucket(prelude, &fees.message_fee, &node, &mut bucket_totals)?;

        Ok(Self {
            buckets: bucket_totals
                .into_iter()
                .map(tokio::sync::Mutex::new)
                .collect(),
            storage,
            message_receipt,
            nondet_output,
            message_fee,
        })
    }

    async fn consume_bucket(
        &self,
        bucket: &Bucket,
        vars: &[(&str, genvm_common::expr::Value)],
    ) -> bool {
        let cost = match bucket
            .delta
            .apply_with(attrs_object(vars), no_free_vars)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .and_then(value_to_u256)
        {
            Ok(cost) => cost,
            Err(e) => {
                genvm_common::log_error!(error:ah = e; "failed to evaluate fee expression");
                return false;
            }
        };
        let Some(slot) = self.buckets.get(bucket.bucket_no as usize) else {
            return false;
        };
        let mut remaining = slot.lock().await;
        if *remaining >= cost {
            *remaining -= cost;
            true
        } else {
            false
        }
    }

    pub async fn remaining(&self) -> Vec<primitive_types::U256> {
        let mut result = Vec::with_capacity(self.buckets.len());
        for bucket in &self.buckets {
            result.push(*bucket.lock().await);
        }
        result
    }

    pub async fn consume_storage_pages(&self, pages: u64) -> bool {
        self.consume_bucket(&self.storage, &[("pages", num(pages))])
            .await
    }

    pub async fn consume_message_receipt(
        &self,
        is_internal: bool,
        is_deploy: bool,
        calldata_length: u64,
    ) -> bool {
        self.consume_bucket(
            &self.message_receipt,
            &[
                ("isInternal", num(u64::from(is_internal))),
                ("isDeploy", num(u64::from(is_deploy))),
                ("calldataLength", num(calldata_length)),
            ],
        )
        .await
    }

    pub async fn consume_nondet_output(&self, output_length: u64) -> bool {
        self.consume_bucket(&self.nondet_output, &[("outputLength", num(output_length))])
            .await
    }

    pub async fn consume_message_fee_internal(
        &self,
        on_acceptance: bool,
        matched_fee_params: &genvm_common::domain::MessageFeeParams,
    ) -> bool {
        self.consume_bucket(
            &self.message_fee,
            &[
                ("isInternal", num(1u64)),
                ("onAcceptance", num(u64::from(on_acceptance))),
                ("matchedFeeParams", fee_params_value(matched_fee_params)),
            ],
        )
        .await
    }

    pub async fn consume_message_fee_external(&self) -> bool {
        self.consume_bucket(&self.message_fee, &[("isInternal", num(0u64))])
            .await
    }
}
