Contract Upgradability
======================

GenVM provides a native contract upgradability system that allows contracts to be modified after deployment while maintaining security guarantees and clear access controls.

Root Slot Structure
-------------------

Every contract has a special :py:class:`~genlayer.py.storage.Root` storage slot that controls upgrade permissions and metadata:

.. code-block:: python

   class Root:
       contract_instance: Indirection[None]
       code: Indirection[VLA[u8]]
       locked_slots: Indirection[VLA[u256]]
       upgraders: Indirection[VLA[Address]]

Fields
~~~~~~

**contract_instance**
   Reference to the contract instance data.

**code**
   The contract's executable code. This can be modified by authorized upgraders.

**locked_slots**
   A list of storage slot IDs that cannot be modified after deployment. By default, includes the root slot itself and all metadata slots.

**upgraders**
   A list of addresses that are authorized to modify the contract code and locked slots.

Upgrade Control Mechanism
-------------------------

The upgrade system works through access control during write transactions:

1. **Permission Check**: When a write transaction occurs, GenVM reads the ``upgraders`` list
2. **Slot Protection**: If the sender is not in the ``upgraders`` list, GenVM prevents writing to any slots listed in ``locked_slots``
3. **Code Execution**: GenVM reads the current ``code`` and executes it

Default Behavior
~~~~~~~~~~~~~~~~

By default, contracts are created in a **frozen state**:

- The ``upgraders`` list is empty (no one can upgrade)
- All metadata slots are locked (root slot, code, locked_slots, upgraders)

This ensures contracts are immutable unless explicitly configured otherwise.

Upgrade Patterns
----------------

Frozen Contracts
~~~~~~~~~~~~~~~~

The default state - completely immutable contracts:

.. code-block:: python

   class Contract(gl.Contract):
       def __init__(self):
           # Contract is frozen by default
           # upgraders = []
           # locked_slots contains all metadata slots
           pass

Upgradable Contracts
~~~~~~~~~~~~~~~~~~~~

Contracts that can be modified by specific addresses:

.. code-block:: python

   class Contract(gl.Contract):
       def __init__(self, owner: Address):
           root = gl.storage.Root.get()

           # Add owner as an upgrader
           root.upgraders.get().append(owner)

           # Optionally unlock specific slots for modification
           # (by default, metadata slots remain locked)

Development-to-Production
~~~~~~~~~~~~~~~~~~~~~~~~~

Contracts that start upgradable but become frozen:

.. code-block:: python

   class Contract(gl.Contract):
       def __init__(self, developers: list[Address]):
           root = gl.storage.Root.get()
           root.upgraders.get().extend(developers)

       @gl.public.write
       def freeze_contract(self):
           """Remove all upgraders, making contract immutable"""
           root = gl.storage.Root.get()

           # Clear upgraders list
           root.upgraders.get().truncate()

Integration with External Systems
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

The upgrade system can work with external contracts like timelocks or multisigs:

.. code-block:: python

   class Contract(gl.Contract):
       def __init__(self, timelock_address: Address):
           root = gl.storage.Root.get()

           # Use timelock contract as the upgrader
           root.upgraders.get().append(timelock_address)

Working with Root Storage
-------------------------

Accessing Root Slot
~~~~~~~~~~~~~~~~~~~

.. code-block:: python

   # Get the root slot
   root = gl.storage.Root.get()

Checking Upgrade Status
~~~~~~~~~~~~~~~~~~~~~~~

.. code-block:: python

   def is_upgradable(self) -> bool:
       """Check if contract can be upgraded"""
       root = gl.storage.Root.get()
       return len(root.upgraders.get()) > 0

   def get_upgraders(self) -> collections.abc.Sequence[Address]:
       """Get list of addresses that can upgrade this contract"""
       root = gl.storage.Root.get()
       return list(root.upgraders.get())

   def is_slot_locked(self, slot_id: int) -> bool:
       """Check if a specific slot is locked"""
       root = gl.storage.Root.get()
       return slot_id in root.locked_slots.get()

Modifying Contract Code
~~~~~~~~~~~~~~~~~~~~~~~

.. code-block:: python

   @gl.public.write
   def upgrade_code(self, new_code: bytes):
       """Upgrade contract code (only for authorized upgraders)"""
       root = gl.storage.Root.get()

       # GenVM automatically checks if sender is in upgraders
       # and prevents modification if not authorized

       code_storage = root.code.get()
       code_storage.clear()
       code_storage.extend(new_code)

Locking Additional Slots
~~~~~~~~~~~~~~~~~~~~~~~~

.. code-block:: python

   def lock_storage_slot(self, slot: u256):
       """Lock a storage slot to prevent future modifications"""
       root = gl.storage.Root.get()
       root.locked_slots.get().append(slot.as_int())

Security Considerations
-----------------------

Eval Safety
~~~~~~~~~~~

The upgrade system provides "eval safety" - even if malicious code tries to manipulate storage through :py:func:`eval` or similar mechanisms, the slot locking system prevents unauthorized modifications.

Storage Layout Compatibility
~~~~~~~~~~~~~~~~~~~~~~~~~~~~

When upgrading contracts, storage layout changes must be compatible with existing data, similar to Solidity's upgrade constraints:

- New fields can be added at the end
- Existing fields cannot be reordered or removed
- Field types should remain compatible

Transparency
~~~~~~~~~~~~

Users can verify a contract's upgrade status by reading its storage:

.. code-block:: python

   # Anyone can check upgrade permissions
   root = gl.storage.Root.get()
   upgraders = list(root.upgraders.get())
   locked_slots = list(root.locked_slots.get())

Best Practices
--------------

1. **Start Frozen**: Use the default frozen state unless upgradability is specifically needed
2. **Minimize Upgraders**: Keep the upgraders list as small as possible
3. **Use External Controls**: Consider timelocks or multisigs for upgrade authorization
4. **Plan Transitions**: Design upgrade paths from development to production
5. **Document Changes**: Maintain clear documentation of storage layout changes
6. **Test Thoroughly**: Verify upgrade compatibility before deploying changes

Example: Complete Upgradable Contract
-------------------------------------

.. code-block:: python

   class UpgradableCounter(gl.Contract):
       def __init__(self, owner: Address, initial_value: int = 0):
           # Set up upgrade permissions
           root = gl.storage.Root.get()
           root.upgraders.get().append(owner)

           # Initialize contract state
           self.counter = initial_value
           self.owner = owner

       @gl.public.view
       def get_counter(self) -> int:
           return self.counter

       @gl.public.write
       def increment(self):
           self.counter += 1

       @gl.public.write
       def upgrade_code(self, new_code: bytes):
           """Only owner can upgrade"""
           if gl.message.sender != self.owner:
               raise ValueError("Only owner can upgrade")

           root = gl.storage.Root.get()
           code_storage = root.code.get()
           code_storage.truncate()
           code_storage.extend(new_code)

       @gl.public.write
       def transfer_ownership(self, new_owner: Address):
           """Transfer upgrade permissions"""
           root = gl.storage.Root.get()
           upgraders = root.upgraders.get()
           upgraders.truncate()
           upgraders.append(new_owner)
           self.owner = new_owner

       @gl.public.write
       def renounce_upgradability(self):
           """Make contract immutable"""
           root = gl.storage.Root.get()
           root.upgraders.get().truncate()

This example demonstrates a complete upgradable contract with ownership transfer and the ability to renounce upgradability.
