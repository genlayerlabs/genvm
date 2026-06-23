Executor
========

The executor (the ``genvm`` binary) runs a single contract execution to
completion and writes its result back to the :term:`Host`. It is a short-lived,
stateless process that the :term:`Manager` spawns once per execution.

No Internal Timeout
-------------------

The executor does **not** implement any execution timeout, deadline, or signal
handling of its own. It runs until the contract finishes — producing a
``Return``, ``UserError``, or ``VMError`` result — or until it is terminated
externally.

Timeouts are enforced entirely by the :term:`Manager`, which owns the executor
process lifecycle. When an execution exceeds its budget (or must otherwise be
stopped), the manager kills the executor process directly with ``SIGKILL``. The
executor installs no signal handlers and has no graceful-shutdown path, so it
**can be killed at any moment**, between any two operations, without notice.

Implications
-----------

- The executor keeps no durable state of its own. All persistent state lives in
  the host and is written only as part of delivering a result. A killed executor
  simply produces no result, which the manager treats as a failed / timed-out
  execution.
- Executor code must not rely on running cleanup, flushing buffers, or
  cancellation logic during shutdown — there is no shutdown to hook into.

Debug modes
-----------

The debug level is a top-level field of the manager ``run`` request
(``debug_mode``), forwarded to the executor as ``--debug-mode <level>``. The
default is ``disabled``.

Output **capture** is a derived property with three states:

- ``disabled`` — nothing is captured into the result: the executor's
  stdout/stderr go to ``/dev/null`` and its logs are *forwarded to the manager's
  own log* (so they are not lost, just not returned in the response).
- ``bounded`` — captured into the result, but bounded: at most the 128 most
  recent log entries are kept (oldest dropped), and stdout/stderr are truncated
  to a 4 MiB tail each.
- ``unbounded`` — captured into the result in full.

When capture is ``disabled`` the result's log/stdout/stderr fields are empty
(empty list / empty strings), not omitted; ``bounded`` and ``unbounded`` use the
same field shapes, differing only in how much they contain.

The levels are ordered; each *adds* to the previous one:

.. list-table::
   :header-rows: 1
   :widths: 20 22 58

   * - Level
     - Capture
     - Adds
   * - ``disabled``
     - ``disabled``
     - Production default. Logs are forwarded to the manager log; no result capture; no debug aids.
   * - ``safe``
     - ``bounded``
     - Logs and stdout/stderr are captured into the result (bounded). Deterministic.
   * - ``safe-unbounded``
     - ``unbounded``
     - Capture becomes unbounded and ``Trace`` gl_call output is emitted. Deterministic.
   * - ``unsafe``
     - ``unbounded``
     - The ``:latest`` / ``:test`` runner ids may be resolved. **Unsafe across machines**: different nodes may resolve different code and diverge consensus, though a single node stays deterministic.
   * - ``unsafe-tracing``
     - ``unbounded``
     - Real wall-clock time is exposed to the contract (``RuntimeMicroSec`` returns actual elapsed time instead of ``0``). **Can break determinism on a single machine.** Local debugging only.

Only ``unsafe`` and ``unsafe-tracing`` can affect determinism (across machines
and on a single machine respectively); ``safe`` and ``safe-unbounded`` are fully
consensus-safe (``safe-unbounded`` deliberately couples unbounded capture with
``Trace`` emission — both are deterministic, verbose debug aids). In code these
distinctions are methods on ``genvm_common::DebugMode``: ``capture()`` (returns
the ``Capture`` state), ``allows_tracing()``, ``allows_latest_resolution()``,
and ``allows_nondeterminism()``.

**Enforcement.** ``debug_mode`` is a per-request field, so on a consensus
network the manager MUST reject (or clamp to a configured maximum) ``unsafe``
and ``unsafe-tracing`` from untrusted callers — otherwise a single request could
force ``:latest``/``:test`` resolution (cross-node divergence) or expose real
time (single-machine non-determinism). Unbounded capture (``safe-unbounded`` and
above) also trades the memory bound for completeness, so operators should treat
the higher levels as privileged. An unrecognized ``debug_mode`` value is
rejected rather than silently downgraded.
