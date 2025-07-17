WASI Preview 1 Implementation
=============================

Overview
--------

GenVM implements WebAssembly System Interface (WASI) Preview 1 to
provide standardized system-level functionality to WebAssembly modules.
The implementation includes modifications for deterministic execution
required by blockchain consensus while maintaining compatibility with
standard WASI applications.

WASI Preview 1 Foundation
-------------------------

Standard Interface
~~~~~~~~~~~~~~~~~~

-  **System Calls**:

   -  File system operations (open, read, write, close)
   -  Process management (exit, args, environment)
   -  Time and clock access
   -  Random number generation
   -  Socket and network operations

-  **Data Types**:

   -  Standard WASI types for file descriptors, time, and sizes
   -  Cross-platform compatibility abstractions
   -  Error code standardization
   -  Memory layout specifications

Deterministic Modifications
---------------------------

Time and Randomness Control
~~~~~~~~~~~~~~~~~~~~~~~~~~~

-  **Controlled Time Access**:

   -  Deterministic time functions for consensus requirements
   -  Time zone and locale standardization

-  **Deterministic Randomness**:

   -  Deterministic randomness for deterministic operations
   -  Cryptographically secure random number generation in non-deterministic mode

Regular system interface
~~~~~~~~~~~~~~~~~~~~~~~~

- **Virtual File System (VFS)**:

  -  Isolated file system namespace per contract execution
  -  Memory-based file system for deterministic behavior
  -  Read-only access to runtime libraries and dependencies
  -  Controlled file system state for reproducible execution

-  **Environment Variables**:

   -  Controlled environment variable access
   -  Deterministic environment setup
   -  Security filtering of sensitive variables
   -  Standardized locale and language settings

-  **Command Line Arguments**:

   -  Controlled argument passing to WebAssembly modules
   -  Deterministic argument parsing and validation
   -  Security filtering of dangerous arguments
   -  Standardized argument format and encoding

WASI Specification Compliance
-----------------------------

-  **Interface Compatibility**:

   -  Full compatibility with WASI Preview 1 specification
   -  Standard function signatures and behavior
   -  Compatible error handling and reporting
   -  Consistent data type definitions

-  **Ecosystem Integration**:

   -  Support for WASI-targeting compilers
   -  Compatibility with existing WASI libraries
   -  Tool chain integration and support
   -  Community standard compliance

Always Erroring Operations
--------------------------

Fail with ``Acces`` error code:

- ``sock_accept``
- ``sock_recv``
- ``sock_send``
- ``sock_shutdown``

Fail with ``Rofs`` error code:

- ``fd_allocate``
- ``fd_fdstat_set_flags``
- ``fd_fdstat_set_rights``
- ``fd_filestat_set_size``
- ``fd_filestat_set_times``
- ``path_create_directory``
- ``path_filestat_set_times``
- ``path_link``
- ``path_remove_directory``
- ``path_symlink``
- ``path_unlink_file``

Fail with ``Badf`` error code:

- ``path_readlink``

Fail with ``Notsup`` error code:

- ``poll_oneoff``
- ``proc_raise``
- ``sched_yield``
- ``fd_pwrite``


Functions
---------

``random_get`` Function
~~~~~~~~~~~~~~~~~~~~~~~

Deterministic mode: mt19937 that is initialized with ``GenLayer`` as ascii bytes

Non-deterministic mode: cryptographically secure random number generator,
with optional fallback to pseudo-random numbers, if secure source is exhausted or unavailable.

``proc_exit`` Function
~~~~~~~~~~~~~~~~~~~~~~

#. ``proc_exit(0)`` is equivalent to :ref:`gvm-def-return` of ``null`` value.
#. ``proc_exit(x)`` where :math:`x \neq 0` results in :ref:`gvm-def-vm-error`

``path_open`` Function
~~~~~~~~~~~~~~~~~~~~~~

``path_filestat_get`` Function
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

``fd_readdir`` Function
~~~~~~~~~~~~~~~~~~~~~~~

``fd_tell`` Function
~~~~~~~~~~~~~~~~~~~~

``fd_datasync`` Function
~~~~~~~~~~~~~~~~~~~~~~~~

Does nothing and always returns success.

``fd_sync`` Function
~~~~~~~~~~~~~~~~~~~~

Does nothing and always returns success.

``fd_seek`` Function
~~~~~~~~~~~~~~~~~~~~

``fd_renumber`` Function
~~~~~~~~~~~~~~~~~~~~~~~~

``fd_prestat_dir_name`` Function
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

``fd_prestat_get`` Function
~~~~~~~~~~~~~~~~~~~~~~~~~~~

#. If file descriptor does not exist, returns ``Badf`` error code
#. Returns ``Notsup`` otherwise

``fd_write`` Function
~~~~~~~~~~~~~~~~~~~~~

``fd_pread`` Function
~~~~~~~~~~~~~~~~~~~~~

``fd_read`` Function
~~~~~~~~~~~~~~~~~~~~

``fd_filestat_get`` Function
~~~~~~~~~~~~~~~~~~~~~~~~~~~~

``fd_fdstat_get`` Function
~~~~~~~~~~~~~~~~~~~~~~~~~~

``fd_close`` Function
~~~~~~~~~~~~~~~~~~~~~

``fd_advise`` Function
~~~~~~~~~~~~~~~~~~~~~~

Does nothing and always returns success.

``clock_time_get`` Function
~~~~~~~~~~~~~~~~~~~~~~~~~~~

Returns transaction unix timestamp in **both** modes

``clock_res_get`` Function
~~~~~~~~~~~~~~~~~~~~~~~~~~

Always returns ``1``

``environ_sizes_get`` Function
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

``environ_get`` Function
~~~~~~~~~~~~~~~~~~~~~~~~

``args_sizes_get`` Function
~~~~~~~~~~~~~~~~~~~~~~~~~~~

``args_get`` Function
~~~~~~~~~~~~~~~~~~~~~

Virtual File System
-------------------

Initial State
~~~~~~~~~~~~~

- :term:`FD` 0 is a file that contains :ref:`Calldata Encoded <gvm-def-calldata-encoding>` extended message
- :term:`FD` 1 is ``stdout``
- :term:`FD` 2 is ``stderr``
- :term:`FD` 3 is directory ``/`` (file system root)

.. _Fd Allocation and Deallocation:

:ref:`gvm-def-det-mode` :term:`FD` Allocation and Deallocation
--------------------------------------------------------------

Pseudocode
~~~~~~~~~~

.. code-block::

   allocate() → FD:
      if free_pool.is_empty():
         consume_ram()
         next_id += 1
         allocated.insert(next_id)
         return next_id
      else:
         fd = free_pool.pop()
         allocated.insert(fd)
         return fd

   deallocate(fd: FD):
      require: fd ∈ allocated
      allocated.remove(fd)
      free_pool.push(fd)

Allocating a new :term:`FD` implies :ref:`gvm-def-ram-consumption` of :ref:`gvm-def-enum-value-memory-limiter-consts-fd-allocation`\.

Invariants
~~~~~~~~~~

#. :math:`\texttt{allocated}\cap\texttt{free_pool} = \emptyset`
#. :math:`\texttt{next_id} \ge \operatorname{max}(\texttt{allocated}\cup\texttt{free_pool})`
#. All returned descriptors are unique until deallocated

.. warning::

   :ref:`gvm-def-non-det-mode` is not obligated to follow this pattern
