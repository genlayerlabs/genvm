use std::sync::Arc;

type Constants = Arc<std::collections::BTreeMap<String, genvm_common::expr::Value>>;

/// Builds a numeric expression value.
pub fn num(v: impl Into<num_bigint::BigInt>) -> genvm_common::expr::Value {
    genvm_common::expr::Value::Rational(num_rational::BigRational::from_integer(v.into()))
}

/// A fee bucket. Total cost is `subtract_on_start` (charged once, up-front)
/// plus the sum of `delta` evaluations (charged per change). The call site
/// passes whatever variables the expression needs.
/// Both expressions are prelude-prefixed and parsed once at construction.
#[derive(Debug)]
struct BucketExpr {
    bucket_no: u8,
    subtract_on_start: genvm_common::expr::Expr,
    delta: genvm_common::expr::Expr,
}

#[derive(Debug)]
pub struct DataLimit {
    buckets: Vec<tokio::sync::Mutex<primitive_types::U256>>,
    storage: BucketExpr,
    message_receipt: BucketExpr,
    nondet_output: BucketExpr,
    message_fee: BucketExpr,
    /// Host-provided constants, exposed as variables to the fee expressions.
    constants: Constants,
}

fn parse_expr(prelude: &str, label: &str, code: &str) -> anyhow::Result<genvm_common::expr::Expr> {
    genvm_common::expr::Expr::parse(&format!("{prelude}\n{code}"))
        .map_err(|e| anyhow::anyhow!("parsing {label} fee expression `{code}`: {e}"))
}

fn parse_bucket(
    prelude: &str,
    cfg: &crate::config::FeesBucketConfig,
) -> anyhow::Result<BucketExpr> {
    Ok(BucketExpr {
        bucket_no: cfg.bucket_no,
        subtract_on_start: parse_expr(prelude, "subtract_on_start", &cfg.subtract_on_start_expr)?,
        delta: parse_expr(prelude, "delta", &cfg.delta_expr)?,
    })
}

fn value_to_u256(value: genvm_common::expr::Value) -> anyhow::Result<primitive_types::U256> {
    let rational = value
        .into_rational()
        .map_err(|e| anyhow::anyhow!("fee expression must yield a number: {e}"))?;
    let int = rational.floor().to_integer();
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

/// Evaluates a fee expression. `vars` are exposed as variables (shadowing
/// constants of the same name); the up-front cost is evaluated with none.
fn eval_cost(
    expr: &genvm_common::expr::Expr,
    constants: &Constants,
    vars: &[(&str, genvm_common::expr::Value)],
) -> anyhow::Result<primitive_types::U256> {
    let constants = constants.clone();
    let vars: std::collections::BTreeMap<String, genvm_common::expr::Value> = vars
        .iter()
        .map(|(k, v)| ((*k).to_owned(), v.clone()))
        .collect();
    let value = expr
        .evaluate_with(move |name: &str| {
            if let Some(v) = vars.get(name) {
                Ok(v.clone())
            } else if let Some(v) = constants.get(name) {
                Ok(v.clone())
            } else {
                Err(genvm_common::expr::EvalError::UndefinedVariable(
                    name.to_owned(),
                ))
            }
        })
        .map_err(|e| anyhow::anyhow!("evaluating fee expression: {e}"))?;
    value_to_u256(value)
}

impl DataLimit {
    pub fn new(
        mut bucket_totals: Vec<primitive_types::U256>,
        fees: crate::config::FeesConfig,
        gas_data: Option<std::collections::BTreeMap<String, String>>,
    ) -> anyhow::Result<Self> {
        let prelude = &fees.expr_prelude;

        let mut constants = std::collections::BTreeMap::new();
        for (name, raw) in gas_data.unwrap_or_default() {
            let src = format!("{prelude}\n{raw}");
            let value = genvm_common::expr::Expr::parse(&src)
                .map_err(|e| anyhow::anyhow!("parsing gas_data constant `{name}`: {e}"))?
                .evaluate()
                .map_err(|e| anyhow::anyhow!("evaluating gas_data constant `{name}`: {e}"))?;
            constants.insert(name, value);
        }
        let constants: Constants = Arc::new(constants);

        let storage = parse_bucket(prelude, &fees.storage)?;
        let message_receipt = parse_bucket(prelude, &fees.message_receipt)?;
        let nondet_output = parse_bucket(prelude, &fees.nondet_output)?;
        let message_fee = parse_bucket(prelude, &fees.message_fee)?;

        // Charge the fixed, up-front part of every fee kind once.
        for bucket in [&storage, &message_receipt, &nondet_output, &message_fee] {
            let start = eval_cost(&bucket.subtract_on_start, &constants, &[])?;
            if let Some(total) = bucket_totals.get_mut(bucket.bucket_no as usize) {
                *total = total.saturating_sub(start);
            }
        }

        Ok(Self {
            buckets: bucket_totals
                .into_iter()
                .map(tokio::sync::Mutex::new)
                .collect(),
            storage,
            message_receipt,
            nondet_output,
            message_fee,
            constants,
        })
    }

    async fn consume_bucket(
        &self,
        bucket: &BucketExpr,
        vars: &[(&str, genvm_common::expr::Value)],
    ) -> bool {
        let cost = match eval_cost(&bucket.delta, &self.constants, vars) {
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

    pub async fn consume_message_fee(&self) -> bool {
        self.consume_bucket(&self.message_fee, &[]).await
    }
}
