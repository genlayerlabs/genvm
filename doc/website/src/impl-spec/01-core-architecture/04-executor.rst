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
