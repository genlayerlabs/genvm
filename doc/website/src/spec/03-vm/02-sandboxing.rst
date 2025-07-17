Sandboxing
==========

For the sake of executing prompt-generated code,
 users are provided with ability to spawn a separate :term:`sub-VM` for executing it.

This :term:`sub-VM`:

#. Has the same non-deterministic level as parent :term:`sub-VM`
#. Can not switch into non-deterministic mode
#. Can be configured to be able to update storage (privilege escalation is forbidden)

Users can catch both :ref:`gvm-def-vm-error` and :ref:`gvm-def-user-error` produced by it, but storage writes can not be reverted
