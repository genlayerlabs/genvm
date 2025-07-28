Contract Upgradability
======================

:term:`GenVM` provides a native contract upgradability system that allows contracts to be modified after deployment
while maintaining security guarantees and clear access controls.

Data that is necessary for this process resides in :ref:`genvm-def-root-slot`\.

Upgrade Control Mechanism
-------------------------

The upgrade system works through access control during write transactions:

#. At start of execution :term:`GenVM` reads the ``upgraders`` list of :ref:`genvm-def-root-slot`\.
    It does not lead to :ref:`gvm-def-ram-consumption`
#. If the sender is not in the ``upgraders`` list of :ref:`genvm-def-root-slot`\,
    :term:`GenVM` reads ``locked_slots`` and will prevent writing to them.
    It implies :math:`32*n` :ref:`gvm-def-ram-consumption`\, where :math:`n` is the number of locked slots.
    This memory is never released.
#. :term:`GenVM` reads the ``code`` field of :ref:`genvm-def-root-slot` and executes it.
    It causes exactly ``code`` size in octets :ref:`gvm-def-ram-consumption`
