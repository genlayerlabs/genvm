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

When ``state`` is ``0`` (``default``) :term:`GenVM` resolves it to ``latest_non_final``.
The motivation is that the caller has already observed (and possibly modified)
non-final state in the current transaction, so reading anything older than
``latest_non_final`` for an in-transaction call would expose stale data and
break causality between the caller and callee.

Requirements
~~~~~~~~~~~~

#. :ref:`gvm-perm-deterministic`
#. :ref:`gvm-perm-call-others`

Creates new :term:`sub-VM` instance for contract execution. See :ref:`gvm-permissions` for permission inheritance details.

.. _gvm-def-post-message:

``PostMessage`` Message
-----------------------

Posts message to GenLayer contract for later execution.

When GenVM forwards this message it derives a :ref:`call_key <gvm-def-call-key>`
from the ``method`` field of ``calldata`` and attaches it to the emitted
message. The :ref:`call_key <gvm-def-call-key>` is the function-selector
analog used to identify the target method.

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
       "data": Bytes,                  // Code/data for sandbox execution
       "allow_write_storage": Bool,    // Whether to allow storage writes
       "allow_send_messages": Bool,    // Whether to allow sending messages
       "allow_register_runners": Bool  // Whether to allow registering runners
     }
   }

Creates isolated VM instance. See :ref:`gvm-permissions` for permission inheritance details.

``RegisterRunner`` Message
--------------------------

Registers a runner archive at runtime, making it available under the
``custom:<hash>`` runner id. The ``<hash>`` is the SHA3-256 of the supplied
``code`` encoded with :doc:`../../04-contract-interface/06-gvm32`, and the parsed
archive is charged against the memory limit. Returns the resulting runner id
(calldata-encoded string).

Payload
~~~~~~~

.. code-block::

   {
     "RegisterRunner": {
       "code": Bytes              // runner archive (ustar/zip or commented text)
     }
   }

Requirements
~~~~~~~~~~~~

#. :ref:`gvm-perm-deterministic`
#. :ref:`gvm-perm-register-runners`

``MapFile`` Message
-------------------

Maps a file from a runner into the VM filesystem at runtime, behaving the same as
the ``MapFile`` runner action (see :doc:`../../../python-sdk` runners). If
``path_in_runner`` ends with ``/`` the whole directory subtree is mapped.

Payload
~~~~~~~

.. code-block::

   {
     "MapFile": {
       "runner": String,          // runner id (name:hash, contract, chain:..., custom:...)
       "path_in_runner": String,  // path within the runner archive
       "path_in_vfs": String      // absolute destination path in the VM filesystem
     }
   }

Mapping into ``/vm/`` is forbidden. Resolving a ``chain:`` runner reads another
contract's storage, so this requires :ref:`gvm-perm-read-storage`.

Requirements
~~~~~~~~~~~~

#. :ref:`gvm-perm-read-storage`

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
In :ref:`gvm-def-det-mode`, it returns ``0`` — exposing real elapsed time there would break determinism. The sole exception is the ``unsafe-tracing`` debug level (see the executor "Debug modes" section), which returns real elapsed time even in deterministic mode; that level is for local debugging only and must never be used on a consensus network.

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
