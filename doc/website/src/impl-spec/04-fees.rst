Fees, Buckets and Expressions
=============================

GenVM charges for run-time resources through a small, operator-defined expression
language. The configuration is consumed by ``executor/src/rt/fees.rs`` and the
expression evaluator lives in ``executor/crates/common/src/expr/``.

Three independent moving parts are involved:

#. **Buckets** — opaque integer reservoirs (``U256``) that GenVM debits during
   execution. Their initial totals come from the host (typically the consensus
   layer) on a per-call basis.
#. **Bucket configs** — operator-provided rules that bind a *named* charge (storage
   pages, message receipts, nondet output bytes, message fees) to a *bucket number*
   and an expression that computes the per-event cost.
#. **Expressions** — a small typed lambda calculus used to write the cost rules. It
   is evaluated once at startup (for ``subtract_on_start_expr``) and once per
   charge (for ``delta_expr``).

Configuration
-------------

The ``fees`` block of the GenVM config (see ``doc/schemas/default-config.json``,
``genvm-fees-conf``) has the following shape::

   "fees": {
     "expr_prelude": "<expression>",
     "storage":         { "bucket_no": N, "subtract_on_start_expr": "...", "delta_expr": "..." },
     "message_receipt": { "bucket_no": N, "subtract_on_start_expr": "...", "delta_expr": "..." },
     "nondet_output":   { "bucket_no": N, "subtract_on_start_expr": "...", "delta_expr": "..." },
     "message_fee":     { "bucket_no": N, "subtract_on_start_expr": "...", "delta_expr": "..." }
   }

``expr_prelude``
   Shared expression text prepended to every bucket expression before parsing. The
   intended use is shared ``let`` bindings (or Y-combinator definitions) that the
   per-bucket expressions can reference.

``bucket_no``
   Either a single index, or an array of indices, into the ``Vec<U256>`` of bucket
   totals the host passes to ``DataLimit::new``. With an array, a scalar ``delta_expr``
   result is charged against every listed bucket, while an array result is charged
   element-wise (lengths must match); all debits in one charge are atomic
   (all-or-nothing). Two bucket configs MAY share the same index; in that case both
   charge against the same reservoir. The ``message_fee`` / ``message_receipt`` pair
   has special atomic-debit behaviour when they share a bucket (``consume_message_fee``
   in ``fees.rs:332``).

``subtract_on_start_expr``
   A bare numeric expression. Evaluated once at startup with ``node`` bound to the
   host-provided gas constants (see below). The resulting ``U256`` is debited from
   the bucket immediately. Defaults to ``"0"`` if omitted.

``delta_expr``
   A function ``\attrs = body`` (one or more ``\`` lambdas) whose ``attrs``
   parameter is an object containing the variables that the charge depends on.
   Evaluated once per charge; the resulting integer is debited from the bucket.

The four named buckets and their ``attrs`` shape are:

==================== ======================================================================
Name                 ``attrs`` keys
==================== ======================================================================
``storage``          ``pages``: number of pages allocated
``message_receipt``  ``isInternal``, ``isDeploy``, ``calldataLength``, ``codeLength``
``nondet_output``    ``outputLength``
``message_fee``      ``isInternal``, ``onAcceptance`` (when internal),
                     ``matchedFeeParams``: an object with
                     ``leaderTimeunitsAllocation``,
                     ``validatorTimeunitsAllocation``, ``executionBudgetPerRound``,
                     ``rotations`` (array)
==================== ======================================================================

The ground truth for these names is ``fees.rs`` (search for ``calculate_bucket``
call sites).

The Expression Language
-----------------------

Source files: ``executor/crates/common/src/expr/``.

The language is a pure, call-by-need lambda calculus over rationals, booleans,
strings, arrays and (sorted) objects. It is **deliberately small** and runs without
fuel or recursion limits — the comment in ``evaluator.rs:8`` reads:

   ``SAFETY: this evaluator has no recursion depth or fuel limits. It is only used
   for trusted, operator-supplied fee config expressions, never for
   contract-supplied or user-supplied input.``

