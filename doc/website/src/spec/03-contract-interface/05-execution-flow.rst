Contract Execution Flow
=======================

Overview
--------

This document describes the complete execution flow for GenVM contracts,
from deployment to method invocation and result processing. The flow
involves multiple components working together to provide a seamless
contract execution experience.

.. _contract-execution-flow-1:

Contract Execution Flow
-----------------------

1. Contract Deployment (if needed)
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

-  Host writes contract code to blockchain storage
-  Code includes runner specification and dependencies

2. Contract Loading
~~~~~~~~~~~~~~~~~~~

-  GenVM receives contract address from message
-  Reads contract's locked slots and code from storage
-  Checks upgradability-related data from :doc:`03-upgradability`
-  Creates empty VFS, empty arguments list and empty environment variables map
-  Inspects contract runner as in :doc:`../02-execution-environment/03-runners`
-  Processes actions until ``StartWasm`` is encountered

3. WebAssembly Execution
~~~~~~~~~~~~~~~~~~~~~~~~

-  :term:`Runner` actions must reach a ``StartWasm`` action
-  GenVM starts WebAssembly :term:`module` with stdin containing calldata-encoded extended-message
-  Executes entry point (``_start``) with calldata from :term:`host`

4. Contract Entry Point Processing
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

The contract startup requires specific fields in the calldata-encoded message:

-  **``entry_kind``**: Determines execution context

   -  ``main``: Regular contract entry for standard method calls
   -  ``sandbox``: Non-deterministic block execution
   -  ``consensus_stage``: Validator consensus functions with ``entry_stage_data``

-  **``entry_data``**: Blob of bytes containing method call information
-  **``entry_stage_data``**: Consensus information for validator nodes

   -  ``null`` for leader nodes
   -  ``{leaders_result: <calldata>}`` for validator nodes
