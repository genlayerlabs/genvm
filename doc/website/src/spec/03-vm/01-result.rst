VM Execution Result
===================

.. _gvm-def-vm-result:

Result Kinds
------------

.. _gvm-def-return:

.. rubric:: Return

Represents successful execution of a :term:`sub-VM`

.. _gvm-def-vm-error:

.. rubric:: VMError

Represents a VM produced error, such as non-zero exit code or
exceeding resource limits.

It uses predefined string error codes.

.. _gvm-def-user-error:

.. rubric:: UserError

Represents a user-produced error in utf-8 format.

.. _gvm-def-internal-error:

InternalError
-------------

It is a special :ref:`gvm-def-vm-result` that represents an internal error in the VM,
such as: :term:`Host` communication failures or :term:`Module` unavailability.

Internal errors are not visible by the contracts. Most likely :term:`Host` will
vote *timeout* if encounters such an error

Non-Deterministic Block Result Encoding
---------------------------------------

- :ref:`gvm-def-return`\: Arbitrary structure in :ref:`gvm-def-calldata-encoding`
- :ref:`gvm-def-user-error`\: utf-8 string
- :ref:`gvm-def-vm-error`\: utf-8 string

Contract Result Encoding
------------------------

:ref:`gvm-def-return`
~~~~~~~~~~~~~~~~~~~~~

Arbitrary structure in :ref:`gvm-def-calldata-encoding`

:ref:`gvm-def-user-error` and :ref:`gvm-def-vm-error`
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

The error value (``UserError`` value or ``VMError`` error code string) is reported
alongside two fingerprint fields, each :ref:`gvm-def-calldata-encoding` encoded:

``backtrace`` — the WASM call stack captured at the failure point:

.. code-block:: json

  [
    {
      "module_name": "<module_name>",
      "func": "<number: function_index>"
    }
  ]

``wasm_store_hashes`` — per-module memory fingerprint:

.. code-block:: json

  {
    "<module_name>": {
      "memories": [
        "<bytes: 32_byte_blake3_hash>"
      ]
    }
  }

For sake of preventing skipping execution for error results, validators are obligated to calculate
the VM fingerprint on error.

Both fields are serialized using :ref:`gvm-def-calldata-encoding` to be deterministic, and have the following structure:

#. ``backtrace`` frames are ordered from most recent to oldest one (most likely, ``_start``)
#. Function index is an index of function in WASM module
#. Memories are ordered by their index in WASM module
#. Memories are hashed using BLAKE3 hash function, which is cryptographically secure and provides acceptable performance

Execution Hash
--------------

.. _gvm-def-execution-hash:

Every run produces an *execution hash*: a SHA3-256 digest over a
:ref:`gvm-def-calldata-encoding` encoding of the consensus-visible result, as a map
with the following keys (in this order):

#. ``backtrace``
#. ``data`` — the contract result value
#. ``data_fees_remaining``
#. ``kind`` — the :ref:`gvm-def-vm-result` result code
#. ``storage_changes``
#. ``subvm_hashes`` — see :ref:`gvm-def-subvm-hash`
#. ``wasm_store_hashes``

Two runs that agree on the deterministic result produce the same execution hash, so
consensus can compare a single 32-byte value instead of the full result.

Sub-VM Result Hash
------------------

.. _gvm-def-subvm-hash:

A deterministic run accumulates the result of each deterministic :term:`sub-VM` call
into a rolling SHA3-256 accumulator. The finalized accumulator is the
``subvm_hashes`` field of the parent's :ref:`gvm-def-execution-hash`.

For each deterministic sub-VM, its *small hash* is folded into the accumulator. The
small hash is a SHA3-256 digest over a :ref:`gvm-def-calldata-encoding` encoded map:

#. ``kind`` — the exact string ``"Return"``, ``"UserError"``, or ``"VMError"``
#. ``result`` — for ``Return``/``UserError`` the result value in :ref:`gvm-def-calldata-encoding`; for ``VMError`` the error code string
#. ``subvm_hashes`` — that sub-VM's own finalized accumulator, making the hash recursive over the whole deterministic call tree
#. ``wasm_store_hashes``

Non-deterministic sub-calls do not contribute. A run with no deterministic sub-calls
finalizes its accumulator to a fixed digest, so that error and edge results hash
uniformly.
