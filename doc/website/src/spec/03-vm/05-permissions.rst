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

Permission Changes on Sub-VM Creation
--------------------------------------

Different operations modify permissions when creating child :term:`sub-VM` instances:

``CallContract``
~~~~~~~~~~~~~~~~

Inherits all parent permissions except:

- :ref:`gvm-perm-write-storage` is **disabled**

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
- :ref:`gvm-perm-write-storage` is **inherited** if ``allow_write_ops`` is set, otherwise **disabled**
- :ref:`gvm-perm-spawn-nondet` is **disabled**
- :ref:`gvm-perm-call-others` is **disabled**
- :ref:`gvm-perm-send-messages` is **inherited** if ``allow_write_ops`` is set, otherwise **disabled**
