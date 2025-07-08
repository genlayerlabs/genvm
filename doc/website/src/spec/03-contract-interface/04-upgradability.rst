Contract Upgradability
======================

:term:`GenVM` provides a native contract upgradability system that allows contracts to be modified after deployment while maintaining security guarantees and clear access controls.

Root Slot Structure
-------------------

Every contract has a special :py:class:`~genlayer.py.storage.Root` storage slot that controls upgrade permissions and metadata. This :term:`storage slot` resides at zero :term:`SlotID`.

Fields
~~~~~~

- **contract_instance:** (offset 0) Reference to the contract instance data.
- **code:** (offset 1) The contract's runner. Slot contains 4 bytes little-endian length followed by data
- **upgraders:** (offset 3) A list of addresses that are authorized to modify the contract code and locked slots.
    Slot contains 4 bytes little-endian length followed length arrays of 20 byte addresses
- **locked_slots:** (offset 2) A list of storage slot IDs that cannot be modified by non-upgraders.
    Slot contains 4 bytes little-endian length followed length arrays of 32 byte :term:`SlotID`\s

Upgrade Control Mechanism
-------------------------

The upgrade system works through access control during write transactions:

#. At start of execution :term:`GenVM` reads the ``upgraders`` list
#. If the sender is not in the ``upgraders`` list, :term:`GenVM` reads ``locked_slots`` and will prevent writing to them
#. :term:`GenVM` reads the ``code`` and executes it
