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

.. _gvm-def-locked-slot-nesting:

Locked Slots in Nested Calls
----------------------------

Each :term:`sub-VM` reads the ``upgraders`` and ``locked_slots`` of *its own* contract address
(the callee), not the caller's. There is no transitive inheritance: a parent VM cannot
relax or tighten its child's locked slots, and an upgrader on contract ``A`` is not implicitly
an upgrader on contract ``B`` that ``A`` calls.

Two consequences:

#. Permission to overwrite a locked slot is determined by ``msg.sender`` *as seen by the
   callee*. For a ``CallContract`` invocation, that is the calling contract's address — the
   caller must be in the callee's ``upgraders`` list for any locked-slot writes to succeed.
#. The :math:`32\cdot n` :ref:`gvm-def-ram-consumption` of materializing ``locked_slots`` is
   paid independently in every sub-VM that needs it and is bounded by
   ``top_limits::LOCKED_SLOTS``. The memory is never released for the lifetime of that sub-VM.

Sub-VMs created by ``Sandbox`` and ``RunNondet`` cannot reach storage at all
(see :ref:`gvm-permissions`), so locked-slot checking does not apply to them.

.. _gvm-def-contract-version:

Contract Major Version
----------------------

The ``major`` field of :ref:`genvm-def-root-slot` (offset 0) stores the public-ABI major version
that the contract was built against. On every load :term:`GenVM` compares this byte to its own
``CURRENT_MAJOR`` constant and refuses to execute a contract whose major differs.

The value is **not** modifiable by the contract itself: it is written once at deploy time by
the host from a value detected in the contract package (a ``genvm.version`` custom WASM
section, a ``version`` file inside a zip-packaged runner, or a leading ``// vX.Y.Z`` comment
in single-file text contracts). The host detection flow and the on-the-wire endpoint
(``POST /contract/detect-version``) are described in the impl-spec.

Implications for upgrades:

- Replacing ``code`` with a binary of a different public-ABI major leaves ``major``
  stale; the contract will fail to load on subsequent invocations. Upgraders MUST update
  ``major`` in the same transaction when crossing a major boundary.
- Bumping ``major`` to a value the running :term:`GenVM` does not support produces a
  load-time error; the upgrade transaction must be performed on a host that already supports
  the target major.