Operators MUST NOT expose this surface to contract or end-user input.

Syntax
~~~~~~

::

   expr   ::= let NAME = expr in expr
            | if expr then expr else expr
            | \NAME [NAME...] = expr            -- lambda; multi-param is sugar
            | comparison
   comparison ::= sum (('<' | '>' | '<=' | '>=' | '==' | '!=') sum)?
   sum    ::= product (('+' | '-') product)*
   product ::= unary (('*' | '/') unary)*
   unary  ::= '-' unary | application
   application ::= primary (primary)*
   primary ::= NUMBER | NAME | STRING | array | object | '(' expr ')' | primary '.' NAME
   array  ::= '[' (expr (',' expr)*)? ']'
   object ::= '{' (NAME '=' expr ';')* '}'
   STRING ::= '"' ( char | '\(' expr ')' )* '"'   -- supports interpolation

``let`` is **non-recursive**: the body of ``let f = ... in ...`` cannot reference
``f``. Recursion is expressed with a Y combinator, e.g.::

   let Y = \f = (\x = f (x x)) (\x = f (x x)) in
   let fact = Y (\rec n = if n <= 1 then 1 else n * rec (n - 1)) in
   fact 5

Strings support ``\(expr)`` interpolation; ``toString`` and ``\(..)`` use the same
rendering.

Types
~~~~~

- ``number`` — arbitrary-precision rational (``BigRational``). Integer division of
  two rationals yields a rational; use ``floor`` before converting back to ``U256``
  if you need truncation.
- ``bool``
- ``string`` (``Arc<str>``)
- ``array`` (``Arc<Vec<Value>>``)
- ``object`` (sorted ``BTreeMap<String, Value>``, accessed via ``.`` or ``hasKey``)
- ``function`` (host or guest, both call-by-need)

Type errors are reported eagerly; division by zero is a runtime error.

Builtins
~~~~~~~~

The following names are predefined by the evaluator (``resolve_builtin`` in
``evaluator.rs``):

- ``floor n`` — floor of a rational.
- ``toString v`` — render any value as a string.
- ``hasKey obj k`` — whether the object has the given key.
- ``arrayLen a``, ``arrayGetElem a i`` — array length and indexing.
- ``pow base exp`` — integer-exponent power (negative exponents take the reciprocal;
  ``pow 0 (-n)`` is a division-by-zero error).

Free Variables and ``node``
~~~~~~~~~~~~~~~~~~~~~~~~~~~

A free variable not satisfied by the surrounding ``let`` bindings or the builtin
table is looked up via the host-provided ``get_var``. In the fee evaluator the only
free variable allowed at the top level is ``node``: an object built from the
``gas_data`` map handed to ``DataLimit::new``. Each ``(name, raw)`` pair in
``gas_data`` is itself evaluated as an expression (against ``expr_prelude``) and the
result is inserted into ``node`` under ``name``.

Bucket expressions reference these as e.g. ``node.gasPerChangedSlot`` and the
node-provided values let consensus parameters drift independently of the executor
binary.

The closing of ``node`` happens at startup:
``\node = <prelude> <code>`` is parsed and applied to the resolved ``node`` object.
The returned value (a number for ``subtract_on_start_expr``, a function for
``delta_expr``) closes over ``node`` and has **no remaining free variables**; any
later lookup at charge time is a configuration bug and is reported as
``UndefinedVariable``.

Evaluation Model
~~~~~~~~~~~~~~~~

- **Lazy.** Argument thunks are forced on first use and memoised
  (``Thunk::force`` in ``value.rs``). A self-referential force returns
  ``"infinite recursion while forcing a lazy value"`` rather than deadlocking.
