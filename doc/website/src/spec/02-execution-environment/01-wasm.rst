WASM Utilization
================

Enabled WASM Features and Proposals
-----------------------------------

#. Core Modules
#. Bulk Memory
#. Sign Extension
#. Mutable Globals
#. Multi Value

:ref:`gvm-def-det-mode` Additional Limitations
------------------------------------------------

Only following ``f32.*`` and ``f64.*`` operations are allowed:

- ``f32.store``, ``f64.store``
- ``f32.load``, ``f64.load``
- ``f32.const``, ``f64.const``
- ``f32.reinterpret_i32``, ``f64.reinterpret_i64``
- ``i32.reinterpret_f32``, ``i64.reinterpret_f64``

Any other floating point operation is considered non-deterministic and is not allowed in :ref:`gvm-def-det-mode`.

:ref:`gvm-def-non-det-mode` does not have these limitations, allowing all floating point operations.

RAM Consumption
---------------

Each WASM table element imposes :ref:`gvm-def-enum-value-memory-limiter-consts-table-entry` :ref:`gvm-def-ram-consumption`\.

Each WASM Memory costs length of bytes it has. WASM ``memory.grow`` instruction which would exceed limit returns :math:`-1`
