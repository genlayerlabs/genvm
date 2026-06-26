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

fn rational_to_u256(rational: num_rational::BigRational) -> anyhow::Result<primitive_types::U256> {
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

fn value_to_u256_vec(
    value: genvm_common::expr::Value,
    bucket_count: usize,
) -> anyhow::Result<Vec<primitive_types::U256>> {
    match value {
        genvm_common::expr::Value::Rational(r) => {
            let cost = rational_to_u256(r)?;
            Ok(vec![cost; bucket_count])
        }
        genvm_common::expr::Value::Array(arr) => {
            anyhow::ensure!(
                arr.len() == bucket_count,
                "fee expression returned array of length {} but bucket_no has {bucket_count} entries",
                arr.len(),
            );
            arr.iter()
                .map(|v| {
                    let r = v
                        .clone()
                        .into_rational()
                        .map_err(|e| anyhow::anyhow!("fee array element must be a number: {e}"))?;
                    rational_to_u256(r)
                })
                .collect()
        }
        other => anyhow::bail!("fee expression must yield a number or array, got {other:?}",),
    }
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

/// A fee bucket: total = `subtract_on_start` (charged once by
/// [`DataLimit::consume_initial`]) + Σ `delta(attrs)`. `delta` is a function
/// closing over `node`/the prelude.
///
/// `bucket_nos` can target multiple on-chain buckets. When the delta expression
/// returns a scalar it is charged identically against every bucket; when it
/// returns an array the lengths must match and each element is charged to the
/// corresponding bucket. All subtractions are atomic (all-or-nothing).
struct Bucket {
    bucket_nos: Vec<u8>,
    subtract_on_start: Vec<primitive_types::U256>,
    delta: genvm_common::expr::Value,
    oom_error: abi::consts::VmError,

    total_consumed: Vec<tokio::sync::Mutex<primitive_types::U256>>,
}

impl std::fmt::Debug for Bucket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Bucket")
            .field("bucket_nos", &self.bucket_nos)
            .field("subtract_on_start", &self.subtract_on_start)
            .finish()
    }
}

fn build_bucket(
    prelude: &str,
    cfg: &crate::config::FeesBucketConfig,
    node: &genvm_common::expr::Value,
    oom_error: abi::consts::VmError,
) -> anyhow::Result<Bucket> {
    let n = cfg.bucket_no.len();
    anyhow::ensure!(n > 0, "bucket_no must have at least one entry");

    let subtract_on_start = value_to_u256_vec(
        eval_with_node(
            prelude,
            "subtract_on_start",
            &cfg.subtract_on_start_expr,
            node,
        )?,
        n,
    )?;

    let delta = eval_with_node(prelude, "delta", &cfg.delta_expr, node)?;
    Ok(Bucket {
        bucket_nos: cfg.bucket_no.clone(),
        subtract_on_start,
        delta,
        oom_error,
        total_consumed: (0..n).map(|_| Default::default()).collect(),
    })
}

/// Per-bucket cost vector returned by `calculate_*` methods.
#[derive(Debug, Clone)]
pub struct CostVec(pub Vec<primitive_types::U256>);

impl CostVec {
    pub fn sum(&self) -> primitive_types::U256 {
        self.0
            .iter()
            .copied()
            .fold(primitive_types::U256::zero(), |a, b| a + b)
    }
}

#[derive(Debug, Clone)]
pub struct MessageFeeConsumption {
    pub message_fee: CostVec,
    pub receipt_fee: CostVec,
}

