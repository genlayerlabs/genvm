:term:`GenLayer WASI SDK` WASI Interface
========================================

Overview
--------

The :term:`GenLayer WASI SDK` WASI interface provides blockchain-specific
functionality to WebAssembly contracts through a custom WASI extension.
This interface enables contracts to interact with blockchain state,
execute non-deterministic operations, and participate in consensus
mechanisms while maintaining security and isolation.

Interface Design
----------------

Throughput-heavy operations are exposed as regular wasm functions.
Others functions are hidden behind ``gl_call`` function,
which accepts :ref:`Calldata Encoded <gvm-def-calldata-encoding>` message and returns an error code.

Interface Definition
--------------------

.. code-block:: C

   #include <stdint.h>

   static const uint32_t error_success = 0

   static const uint32_t error_overflow = 1
   static const uint32_t error_inval = 2
   static const uint32_t error_fault = 3
   static const uint32_t error_ilseq = 4

   static const uint32_t error_io = 5

   static const uint32_t error_forbidden = 6
   static const uint32_t error_inbalance = 7

   __attribute__((import_module("genlayer_sdk"))) uint32_t
   storage_read(char const* slot, uint32_t index, char* buf, uint32_t buf_len);
   __attribute__((import_module("genlayer_sdk"))) uint32_t
   storage_write(
      char const* slot,
      int32_t index,
      char const* buf,
      uint32_t buf_len
   );
   __attribute__((import_module("genlayer_sdk"))) uint32_t
   get_balance(char const* address, char* result);
   __attribute__((import_module("genlayer_sdk"))) uint32_t
   get_self_balance(char* result);
   __attribute__((import_module("genlayer_sdk"))) uint32_t
   gl_call(char const* request, uint32_t request_len, uint32_t* result_fd);

WebAssembly Integration
~~~~~~~~~~~~~~~~~~~~~~~

-  **Import Namespace**:

   -  Functions exposed under ``genlayer_sdk`` namespace
   -  Type-safe function signatures with WebAssembly validation
   -  Consistent error handling and return value patterns

-  **Data Serialization**:

   -  :ref:`gvm-def-calldata-encoding` for complex data structures
   -  Efficient binary encoding for blockchain primitives
   -  Cross-language type compatibility
   -  Deterministic serialization for consensus
   -  Safe decoding

Backwards Compatibility
-----------------------

Passing invalid request to ``gl_call`` results in ``error_inval``.
Passing data that turned out to be compatible with future version
is filtered out by version limitation. And will result in ``error_inval``
if method wasn't available at given version

Function Descriptions
---------------------

``gl_call``
~~~~~~~~~~~

``storage_read``
~~~~~~~~~~~~~~~~

Reads data from contract storage at the specified slot and index.

**Parameters:**
- ``slot``: Storage slot identifier (32 bytes)
- ``index``: Byte offset within the slot (u32)
- ``buf``: Buffer to read data into
- ``buf_len``: Length of the buffer (u32)

**Returns:** Error code (0 for success)

**Requirements:**
- Contract must be in deterministic mode
- Contract must have read storage permission
- Index + buf_len must not overflow

``storage_write``
~~~~~~~~~~~~~~~~~

Writes data to contract storage at the specified slot and index.

**Parameters:**
- ``slot``: Storage slot identifier (32 bytes)
- ``index``: Byte offset within the slot (u32)
- ``buf``: Buffer containing data to write
- ``buf_len``: Length of the data to write (u32)

**Returns:** Error code (0 for success)

**Requirements:**
- Contract must be in deterministic mode
- Contract must have write storage permission
- Index + buf_len must not overflow
- Storage slot must not be locked, unless the sender is in ``upgraders``

``get_balance``
~~~~~~~~~~~~~~~

Queries the balance of a specified contract address.

**Parameters:**
- ``address``: Contract address (20 bytes)
- ``result``: Buffer to store the balance result (32 bytes, little-endian)

**Returns:** Error code (0 for success)

**Behavior:**
- Returns the current balance of the specified address
- Balance is returned as a 32-byte little-endian encoded value

``get_self_balance``
~~~~~~~~~~~~~~~~~~~~

Gets the current contract's balance, adjusted for the current transaction context.

**Parameters:**
- ``result``: Buffer to store the balance result (32 bytes, little-endian)

**Returns:** Error code (0 for success)

**Requirements:**
- Contract must be in deterministic mode

**Behavior:**
- Returns: balance_before_transaction + message.value - value_consumed_by_current_tx
- Balance is returned as a 32-byte little-endian encoded value
