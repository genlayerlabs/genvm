Functions
=========

``storage_read``
----------------

Reads data from contract storage at the specified slot and index.

Requirements
~~~~~~~~~~~~

#. :term:`Sub-VM` must be in deterministic mode
#. :term:`Sub-VM` must have read storage permission
#. index + buf_len must not overflow

``storage_write``
-----------------

Writes data to contract storage at the specified slot and index.

Requirements
~~~~~~~~~~~~

#. :term:`Sub-VM` must be in deterministic mode
#. :term:`Sub-VM` must have write storage permission
#. index + buf_len must not overflow
#. :term:`Sub-VM` Storage slot must not be locked, unless the sender is in ``upgraders``

``get_balance``
---------------

Queries the balance of a specified contract address.

Result value is a 32 octets long little-endian unsigned integer

``get_self_balance``
--------------------

Gets the current contract's balance, adjusted for the current transaction context.
It is following: balance_before_transaction + message.value - value_consumed_by_current_tx

Result value is a 32 octets long little-endian unsigned integer

Requirements
~~~~~~~~~~~~

- Contract must be in deterministic mode

``gl_call``
-----------

Primary GenLayer WASI SDK function handling most intelligent contract operations.
Takes serialized :ref:`Calldata Encoded <gvm-def-calldata-encoding>` message buffer
and dispatches to various blockchain operations based on message type.

Parameters: ``request`` (calldata buffer), ``request_len`` (buffer length), ``result_fd`` (output file descriptor)

Returns

- ``error_success`` on success
- ``error_inval`` for invalid requests
- ``error_forbidden`` for permission violations
- ``error_inbalance`` for insufficient balance

See :doc:`02-gl_call` for the list of available messages.
