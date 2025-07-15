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

.. _Fd Allocation and Deallocation:

:term:`FD` Allocation and Deallocation
--------------------------------------

.. code-block::

   allocate() → FD:
      if free_pool.is_empty():
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

Invariants:

- allocated ∩ free_pool = ∅ (no descriptor in both sets)
- next_id ≥ max(allocated ∪ free_pool) (counter never decreases)
- All returned descriptors are unique until deallocated

Functions specification
-----------------------

Always Erroring Operations
~~~~~~~~~~~~~~~~~~~~~~~~~~

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


``random_get``
~~~~~~~~~~~~~~

Deterministic mode: mt19937 that is initialized with "GenLayer" as ascii bytes

Non-deterministic mode: cryptographically secure random number generator,
with optional fallback to pseudo-random numbers, if secure source is exhausted or unavailable.
