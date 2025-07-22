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

:ref:`gvm-def-calldata-encoding` encoding of

.. code-block:: json

  {
    "message": "<error_message_string>",
    "fingerprint": {
      "frames": [
        {
          "module_name": "<module_name>",
          "func": "<number: function_index>"
        }
      ],
      "module_instances": {
        "<module_name>": {
          "memories": [
            "<bytes: 32_byte_blake3_hash>"
          ]
        }
      }
    }
  }

For sake of preventing skipping execution for error results, validators are obligated to calculate
VM fingerprint on error.

Fingerprint is serialized using :ref:`gvm-def-calldata-encoding` to be deterministic, and has following structure:

#. Frames are ordered from most recent to oldest one (most likely, ``_start``)
#. Function index is an index of function in WASM module
#. Memories are ordered by their index in WASM module
#. Memories are hashed using BLAKE3 hash function, which is cryptographically secure and provides acceptable performance
