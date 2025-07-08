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
which accepts calldata-encoded message and returns an error code or :term:`fd`.

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

   -  Custom calldata format for complex data structures
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
