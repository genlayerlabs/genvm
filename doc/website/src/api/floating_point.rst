Floating Point in Python API
============================

GenVM handles floating point arithmetic differently depending on the execution mode.

Non-Deterministic Mode
----------------------

In non-deterministic mode, floating point operations are unrestricted. Standard hardware-native IEEE 754 floating point is used directly, providing full performance with no additional overhead.

Deterministic Mode
------------------

In deterministic mode, GenVM uses a software-implemented IEEE 754 floating point via integer arithmetic. This ensures that floating point computations produce bit-exact identical results across all validator nodes, regardless of the underlying hardware or platform. Native floating point instructions can yield slightly different results depending on the CPU architecture, compiler optimizations, or rounding behavior, which would break consensus. The software implementation eliminates this source of non-determinism at the cost of **significant** performance overhead.
