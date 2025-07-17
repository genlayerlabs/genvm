Storage System
==============

GenVM's storage system provides persistent state management for
intelligent contracts. It is language-agnostic.

Storage Architecture
--------------------

#. Storage is scoped to an address
#. Storage is organized into :term:`Storage Slot`\s: blocks of ``4294967296`` octets (4GB)
#. For given address, each :term:`Storage Slot` has a unique identifier called :term:`SlotID` which is a 32-octet value
#. Reading uninitialized memory returns zeroes
#. Storage is linear, meaning that each slot provides a contiguous block of memory

Default Derivation Algorithm
----------------------------

.. note::

   This is a proposed default algorithm, but using it is not mandatory.

Consider following structure:

.. code-block:: python

   x: str
   y: str

Both ``x`` and ``y`` may occupy arbitrary amount of space. For that reason variable-length content is stored at an *indirection*:
separate :term:`Storage Slot` which :term:`SlotID` is computed based on previous location,
using following algorithm: *sha3_256(slot_id, offset_in_slot_as_4_bytes_little_endian)*.

This means that that it is: *sha3_256(slot_id, [0, 0, 0, 0])* for ``x`` and *sha3_256(slot_id, [0, 0, 0, 4])* for ``y``.
4 is because maximum length of string is bound by 4GB and there is no point in storing it at indirection.
Note that any data that uses an indirection must occupy at least one byte in it's residing slot

.. _genvm-def-root-slot:

Root Slot
---------

:term:`Storage Slot` with :term:`SlotID` of all zeroes is called :ref:`genvm-def-root-slot`. It uses `Default Derivation Algorithm`_ to store
the following data:


- ``contract_instance``: (offset 0) Reference to the contract instance data.
- ``code``: (offset 1) The contract's code. Slot contains 4 bytes little-endian length followed by data
- ``locked_slots``: (offset 2) A list of storage :term:`SlotID`\s that cannot be modified by non-upgraders.
    Slot contains 4 bytes little-endian length followed length arrays of 32 byte :term:`SlotID`\s
- ``upgraders``: (offset 3) A list of addresses that are authorized to modify the contract code and locked slots.
    Slot contains 4 bytes little-endian length followed length arrays of 20 byte addresses

Upgrade permissions and slot locking is described in :doc:`04-upgradability`
