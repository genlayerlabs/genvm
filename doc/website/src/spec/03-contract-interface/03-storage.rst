Storage System
==============

Overview
--------

GenVM's storage system provides persistent state management for
intelligent contracts with a focus on deterministic behavior, efficient
access patterns, and cross-language compatibility. The system supports
complex data structures while maintaining blockchain consensus
requirements and providing type-safe access patterns.

Storage Architecture
--------------------

Design Principles
~~~~~~~~~~~~~~~~~

-  **Deterministic Access**: Consistent storage behavior across all validator nodes. Reading uninitialied memory must return zeroes
-  **Efficiency**: Optimized storage layout and access patterns for
   blockchain use
-  **Language Agnostic**: Storage abstractions that work across
   programming languages

Hierarchical Structure
~~~~~~~~~~~~~~~~~~~~~~

-  **Contract Namespace**: Each contract has an isolated storage
   namespace
-  **Slot Organization**: Storage organized into fixed-size :term:`Storage Slot`\s with
   unique identifiers (:term:`SlotID`)
-  **Linear Memory Model**: Each slot provides linear memory access with
   read/write operations
-  **Hierarchical Addressing**: Complex data structures using derived
   slot addresses

Root Slot Management
~~~~~~~~~~~~~~~~~~~~

-  **Metadata Storage**: Contract metadata and control information in
   root slot
-  **Access Control**: Upgrade permissions and slot locking :doc:`mechanisms <03-upgradability>`
-  **Code Management**: Contract code storage and versioning
-  **Lifecycle Control**: Contract initialization and upgrade
   coordination

Default Derivation Algorithm
~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Consider following structure:

.. code-block:: python

   x: str
   y: str

Both ``x`` and ``y`` may occupy arbitrary amount of space. For that reason variable-length content is stored at an *indirection*
(separate :term:`Storage Slot` which :term:`SlotID` is computed based on previous location).
It is computed as following: *sha3_256(slot_id, offset_in_slot_as_4_bytes_little_endian)*.

This means that that it is: *sha3_256(slot_id, [0, 0, 0, 0])* for ``x`` and *sha3_256(slot_id, [0, 0, 0, 4])* for ``y``.
4 is because maximum length of string is bound by 4GB and there is no point in storing it at indirection.
Note that any data that uses an indirection must occupy at least one byte in it's residing slot
