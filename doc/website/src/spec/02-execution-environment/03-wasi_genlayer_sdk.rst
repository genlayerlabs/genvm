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

Backwards Compatibility
-----------------------

Passing invalid request to ``gl_call`` results in ``error_inval``.
Passing data that turned out to be compatible with future version
is filtered out by version limitation. And will result in ``error_inval``
if method wasn't available at given version

Functions
---------

``storage_read``
~~~~~~~~~~~~~~~~

Reads data from contract storage at the specified slot and index.

Requirements
^^^^^^^^^^^^

#. :term:`Sub-VM` must be in deterministic mode
#. :term:`Sub-VM` must have read storage permission
#. index + buf_len must not overflow

``storage_write``
~~~~~~~~~~~~~~~~~

Writes data to contract storage at the specified slot and index.

Requirements
^^^^^^^^^^^^

#. :term:`Sub-VM` must be in deterministic mode
#. :term:`Sub-VM` must have write storage permission
#. index + buf_len must not overflow
#. :term:`Sub-VM` Storage slot must not be locked, unless the sender is in ``upgraders``

``get_balance``
~~~~~~~~~~~~~~~

Queries the balance of a specified contract address.

Result value is a 32 octets long little-endian unsigned integer

``get_self_balance``
~~~~~~~~~~~~~~~~~~~~

Gets the current contract's balance, adjusted for the current transaction context.
It is following: balance_before_transaction + message.value - value_consumed_by_current_tx

Result value is a 32 octets long little-endian unsigned integer

Requirements
^^^^^^^^^^^^

- Contract must be in deterministic mode

``gl_call``
~~~~~~~~~~~

Primary GenLayer WASI SDK function handling most intelligent contract operations.
Takes serialized :ref:`Calldata Encoded <gvm-def-calldata-encoding>` message buffer
and dispatches to various blockchain operations based on message type.

Parameters: ``request`` (calldata buffer), ``request_len`` (buffer length), ``result_fd`` (output file descriptor)

Returns

- ``error_success`` on success
- ``error_inval`` for invalid requests
- ``error_forbidden`` for permission violations
- ``error_inbalance`` for insufficient balance

``gl_call`` Functions
---------------------

``EthSend`` Message
~~~~~~~~~~~~~~~~~~~

Sends transaction to Ethereum address with optional value transfer.

Payload
^^^^^^^

.. code-block::

   {
     "EthSend": {
       "address": Address,      // 20-byte target address
       "calldata": Bytes,       // EVM calldata
       "value": U256            // Wei to transfer
     }
   }

Requirements
^^^^^^^^^^^^

#. :ref:`gvm-def-det-mode` execution
#. ``can_send_messages`` permission
#. Sufficient contract balance for value transfer

``EthCall`` Message
~~~~~~~~~~~~~~~~~~~

Calls Ethereum contract method (read-only operation).

Payload
^^^^^^^

.. code-block::

   {
     "EthCall": {
       "address": Address,      // 20-byte target contract address
       "calldata": Bytes        // EVM calldata
     }
   }

Requirements
^^^^^^^^^^^^

#. :ref:`gvm-def-det-mode` execution
#. ``can_call_others`` permission

``CallContract`` Message
~~~~~~~~~~~~~~~~~~~~~~~~

Calls another GenLayer Intelligent Contract.

Payload
^^^^^^^

.. code-block::

   {
     "CallContract": {
       "address": Address,      // 20-byte target contract address
       "calldata": Calldata,    // Method call in calldata format
       "state": Number          // Storage type: 0=default, 1=latest_final, 2=latest_non_final
     }
   }

Requirements
^^^^^^^^^^^^

#. :ref:`gvm-def-det-mode` execution
#. ``can_call_others`` permission

Creates new :term:`sub-VM` instance for contract execution. Inherits sender permissions but disables ``write_storage``.

``PostMessage`` Message
~~~~~~~~~~~~~~~~~~~~~~~

Posts message to GenLayer contract for later execution.

Payload
^^^^^^^

.. code-block::

   {
     "PostMessage": {
       "address": Address,      // 20-byte target contract address
       "calldata": Calldata,    // Method call in calldata format
       "value": U256,           // Wei to transfer
       "on": String             // "finalized" or "accepted"
     }
   }

Requirements
^^^^^^^^^^^^

#. :ref:`gvm-def-det-mode` execution
#. ``can_send_messages`` permission
#. Sufficient contract balance for value transfer

``DeployContract`` Message
~~~~~~~~~~~~~~~~~~~~~~~~~~

Deploys new intelligent contract to blockchain.

Payload
^^^^^^^

.. code-block::

   {
     "DeployContract": {
       "calldata": Calldata,    // Constructor arguments in calldata format
       "code": Bytes,           // Contract bytecode
       "value": U256,           // Wei to transfer
       "on": String,            // "finalized" or "accepted"
       "salt_nonce": U256       // Salt for CREATE2-style deterministic addressing
     }
   }

Requirements
^^^^^^^^^^^^

#. :ref:`gvm-def-det-mode` execution
#. ``can_send_messages`` permission
#. Sufficient contract balance for value transfer

Supports CREATE2-style deployment with salt nonce for deterministic addressing.

``RunNondet`` Message
~~~~~~~~~~~~~~~~~~~~~

Executes non-deterministic code with leader/validator consensus.
Creates :ref:`gvm-def-non-det-mode` VM instance with restricted permissions.
See :doc:`../03-vm/04-determinism-mode-switching` for more details.

Payload
^^^^^^^

.. code-block::

   {
     "RunNondet": {
       "data_leader": Bytes,      // Code/data for leader execution
       "data_validator": Bytes    // Code/data for validator execution
     }
   }

Requirements
^^^^^^^^^^^^

#. ``can_spawn_nondet`` permission