- **Pure.** No I/O, no mutation, no clock access. The only nondeterminism the
  evaluator can produce is the result of host functions, which in the fee surface
  is a no-op (``no_free_vars`` is the resolver).

Charge Lifecycle
----------------

Per process, per non-batched call:

#. The host invokes ``DataLimit::new(bucket_totals, fees_cfg, gas_data)``.
#. The evaluator parses ``expr_prelude + gas_data[i].value`` for every ``gas_data``
   entry and builds the ``node`` object. Parse or evaluation errors here surface as
   ``parsing gas_data constant 'X'`` / ``evaluating gas_data constant 'X'``.
#. For each of the four buckets the evaluator builds ``\node = <prelude> <code>``
   for ``subtract_on_start_expr`` and ``delta_expr``, applies the ``node`` object,
   and stores the resulting ``Value`` (a rational and a function respectively).
#. The startup cost is debited immediately. Underflow surfaces as the bucket's
   configured OOM ``VmError`` (see the ``VmError::oom().*`` enum chain at the call
   site).
#. During the run, every ``consume_*`` function in ``fees.rs`` packs the per-event
   ``attrs`` into an object, applies the bucket's ``delta`` function, converts the
   result to ``U256`` (rejecting non-integers and negatives), and debits the
   reservoir.

Bucket totals are mutated under a single ``tokio::sync::Mutex<Vec<U256>>`` so the
debits are sequentially consistent across concurrent sub-VMs. ``message_fee`` and
``message_receipt`` debits are atomic with respect to each other: when they share a
``bucket_no``, the costs are summed and debited once; otherwise both reservoirs
are checked before either is decremented. This avoids partial charges when a
combined send-and-deliver flow runs out of fee budget mid-call.

Failure Modes
-------------

- **Parse error** — bad prelude/expr syntax. Surfaced as
  ``parsing <label> fee expression``; the GenVM process refuses to start.
- **Evaluation error** at startup — type mismatch, undefined variable, division by
  zero. Same surface; refuses to start.
- **Type / range error** during a charge — the delta returned a non-integer, a
  negative, or a value above ``U256::MAX``. Logged as
  ``failed to evaluate fee expression`` and propagated to the caller; the cost is
  treated as undefined and the charge fails closed.
- **Bucket underflow** at startup — ``subtract_on_start`` exceeds the bucket total.
  Returns the bucket's OOM ``VmError`` immediately.
- **Bucket underflow** during a charge — ``consume_bucket_raw`` returns ``false``.
  The caller turns this into an OOM-class error
  (``oom().storage()`` for ``consume_storage_pages``,
  ``oom().receipt().nondet_output()`` for ``consume_nondet_output``, etc.).

Configuration Sketch
--------------------

A minimal, illustrative ``fees`` block::

   "fees": {
     "expr_prelude":
       "let Y = \\f = (\\x = f (x x)) (\\x = f (x x)) in",
     "storage": {
       "bucket_no": 0,
       "subtract_on_start_expr": "node.gasPerStorageBootstrap",
       "delta_expr":              "\\a = a.pages * node.gasPerStoragePage"
     },
     "message_receipt": {
       "bucket_no": 1,
       "delta_expr":
         "\\a = if a.isDeploy then node.gasPerDeployByte * (a.calldataLength + a.codeLength)\n         else node.gasPerCallByte * a.calldataLength"
     },
     "nondet_output": {
       "bucket_no": 2,
       "delta_expr": "\\a = a.outputLength * node.gasPerNondetByte"
     },
     "message_fee": {
       "bucket_no": 1,
       "delta_expr":
         "\\a = if a.isInternal\n        then (if a.onAcceptance then node.gasInternalAccept else node.gasInternalFinal)\n        else floor (a.matchedFeeParams.executionBudgetPerRound * arrayLen a.matchedFeeParams.rotations)"
     }
   }

``message_fee`` and ``message_receipt`` share ``bucket_no = 1`` here, so an outbound
message debits both costs atomically against bucket 1.
