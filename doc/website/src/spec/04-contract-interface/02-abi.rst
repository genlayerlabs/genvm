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

.. _gvm-def-call-key:

Call Key
--------

Every emitted message (see :ref:`gvm-def-post-message`) carries a ``call_key``:
a 256-bit unsigned integer that identifies which method the message targets.
It serves the same role as a function selector in EVM, but is derived
differently and is not truncated.

The ``call_key`` is computed by GenVM from the ``method`` field of the
:ref:`gvm-def-contract-call-conv` calldata as follows:

- A deployment message uses the reserved value ``0`` (``DEPLOY``).
- A non-deployment message with no ``method`` field uses the reserved
  value ``0`` (``UNNAMED``).
- For a method name shorter than 32 bytes (UTF-8), the ``call_key`` is the
  name bytes placed at the most significant end of a 32-byte big-endian
  buffer, zero-padded on the right. This keeps short names directly
  human-readable from their key.
- For a method name of 32 bytes or longer, the ``call_key`` is
  ``keccak256(name)`` with the least significant bit of the last octet
  forced to ``1``. The forced bit guarantees this branch can never collide
  with the zero-padded short-name encoding above (whose low bytes are
  always zero once the name fits) nor with the reserved ``0`` value.

The resulting value is interpreted as a big-endian ``U256``.

Special Methods
---------------

All special methods start with a ``#`` character. Currently there are:

- :ref:`gvm-def-enum-value-special-method-get-schema` may expose contract schema, that provides definition of existing methods.
    This method must :ref:`gvm-def-return` a string containing a JSON object, that follows a schema.
- :ref:`gvm-def-enum-value-special-method-errored-message` called when execution of an emitted message, that had a value, was not successful
