Resource Limiting
=================

:ref:`gvm-def-det-mode` and :ref:`gvm-def-non-det-mode` have two separate RAM limits.
However, all :term:`sub-VM` instances share within same mode the same RAM capacity.

RAM limit is 4294967295 octets (4 GB) for each :ref:`gvm-def-non-det-mode`\.

.. _gvm-def-ram-consumption:

RAM Consumption
---------------

RAM is consumed by subtracting specified amount of memory from limit of current :ref:`gvm-def-vm-mode` of given :term:`sub-VM`\.

When consuming RAM would lead to remaining RAM to be negative,
the :term:`Sub-VM` exits with :ref:`gvm-def-vm-error` with :ref:`gvm-def-enum-value-vm-error-oom` message.

RAM Release
-----------

When a :term:`sub-VM` finishes execution, all RAM memory occupied by it gets released.
However, during the execution process, the RAM memory can not be released.
