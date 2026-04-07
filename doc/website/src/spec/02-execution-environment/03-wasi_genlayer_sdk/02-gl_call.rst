``gl_call`` Messages
====================

``EthSend`` Message
-------------------

Sends transaction to Ethereum address with optional value transfer.

Payload
~~~~~~~

.. code-block::

   {
     "EthSend": {
       "address": Address,      // 20-byte target address
       "calldata": Bytes,       // EVM calldata
       "value": U256            // Wei to transfer
     }
   }

Requirements
~~~~~~~~~~~~

#. :ref:`gvm-perm-deterministic`
#. :ref:`gvm-perm-send-messages`
#. Sufficient contract balance for value transfer

``EthCall`` Message
-------------------

Calls Ethereum contract method (read-only operation).

Payload
~~~~~~~

.. code-block::

   {
     "EthCall": {
       "address": Address,      // 20-byte target contract address
       "calldata": Bytes        // EVM calldata
     }
   }

Requirements
~~~~~~~~~~~~

#. :ref:`gvm-perm-deterministic`
#. :ref:`gvm-perm-call-others`

``CallContract`` Message
------------------------

Calls another GenLayer Intelligent Contract.

Payload
~~~~~~~

.. code-block::

   {
     "CallContract": {
       "address": Address,      // 20-byte target contract address
       "calldata": Calldata,    // Method call in calldata format
       "state": Number          // Storage type: 0=default, 1=latest_final, 2=latest_non_final
     }
   }

Requirements
~~~~~~~~~~~~

#. :ref:`gvm-perm-deterministic`
#. :ref:`gvm-perm-call-others`

Creates new :term:`sub-VM` instance for contract execution. See :ref:`gvm-permissions` for permission inheritance details.

``PostMessage`` Message
-----------------------

Posts message to GenLayer contract for later execution.

Payload
~~~~~~~

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
~~~~~~~~~~~~

#. :ref:`gvm-perm-deterministic`
#. :ref:`gvm-perm-send-messages`
#. Sufficient contract balance for value transfer

``DeployContract`` Message
--------------------------

Deploys new intelligent contract to blockchain.

Payload
~~~~~~~

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
~~~~~~~~~~~~

#. :ref:`gvm-perm-deterministic`
#. :ref:`gvm-perm-send-messages`
#. Sufficient contract balance for value transfer

Supports CREATE2-style deployment with salt nonce for deterministic addressing.

``RunNondet`` Message
---------------------

Executes non-deterministic code with leader/validator consensus.
Creates :ref:`gvm-def-non-det-mode` VM instance with restricted permissions.
See :doc:`../../03-vm/04-determinism-mode-switching` and :ref:`gvm-permissions` for more details.

Payload
~~~~~~~

.. code-block::

   {
     "RunNondet": {
       "data_leader": Bytes,      // Code/data for leader execution
       "data_validator": Bytes    // Code/data for validator execution
     }
   }

Requirements
~~~~~~~~~~~~

#. :ref:`gvm-perm-spawn-nondet`

``Sandbox`` Message
-------------------

Executes code in sandboxed environment with restricted permissions.

Payload
~~~~~~~

.. code-block::

   {
     "Sandbox": {
       "data": Bytes,             // Code/data for sandbox execution
       "allow_write_ops": Bool    // Whether to allow write operations
     }
   }

Creates isolated VM instance. See :ref:`gvm-permissions` for permission inheritance details.

``WebRender`` Message
---------------------

Renders web content using GenVM web module.

Payload
~~~~~~~

.. code-block::

   {
     "WebRender": {
       "mode": String,            // "text", "html", or "screenshot"
       "url": String,             // URL to render
       "wait_after_loaded": String // Wait duration, e.g. "5s" or "500ms"
     }
   }

Requirements
~~~~~~~~~~~~

#. :ref:`gvm-def-non-det-mode` execution
#. Web module availability

``WebRequest`` Message
----------------------

Makes HTTP requests using GenVM web module.

Payload
~~~~~~~

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
~~~~~~~~

.. code-block::

   {
     "status": Number,            // HTTP status code
     "headers": Map,              // String -> Bytes mapping of response headers
     "body": Bytes                // Response body
   }

Requirements
~~~~~~~~~~~~

#. :ref:`gvm-def-non-det-mode` execution
#. Web module availability

``ExecPrompt`` Message
----------------------

Executes LLM prompts using GenVM LLM module.

Payload
~~~~~~~

.. code-block::

   {
     "ExecPrompt": {
       "response_format": String, // "text" (default) or "json"
       "prompt": String,          // The prompt text
       "images": Array            // Array of image bytes (max 2)
     }
   }

Requirements
~~~~~~~~~~~~

#. :ref:`gvm-def-non-det-mode` execution
#. LLM module availability

Supports up to 2 images per prompt. Consumes fuel based on LLM usage.

``ExecPromptTemplate`` Message
------------------------------

Executes structured LLM prompt templates with type-specific validation.

Payload
~~~~~~~

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
~~~~~~~~~~~~

#. :ref:`gvm-def-non-det-mode` execution
#. LLM module availability

Comparative templates expect boolean responses. Non-comparative templates expect text responses.

``EmitEvent`` Message
---------------------

Emits blockchain events with topics and data.

Payload
~~~~~~~

.. code-block::

   {
     "EmitEvent": {
       "topics": Array,           // Array of 32-byte topics (max 4)
       "blob": Map                // String -> Calldata mapping of event data
     }
   }

Requirements
~~~~~~~~~~~~

#. :ref:`gvm-perm-deterministic`
#. GenVM version 0.1.5 or higher

Topics must be exactly 32 bytes each.

``Rollback`` Message
--------------------

Deprecated. Use ``UserError`` instead.

``UserError`` Message
---------------------

Triggers contract UserError with custom error message.

Payload
~~~~~~~

.. code-block::

   {
     "UserError": Any           // Error message
   }

Causes VM to exit with ``UserError``. Terminates contract execution immediately.

``Return`` Message
------------------

Returns value from contract execution and terminates.

Payload
~~~~~~~

.. code-block::

   {
     "Return": Calldata           // Return value in calldata format
   }

Causes VM to exit with ``ContractReturn``. Encodes return value using :ref:`Calldata Encoded <gvm-def-calldata-encoding>` format.

``Trace.Message`` Message
-------------------------

Logs a debug message with timing information including:

- Custom message text
- Total elapsed time since VM start
- Time elapsed since last trace call

Payload
~~~~~~~

.. code-block::

   {
     "Trace": {
       "Message": String          // Debug message text
     }
   }

.. note::

   Implementations may choose to ignore this message and return an error.

Requirements
~~~~~~~~~~~~

#. GenVM version 0.1.10 or higher
#. :term:`GenVM` implementation is allowed ignore this message

.. _tracing-runtime-microsec:

``Trace.RuntimeMicroSec`` Sub-Message
-------------------------------------

In :ref:`gvm-def-non-det-mode` returns the elapsed execution time in microseconds since VM start.
In :ref:`gvm-def-det-mode`, it always returns ``0``.

Payload
~~~~~~~

.. code-block::

   {
     "Trace": "RuntimeMicroSec"
   }

.. note::

   Implementations may choose to ignore this message and return an error.

Requirements
~~~~~~~~~~~~

#. GenVM version 0.1.10 or higher
#. :term:`GenVM` implementation is allowed ignore this message