#[derive(Debug, Clone, Copy, Default, genlayer_calldata::Encode)]
pub struct BucketsConsumed {
    pub storage: primitive_types::U256,
    pub message_receipt: primitive_types::U256,
    pub nondet_output: primitive_types::U256,
    pub message_fee: primitive_types::U256,
    pub event: primitive_types::U256,
}

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
        bucket_totals: Vec<primitive_types::U256>,
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
            abi::consts::VmError::oom().storage(),
        )?;
        let message_receipt = build_bucket(
            prelude,
            &fees.message_receipt,
            &node,
            abi::consts::VmError::oom().receipt().message().internal(),
        )?;
        let nondet_output = build_bucket(
            prelude,
            &fees.nondet_output,
            &node,
            abi::consts::VmError::oom().receipt().nondet_output(),
        )?;
        let message_fee = build_bucket(
            prelude,
            &fees.message_fee,
            &node,
            abi::consts::VmError::oom().fees().internal(),
        )?;
        let event = build_bucket(
            prelude,
            &fees.event,
            &node,
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
    ) -> anyhow::Result<CostVec> {
        match bucket
            .delta
            .apply_with(attrs_object(vars), no_free_vars)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .and_then(|v| value_to_u256_vec(v, bucket.bucket_nos.len()))
        {
            Ok(costs) => Ok(CostVec(costs)),
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
        let costs = self.calculate_bucket(bucket, vars)?;
        Ok(self.consume_bucket_raw(bucket, &costs.0).await)
    }

    async fn consume_bucket_raw(&self, bucket: &Bucket, costs: &[primitive_types::U256]) -> bool {
        let mut buckets = self.buckets.lock().await;
        for (idx, (&bno, &cost)) in bucket.bucket_nos.iter().zip(costs.iter()).enumerate() {
            let Some(remaining) = buckets.get(bno as usize) else {
                log_warn!(bucket = bno; "consume_bucket: bucket index out of range");
                return false;
            };
            if *remaining < cost {
                log_warn!(
                    bucket = bno,
                    cost:display = cost,
                    remaining:display = *remaining;
                    "consume_bucket: insufficient funds"
                );
                return false;
            }
            // when the same bucket_no appears more than once, verify
            // cumulative cost fits
            let mut cumulative = cost;
            for (&prev_bno, &prev_cost) in bucket.bucket_nos[..idx].iter().zip(costs[..idx].iter())
            {
                if prev_bno == bno {
                    cumulative += prev_cost;
                }
            }
            if *remaining < cumulative {
                log_warn!(
                    bucket = bno,
                    cumulative:display = cumulative,
                    remaining:display = *remaining;
                    "consume_bucket: insufficient funds (cumulative)"
                );
                return false;
            }
        }
        for (i, (&bno, &cost)) in bucket.bucket_nos.iter().zip(costs.iter()).enumerate() {
            buckets[bno as usize] -= cost;
            log_debug!(
                bucket = bno,
                cost:display = cost,
                remaining:display = buckets[bno as usize];
                "consume_bucket: ok"
            );
            *bucket.total_consumed[i].lock().await += cost;
        }
        std::mem::drop(buckets);
        true
    }

    pub async fn remaining(&self) -> Vec<primitive_types::U256> {
        self.buckets.lock().await.clone()
    }

    async fn sum_consumed(bucket: &Bucket) -> primitive_types::U256 {
        let mut total = primitive_types::U256::zero();
        for tc in &bucket.total_consumed {
            total += *tc.lock().await;
        }
        total
    }

    pub async fn consumed(&self) -> BucketsConsumed {
        BucketsConsumed {
            storage: Self::sum_consumed(&self.storage).await,
            message_receipt: Self::sum_consumed(&self.message_receipt).await,
            nondet_output: Self::sum_consumed(&self.nondet_output).await,
            message_fee: Self::sum_consumed(&self.message_fee).await,
            event: Self::sum_consumed(&self.event).await,
        }
    }

    /// Charges every bucket's up-front `subtract_on_start`. Must be called once,
    /// before execution. Returns the first bucket's OOM [`rt::errors::VMError`]
    /// whose total cannot cover its initial cost, so the caller can report it to
    /// the host as a receipt; returns `None` when all initials are charged.
    pub async fn consume_initial(&self) -> Option<rt::errors::VMError> {
        for bucket in [
            &self.storage,
            &self.message_receipt,
            &self.nondet_output,
            &self.message_fee,
            &self.event,
        ] {
            if !self
                .consume_bucket_raw(bucket, &bucket.subtract_on_start.clone())
                .await
            {
                return Some(rt::errors::VMError(
                    bucket.oom_error.clone(),
                    Some(anyhow::anyhow!(
                        "subtract_on_start exceeds bucket {:?} total",
                        bucket.bucket_nos
                    )),
                ));
            }
        }
        None
    }

    pub async fn consume_storage_pages(&self, pages: u64) -> anyhow::Result<bool> {
        self.consume_bucket(&self.storage, &[("pages", num(pages))])
            .await
    }

    pub fn calculate_message_receipt(
        &self,
        params: MessageReceiptParams,
    ) -> anyhow::Result<CostVec> {
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
        let costs = self.calculate_bucket(
            &self.event,
            &[
                ("blobSize", num(blob_size)),
                ("topicsCount", num(topics_count)),
            ],
        )?;
        if self.consume_bucket_raw(&self.event, &costs.0).await {
            Ok(Some(costs.sum()))
        } else {
            Ok(None)
        }
    }

    pub fn calculate_message_fee_internal(
        &self,
        on: abi::gl_call::On,
        matched_fee_params: &genvm_common::domain::fees::InternalMessageParams,
    ) -> anyhow::Result<CostVec> {
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

    pub async fn consume_message_fee(&self, cost_fee: &CostVec, cost_receipt: &CostVec) -> bool {
        let mut buckets = self.buckets.lock().await;

        // Build a cumulative deduction map: bucket_index → total to subtract.
        let mut deductions: std::collections::BTreeMap<u8, primitive_types::U256> =
            std::collections::BTreeMap::new();

        for (&bno, &cost) in self.message_fee.bucket_nos.iter().zip(cost_fee.0.iter()) {
            *deductions.entry(bno).or_default() += cost;
        }
        for (&bno, &cost) in self
            .message_receipt
            .bucket_nos
            .iter()
            .zip(cost_receipt.0.iter())
        {
            *deductions.entry(bno).or_default() += cost;
        }

        // Check all buckets first (atomic: all-or-nothing).
        for (&bno, &total) in &deductions {
            let Some(remaining) = buckets.get(bno as usize) else {
                log_warn!(bucket = bno; "consume_message_fee: bucket index out of range");
                return false;
            };
            if *remaining < total {
                log_warn!(
                    bucket = bno,
                    cost:display = total,
                    remaining:display = *remaining;
                    "consume_message_fee: insufficient funds"
                );
                return false;
            }
        }

        // Apply all deductions.
        for (&bno, &total) in &deductions {
            buckets[bno as usize] -= total;
        }
        std::mem::drop(buckets);

        // Update per-category consumed trackers.
        for (i, &cost) in cost_fee.0.iter().enumerate() {
            *self.message_fee.total_consumed[i].lock().await += cost;
        }
        for (i, &cost) in cost_receipt.0.iter().enumerate() {
            *self.message_receipt.total_consumed[i].lock().await += cost;
        }

        log_debug!("consume_message_fee: ok");
        true
    }

    pub fn calculate_message_fee_external(
        &self,
        matched_fee_params: &genvm_common::domain::fees::ExternalMessageParams,
    ) -> anyhow::Result<CostVec> {
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