``Sandbox`` Message
~~~~~~~~~~~~~~~~~~~

Executes code in sandboxed environment with restricted permissions.

Payload
^^^^^^^

.. code-block::

   {
     "Sandbox": {
       "data": Bytes,             // Code/data for sandbox execution
       "allow_write_ops": Bool    // Whether to allow write operations
     }
   }

Creates isolated VM instance. Inherits :ref:`gvm-def-det-mode` from parent. Disables storage read access and ``spawn_nondet``/``call_others`` permissions.

``WebRender`` Message
~~~~~~~~~~~~~~~~~~~~~

Renders web content using GenVM web module.

Payload
^^^^^^^

.. code-block::

   {
     "WebRender": {
       "mode": String,            // "text", "html", or "screenshot"
       "url": String,             // URL to render
       "wait_after_loaded": String // Wait duration, e.g. "5s" or "500ms"
     }
   }

Requirements
^^^^^^^^^^^^

#. :ref:`gvm-def-non-det-mode` execution
#. Web module availability

``WebRequest`` Message
~~~~~~~~~~~~~~~~~~~~~~

Makes HTTP requests using GenVM web module.

Payload
^^^^^^^

.. code-block::

   {
     "WebRequest": {
       "method": String,          // "GET", "POST", "HEAD", "DELETE", "OPTIONS", or "PATCH"
       "url": String,             // Request URL
       "headers": Map,            // String -> Bytes mapping of headers
       "body": Bytes | null,      // Optional request body
       "sign": Bool               // Whether to sign the request (default: false)
     }
   }

Response
^^^^^^^^

.. code-block::

   {
     "status": Number,            // HTTP status code
     "headers": Map,              // String -> Bytes mapping of response headers
     "body": Bytes                // Response body
   }

Requirements
^^^^^^^^^^^^

#. :ref:`gvm-def-non-det-mode` execution
#. Web module availability

``ExecPrompt`` Message
~~~~~~~~~~~~~~~~~~~~~~

Executes LLM prompts using GenVM LLM module.

Payload
^^^^^^^

.. code-block::

   {
     "ExecPrompt": {
       "response_format": String, // "text" (default) or "json"
       "prompt": String,          // The prompt text
       "images": Array            // Array of image bytes (max 2)
     }
   }

Requirements
^^^^^^^^^^^^

#. :ref:`gvm-def-non-det-mode` execution
#. LLM module availability

Supports up to 2 images per prompt. Consumes fuel based on LLM usage.

``ExecPromptTemplate`` Message
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Executes structured LLM prompt templates with type-specific validation.

Payload
^^^^^^^

One of the following template types:

.. code-block::

   // Comparative template (expects boolean response)
   {
     "ExecPromptTemplate": {
       "template": "EqComparative",
       "leader_answer": String,
       "validator_answer": String,
       "principle": String
     }
   }

   // Non-comparative validator template
   {
     "ExecPromptTemplate": {
       "template": "EqNonComparativeValidator",
       "task": String,
       "criteria": String,
       "input": String,
       "output": String
     }
   }

   // Non-comparative leader template
   {
     "ExecPromptTemplate": {
       "template": "EqNonComparativeLeader",
       "task": String,
       "criteria": String,
       "input": String
     }
   }

Requirements
^^^^^^^^^^^^

#. :ref:`gvm-def-non-det-mode` execution
#. LLM module availability

Comparative templates expect boolean responses. Non-comparative templates expect text responses.

``EmitEvent`` Message
~~~~~~~~~~~~~~~~~~~~~

Emits blockchain events with topics and data.

Payload
^^^^^^^

.. code-block::

   {
     "EmitEvent": {
       "topics": Array,           // Array of 32-byte topics (max 4)
       "blob": Map                // String -> Calldata mapping of event data
     }
   }

Requirements
^^^^^^^^^^^^

#. :ref:`gvm-def-det-mode` execution
#. GenVM version 0.1.5 or higher

Topics must be exactly 32 bytes each.

``Rollback`` Message
~~~~~~~~~~~~~~~~~~~~

Triggers contract rollback with custom error message.

Payload
^^^^^^^

.. code-block::

   {
     "Rollback": String           // Error message
   }

Causes VM to exit with ``UserError``. Terminates contract execution immediately.

``Return`` Message
~~~~~~~~~~~~~~~~~~

Returns value from contract execution and terminates.

Payload
^^^^^^^

.. code-block::

   {
     "Return": Calldata           // Return value in calldata format
   }

Causes VM to exit with ``ContractReturn``. Encodes return value using :ref:`Calldata Encoded <gvm-def-calldata-encoding>` format.

``Trace.Message`` Message
~~~~~~~~~~~~~~~~~~~~~~~~~

Logs a debug message with timing information including:

- Custom message text
- Total elapsed time since VM start
- Time elapsed since last trace call

Payload
^^^^^^^

.. code-block::

   {
     "Trace": {
       "Message": String          // Debug message text
     }
   }

.. note::

   Implementations may choose to ignore this message and return an error.

Requirements
^^^^^^^^^^^^

#. GenVM version 0.1.10 or higher
#. :term:`GenVM` implementation is allowed ignore this message

.. _tracing-runtime-microsec:

``Trace.RuntimeMicroSec`` Sub-Message
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

In :ref:`gvm-def-non-det-mode` returns the elapsed execution time in microseconds since VM start.
In :ref:`gvm-def-det-mode`, it always returns ``0``.

Payload
^^^^^^^

.. code-block::

   {
     "Trace": "RuntimeMicroSec"
   }

.. note::

   Implementations may choose to ignore this message and return an error.

Requirements
^^^^^^^^^^^^

#. GenVM version 0.1.10 or higher
#. :term:`GenVM` implementation is allowed ignore this message
