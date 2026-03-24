Resource Limiting
=================

:ref:`gvm-def-det-mode` and :ref:`gvm-def-non-det-mode` have two separate RAM budgets.
Each budget starts at 4294967295 octets (4 GiB).
All :term:`sub-VM` instances within the same mode share the same budget.

.. _gvm-def-ram-consumption:

RAM Consumption
---------------

Every resource allocation subtracts from the RAM budget of the current :ref:`gvm-def-vm-mode`.
When an allocation would cause the remaining budget to become negative,
the :term:`sub-VM` exits with :ref:`gvm-def-vm-error` with :ref:`gvm-def-enum-value-vm-error-oom` message.

The following operations consume RAM:

- **WASM memory growth**: each page (65536 octets) costs its size in bytes
- **WASM table growth**: each table entry costs :ref:`gvm-def-consts-value-memory-limiter-consts-table-entry` octets
- **File mapping**: :ref:`gvm-def-consts-value-memory-limiter-consts-file-mapping` octets base cost plus the length of the filename in bytes
- **File descriptor allocation**: :ref:`gvm-def-consts-value-memory-limiter-consts-fd-allocation` octets per descriptor

RAM Release
-----------

File content memory is released when the corresponding file descriptor is closed via ``fd_close``.
When a :term:`sub-VM` finishes execution, all remaining RAM consumed by it is released back to the shared budget.

Other Limits
------------

In addition to the RAM budget, the following hard limits apply:

- Maximum :term:`sub-VM` nesting depth: :ref:`gvm-def-consts-value-top-limits-vm-recursion`
- Maximum ``RunNondet`` calls per execution: :ref:`gvm-def-consts-value-top-limits-nondet-blocks`
- Maximum open file descriptors per :term:`sub-VM`: :ref:`gvm-def-consts-value-top-limits-max-fds`
- Maximum locked storage slots: :ref:`gvm-def-consts-value-top-limits-locked-slots`
- Minimum remaining RAM to issue a ``WebRequest``: :ref:`gvm-def-consts-value-top-limits-web-request-min-space` octets
- Minimum remaining RAM to issue a ``WebRender``: :ref:`gvm-def-consts-value-top-limits-web-render-min-space` octets

Exceeding any of these limits causes the :term:`sub-VM` to exit with :ref:`gvm-def-vm-error`.
