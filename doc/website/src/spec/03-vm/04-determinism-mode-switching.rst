Switching To :ref:`gvm-def-non-det-mode`
========================================

When requesting a non-deterministic execution, new :term:`sub-VM` is created.

Leader Mode
-----------

Returns back :ref:`gvm-def-vm-result` produced by :term:`sub-VM` as-is.

Sync Mode
---------

Returns back leaders result as is.

Validator Mode
--------------

This :term:`sub-VM` must :ref:`gvm-def-return` a ``bool`` value, if validator
accepts the result of the leader :term:`sub-VM` execution.

Any other results have the same effect as producing ``bool(false)``.

Returns back leaders result.
