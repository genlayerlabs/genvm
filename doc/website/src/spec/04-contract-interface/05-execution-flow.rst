Contract Execution Flow
=======================

This document describes the complete execution flow for GenVM contracts,
from deployment to method invocation and result processing. The flow
involves multiple components working together to provide a seamless
contract execution experience.

.. _contract-execution-flow-1:

1. Contract Deployment (if needed)
----------------------------------

- :term:`Host` writes contract code to blockchain storage, according to :ref:`genvm-def-root-slot` definition
- Code includes runner specification and dependencies

2. Contract Loading
-------------------

:term:`GenVM` does the following steps:

#. Receives contract address from message
#. Reads contract's locked slots and code from storage
#. Checks upgradability-related data from :doc:`04-upgradability`
#. Creates empty VFS, empty arguments list and empty environment variables map
#. Inspects contract runner as in :doc:`../02-execution-environment/04-runners`
#. Processes actions until :ref:`gvm-def-start-wasm` is encountered

3. WebAssembly Execution
------------------------

- GenVM starts WebAssembly :term:`module` with stdin containing :ref:`Calldata Encoded <gvm-def-calldata-encoding>` extended-message
- Executes entry point (``_start``) with calldata from :term:`host`

4. Contract Entry Point Processing
----------------------------------

The contract startup requires specific fields in the :ref:`Calldata Encoded <gvm-def-calldata-encoding>` extended-message:

- ``entry_kind``: Determines execution context

   -  :ref:`gvm-def-enum-value-entry-kind-main`\: Regular contract entry for standard method calls,
      ``entry_data`` contains method call information as described in :ref:`gvm-def-contract-call-conv`
   -  :ref:`gvm-def-enum-value-entry-kind-sandbox`\: Contract decides for itself how to handle the payload in ``entry_data``
   -  :ref:`gvm-def-enum-value-entry-kind-consensus-stage`\: Contract decides for itself how to handle the payload in ``entry_data``
       to call validator consensus functions with ``entry_stage_data``

-  ``entry_data``: Blob of bytes containing method call information
-  ``entry_stage_data``: Consensus information for validator nodes

   -  ``null`` for leader nodes
   -  ``{leaders_result: <calldata>}`` for validator nodes

Extended-Message Format
^^^^^^^^^^^^^^^^^^^^^^^

.. code-block:: rust

   pub struct ExtendedMessage {
      pub contract_address: calldata::Address,
      pub sender_address: calldata::Address,
      pub origin_address: calldata::Address,
      /// View methods call chain.
      /// It is empty for entrypoint (refer to [`contract_address`])
      pub stack: Vec<calldata::Address>,

      pub chain_id: num_bigint::BigInt,
      pub value: num_bigint::BigInt,
      pub is_init: bool,
      /// Transaction timestamp
      pub datetime: chrono::DateTime<chrono::Utc>,

      #[serde(serialize_with = "entry_kind_as_int")]
      pub entry_kind: public_abi::EntryKind,
      #[serde(with = "serde_bytes")]
      pub entry_data: Vec<u8>,

      pub entry_stage_data: calldata::Value,
   }
