Application Binary Interface
============================

The GenVM Application Binary Interface defines how contracts
expose their functionality to external callers and how different
contracts interact with each other. The ABI provides a standardized way
to encode method calls, handle parameters, and manage contract schemas
while supporting both deterministic and non-deterministic operations.

.. _gvm-def-contract-call-conv:

Method Calling Convention
-------------------------

Method calls use :ref:`gvm-def-calldata-encoding` format with following convention:

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

All special methods start with a ``#`` character. Currently there are:

- :ref:`gvm-def-enum-value-special-method-get-schema` may expose contract schema, that provides definition of existing methods.
    This method must :ref:`gvm-def-return` a string containing a JSON object, that follows a schema.
- :ref:`gvm-def-enum-value-special-method-errored-message` called when execution of an emitted message, that had a value, was not successful
