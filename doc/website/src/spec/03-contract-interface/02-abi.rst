Application Binary Interface (ABI)
==================================

Overview
--------

The GenVM Application Binary Interface (ABI) defines how contracts
expose their functionality to external callers and how different
contracts interact with each other. The ABI provides a standardized way
to encode method calls, handle parameters, and manage contract schemas
while supporting both deterministic and non-deterministic operations.

Method calls use :term:`calldata` format with following convention:

.. code-block::

    # deployment
    {
      "args": Array | absent,
      "kwargs": Map | absent,
    }

    # not deployment
    {
      "method": String | absent
      "args": Array | absent,
      "kwargs": Map | absent,
    }

Special Methods
---------------

- ``#error`` will be called when execution of an emitted message, that had a value, was not successful
- ``#get-schema`` may expose contract schema, that provides definition of existing methods
