use std::sync::Arc;

use anyhow::Context;
use genvm_common::*;

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
pub fn fee_params_value_internal(
    p: &genvm_common::domain::fees::InternalMessageParams,
) -> genvm_common::expr::Value {
    use genvm_common::expr::Value;
    let mut m = std::collections::BTreeMap::new();
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
    // v0.6-dev (CON-549) price caps, exposed so the host-provided fee expression
    // can charge at `maxPriceGenPerTimeUnit` as the funding multiplier and clamp
    // the storage/receipt components, matching the chain's `_calculateRoundFees`.
    m.insert(
        "maxPriceGenPerTimeUnit".to_owned(),
        num_u256(p.max_price_gen_per_time_unit),
    );
    m.insert(
        "storageFeeMaxGasPrice".to_owned(),
        num_u256(p.storage_fee_max_gas_price),
    );
    m.insert(
        "receiptFeeMaxGasPrice".to_owned(),
        num_u256(p.receipt_fee_max_gas_price),
    );
    Value::Object(Arc::new(m))
}

pub fn fee_params_value_external(
    p: &genvm_common::domain::fees::ExternalMessageParams,
) -> genvm_common::expr::Value {
    use genvm_common::expr::Value;
    let mut m = std::collections::BTreeMap::new();
    m.insert("gasLimit".to_owned(), num_u256(p.gas_limit));
    m.insert("maxGasPrice".to_owned(), num_u256(p.max_gas_price));
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
    oom_error: abi::consts::VmError,
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
            oom_error.clone(),
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

#[derive(Debug, Clone, Copy)]
pub struct MessageFeeConsumption {
    pub message_fee: primitive_types::U256,
    pub receipt_fee: primitive_types::U256,
}

/// Inputs to [`DataLimit::calculate_message_receipt`]. Passed as a struct so the
/// several `bool`/`u64` arguments are named at the call site.
#[derive(Debug, Clone, Copy)]
pub struct MessageReceiptParams {
    pub is_internal: bool,
    pub is_deploy: bool,
    pub calldata_length: u64,
    pub code_length: u64,
    pub subtree_length: u64,
}

#[derive(Debug)]
pub struct DataLimit {
    buckets: tokio::sync::Mutex<Vec<primitive_types::U256>>,
    storage: Bucket,
    message_receipt: Bucket,
    nondet_output: Bucket,
    message_fee: Bucket,
    event: Bucket,
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

        let storage = build_bucket(
            prelude,
            &fees.storage,
            &node,
            &mut bucket_totals,
            abi::consts::VmError::oom().storage(),
        )?;
        let message_receipt = build_bucket(
            prelude,
            &fees.message_receipt,
            &node,
            &mut bucket_totals,
            abi::consts::VmError::oom().receipt().message().internal(),
        )?;
        let nondet_output = build_bucket(
            prelude,
            &fees.nondet_output,
            &node,
            &mut bucket_totals,
            abi::consts::VmError::oom().receipt().nondet_output(),
        )?;
        let message_fee = build_bucket(
            prelude,
            &fees.message_fee,
            &node,
            &mut bucket_totals,
            abi::consts::VmError::oom().fees().internal(),
        )?;
        let event = build_bucket(
            prelude,
            &fees.event,
            &node,
            &mut bucket_totals,
            abi::consts::VmError::oom().storage(),
        )?;

        Ok(Self {
            buckets: tokio::sync::Mutex::new(bucket_totals),
            storage,
            message_receipt,
            nondet_output,
            message_fee,
            event,
        })
    }

    fn calculate_bucket(
        &self,
        bucket: &Bucket,
        vars: &[(&str, genvm_common::expr::Value)],
    ) -> anyhow::Result<primitive_types::U256> {
        match bucket
            .delta
            .apply_with(attrs_object(vars), no_free_vars)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .and_then(value_to_u256)
        {
            Ok(cost) => Ok(cost),
            Err(e) => {
                log_error!(error:ah = e; "failed to evaluate fee expression");
                Err(e).context("failed to evaluate fee expression")
            }
        }
    }

    async fn consume_bucket(
        &self,
        bucket: &Bucket,
        vars: &[(&str, genvm_common::expr::Value)],
    ) -> anyhow::Result<bool> {
        let cost = self.calculate_bucket(bucket, vars)?;
        Ok(self.consume_bucket_raw(bucket, cost).await)
    }

    async fn consume_bucket_raw(&self, bucket: &Bucket, cost: primitive_types::U256) -> bool {
        let mut buckets = self.buckets.lock().await;
        let Some(remaining) = buckets.get_mut(bucket.bucket_no as usize) else {
            log_warn!(bucket = bucket.bucket_no; "consume_bucket: bucket index out of range");
            return false;
        };
        if *remaining >= cost {
            *remaining -= cost;
            log_debug!(
                bucket = bucket.bucket_no,
                cost:display = cost,
                remaining:display = *remaining;
                "consume_bucket: ok"
            );
            true
        } else {
            log_warn!(
                bucket = bucket.bucket_no,
                cost:display = cost,
                remaining:display = *remaining;
                "consume_bucket: insufficient funds"
            );
            false
        }
    }

    pub async fn remaining(&self) -> Vec<primitive_types::U256> {
        self.buckets.lock().await.clone()
    }

    pub async fn consume_storage_pages(&self, pages: u64) -> anyhow::Result<bool> {
        self.consume_bucket(&self.storage, &[("pages", num(pages))])
            .await
    }

    pub fn calculate_message_receipt(
        &self,
        params: MessageReceiptParams,
    ) -> anyhow::Result<primitive_types::U256> {
        self.calculate_bucket(
            &self.message_receipt,
            &[
                ("isInternal", params.is_internal.into()),
                ("isDeploy", params.is_deploy.into()),
                ("calldataLength", num(params.calldata_length)),
                ("codeLength", num(params.code_length)),
                ("subtreeLength", num(params.subtree_length)),
            ],
        )
        .context("calculating message receipt")
    }

    pub async fn consume_nondet_output(&self, output_length: u64) -> anyhow::Result<bool> {
        self.consume_bucket(
            &self.nondet_output,
            &[("outputLength", output_length.into())],
        )
        .await
        .context("consuming nondet output")
    }

    pub async fn consume_event(
        &self,
        blob_size: u64,
        topics_count: u64,
    ) -> anyhow::Result<Option<primitive_types::U256>> {
        let cost = self.calculate_bucket(
            &self.event,
            &[
                ("blobSize", num(blob_size)),
                ("topicsCount", num(topics_count)),
            ],
        )?;
        if self.consume_bucket_raw(&self.event, cost).await {
            Ok(Some(cost))
        } else {
            Ok(None)
        }
    }

    pub fn calculate_message_fee_internal(
        &self,
        on: abi::gl_call::On,
        matched_fee_params: &genvm_common::domain::fees::InternalMessageParams,
    ) -> anyhow::Result<primitive_types::U256> {
        self.calculate_bucket(
            &self.message_fee,
            &[
                ("isInternal", true.into()),
                ("onAcceptance", (on == abi::gl_call::On::Accepted).into()),
                (
                    "matchedFeeParams",
                    fee_params_value_internal(matched_fee_params),
                ),
            ],
        )
        .context("calculating message fee internal")
    }

    pub async fn consume_message_fee(
        &self,
        cost_fee: primitive_types::U256,
        cost_receipt: primitive_types::U256,
    ) -> bool {
        if self.message_fee.bucket_no == self.message_receipt.bucket_no {
            let Some(sum) = cost_fee.checked_add(cost_receipt) else {
                log_warn!(
                    cost_fee:display = cost_fee,
                    cost_receipt:display = cost_receipt;
                    "consume_message_fee: overflow adding fee + receipt"
                );
                return false;
            };
            return self.consume_bucket_raw(&self.message_fee, sum).await;
        }

        let mut buckets = self.buckets.lock().await;

        if cost_fee > buckets[self.message_fee.bucket_no as usize] {
            log_warn!(
                bucket = self.message_fee.bucket_no,
                cost_fee:display = cost_fee,
                remaining:display = buckets[self.message_fee.bucket_no as usize];
                "consume_message_fee: insufficient funds for fee"
            );
            return false;
        }

        if cost_receipt > buckets[self.message_receipt.bucket_no as usize] {
            log_warn!(
                bucket = self.message_receipt.bucket_no,
                cost_receipt:display = cost_receipt,
                remaining:display = buckets[self.message_receipt.bucket_no as usize];
                "consume_message_fee: insufficient funds for receipt"
            );
            return false;
        }

        buckets[self.message_fee.bucket_no as usize] -= cost_fee;
        buckets[self.message_receipt.bucket_no as usize] -= cost_receipt;

        log_debug!(
            fee_bucket = self.message_fee.bucket_no,
            cost_fee:display = cost_fee,
            receipt_bucket = self.message_receipt.bucket_no,
            cost_receipt:display = cost_receipt;
            "consume_message_fee: ok"
        );

        true
    }

    pub fn calculate_message_fee_external(
        &self,
        matched_fee_params: &genvm_common::domain::fees::ExternalMessageParams,
    ) -> anyhow::Result<primitive_types::U256> {
        self.calculate_bucket(
            &self.message_fee,
            &[
                ("isInternal", false.into()),
                (
                    "matchedFeeParams",
                    fee_params_value_external(matched_fee_params),
                ),
            ],
        )
        .context("calculating message fee external")
    }
}
