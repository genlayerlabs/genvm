.. _gvm-permissions:

Permissions
===========

Each :term:`sub-VM` instance has a set of boolean permissions that control what operations it can perform.
Permissions are inherited from the parent VM when spawning child :term:`sub-VM` instances, with certain restrictions applied depending on the context.

.. _gvm-perm-deterministic:

``deterministic``
-----------------

When set, the VM is executing in :ref:`gvm-def-det-mode`.
Many operations require this permission, including storage writes, sending messages, calling other contracts, and emitting events.

.. _gvm-perm-read-storage:

``read_storage``
----------------

Allows reading contract storage slots. When unset, any attempt to read storage will fail with a ``Forbidden`` error.

.. _gvm-perm-write-storage:

``write_storage``
-----------------

Allows writing to contract storage slots. Requires :ref:`gvm-perm-deterministic` as well.

This permission also gates ``EmitEvent``: a VM may emit events *iff* it can write storage, since events are state-mutating log emissions. Consequently a read-only (static) context, such as a ``CallContract`` child, cannot emit events.

.. _gvm-perm-send-messages:

``send_messages``
-----------------

Allows sending messages to other addresses. This permission is required by ``EthSend``, ``PostMessage``, and ``DeployContract`` operations.
Requires :ref:`gvm-perm-deterministic` as well.

.. _gvm-perm-call-others:

``call_others``
---------------

Allows calling other contracts. This permission is required by ``EthCall`` and ``CallContract`` operations.
Requires :ref:`gvm-perm-deterministic` as well.

.. _gvm-perm-spawn-nondet:

``spawn_nondet``
----------------

Allows spawning :ref:`gvm-def-non-det-mode` :term:`sub-VM` instances via ``RunNondet``.

.. _gvm-perm-register-runners:

``register_runners``
--------------------

Allows registering runner archives at runtime via ``RegisterRunner``, making
them available under ``custom:<hash>`` runner ids. Requires :ref:`gvm-perm-deterministic` as well.

Permission Changes on Sub-VM Creation
--------------------------------------

Different operations modify permissions when creating child :term:`sub-VM` instances:

``CallContract``
~~~~~~~~~~~~~~~~

A ``CallContract`` child runs as a read-only (static) call. It inherits all parent permissions except:

- :ref:`gvm-perm-write-storage` is **disabled**
- :ref:`gvm-perm-send-messages` is **disabled**

A static call must not produce externally visible effects, so storage writes, message sends, and event emissions all fail: emitting an event additionally requires :ref:`gvm-perm-write-storage` (disabled here). Allowing any such emission would charge fees for effects that are discarded together with the child VM.

``RunNondet``
~~~~~~~~~~~~~

The non-deterministic :term:`sub-VM` has:

- :ref:`gvm-perm-deterministic` is **disabled**
- :ref:`gvm-perm-read-storage` is **inherited**
- :ref:`gvm-perm-write-storage` is **disabled**
- :ref:`gvm-perm-spawn-nondet` is **disabled**
- :ref:`gvm-perm-call-others` is **disabled**
- :ref:`gvm-perm-send-messages` is **disabled**

``Sandbox``
~~~~~~~~~~~

The sandboxed :term:`sub-VM` has:

- :ref:`gvm-perm-deterministic` is **inherited**
- :ref:`gvm-perm-read-storage` is **inherited**
- :ref:`gvm-perm-write-storage` is **inherited** *iff* ``allow_write_storage`` is set, otherwise **disabled**
- :ref:`gvm-perm-spawn-nondet` is **disabled**
- :ref:`gvm-perm-call-others` is **disabled**
- :ref:`gvm-perm-send-messages` is **inherited** *iff* ``allow_send_messages`` is set, otherwise **disabled**
- :ref:`gvm-perm-register-runners` is **inherited** *iff* ``allow_register_runners`` is set, otherwise **disabled**
