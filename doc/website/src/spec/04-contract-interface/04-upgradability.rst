Contract Upgradability
======================

:term:`GenVM` provides a native contract upgradability system that allows contracts to be modified after deployment
while maintaining security guarantees and clear access controls.

Data that is necessary for this process resides in :ref:`genvm-def-root-slot`\.

Upgrade Control Mechanism
-------------------------

The upgrade system works through access control during write transactions:

#. At start of execution :term:`GenVM` reads the ``upgraders`` list
#. If the sender is not in the ``upgraders`` list, :term:`GenVM` reads ``locked_slots`` and will prevent writing to them
#. :term:`GenVM` reads the ``code`` and executes it
