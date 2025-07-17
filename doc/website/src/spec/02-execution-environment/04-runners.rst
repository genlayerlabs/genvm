:term:`Runners <Runner>`
========================

:term:`Runners <Runner>` specify execution environments for GenVM contracts.

:term:`Runner` Architecture
---------------------------

Identification and Packaging
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Each :term:`runner` is identified by ``<human-readable-id>:<hash>``.
``human-readable-id`` is provided for convenience.
``hash`` is a sha3-256 hash of its contents

Hash Format
~~~~~~~~~~~

Hash is SHA3 256-bit hash, converted to a string with following algorithm:

.. code-block:: python

    def digest_to_hash_id(got_hash: bytes) -> str:
        chars = '0123456789abcdfghijklmnpqrsvwxyz'

        bytes_count = len(got_hash)
        base32_len = (bytes_count * 8 - 1) // 5 + 1

        my_hash_arr = []
        for n in range(base32_len - 1, -1, -1):
            b = n * 5
            i = b // 8
            j = b % 8
            c = (got_hash[i] >> j) | (0 if i >= bytes_count - 1 else got_hash[i + 1] << (8 - j))
            my_hash_arr.append(chars[c & 0x1F])

        return ''.join(my_hash_arr)

This ensures that it contains no fs-illegal characters and is case insensitive.

Runner Layout
-------------

For any of the layouts a file list is constructed. Each file in this name has:

- a name

    #. it must not start with ``/``
    #. path separator must be ``/``
    #. it must not contain ``.`` or ``..`` path components

- contents (raw bytes slice)

1. ZIP Archive
~~~~~~~~~~~~~~

Used if runner bytes represent a ZIP archive

- If successful, extracts the archive contents and processes it as a structured :term:`runner` package
- This format supports complex :term:`runners <Runner>` with multiple files, dependencies, and configuration
- Only allowed compression is ``stored`` (no compression)

2. Raw WASM
~~~~~~~~~~~

Used if runner bytes represent a wasm file (magic matches)

Creates a minimal :term:`runner` configuration

.. code-block::

    version = v0.1.0
    runner.json = { "StartWasm": "file" }
    file = # source bytes

3. Text-based
~~~~~~~~~~~~~

Used if neither of previous worked. Must be a valid utf-8 encoded string

Comment Header Format
^^^^^^^^^^^^^^^^^^^^^

The contract source code must begin with comment lines using one of the supported comment syntaxes:

- ``//`` (C-style comments)
- ``#`` (Shell/Python-style comments)
- ``--`` (SQL/Lua/Haskell-style comments)

The comment header consists of:

#. **Version Line** (first comment line): Must start with ``v`` followed by version information
#. **:term:`Runner` Configuration** (subsequent comment lines): JSON configuration for the :term:`runner`

Resulting structure
^^^^^^^^^^^^^^^^^^^

.. code-block::

    version = # first line if started with version, else default
    runner.json = # consequent comment lines with removed comment prefix. All whitespaces are kept as-is
    file = # source bytes

Example
^^^^^^^

.. code-block:: python

   # v1.0.0
   # {
   #   "Depends": "python:latest",
   #   "StartWasm": "python.wasm"
   # }

   exit(30)

``version`` file
----------------
This file must contain a single line with the version of ``genvm`` in ``v<major>.<minor>.<patch>`` format.

If this file is not present, the default version is used.

``runner.json`` File
--------------------

The ``runner.json`` file defines a recursive structure of initialization actions that configure the execution environment for a contract.

Schema is available in :doc:`../appendix/runner-schema`\.

It must be a valid JSON object with described below structure

AddEnv
~~~~~~

Adds an environment variable to the GenVM environment with variable interpolation support using ``${}`` syntax.

Example
^^^^^^^

.. code-block:: json

   {
       "AddEnv": {
           "name": "DEBUG",
           "val": "true"
       }
   }

MapFile
~~~~~~~

Maps files or directories from an archive to specific paths in the GenVM filesystem.

Properties
^^^^^^^^^^

- ``file`` (string): Path within the archive. If ending with ``/``, recursively maps all files in the directory
- ``to`` (string): Absolute destination path in the GenVM filesystem

Example
^^^^^^^

.. code-block:: json

   {
       "MapFile": {
           "file": "config/",
           "to": "/etc/myapp/"
       }
   }

Creating a single file mapping implies :ref:`gvm-def-ram-consumption` of :ref:`gvm-def-enum-value-memory-limiter-consts-file-mapping`\.

SetArgs
~~~~~~~

Sets process arguments for the GenVM environment.

**Type:** Array of strings

Example
^^^^^^^

.. code-block:: json

   {
       "SetArgs": ["exe-name", "--verbose", "--config", "/path/to/config"]
   }

LinkWasm
~~~~~~~~

Links a WebAssembly file to make it available in GenVM.

**Type:** String (path to WebAssembly file)

.. code-block:: json

   {
       "LinkWasm": "path/in/arch/to/module.wasm"
   }

If function _initialize is present, it will be called immediately after linking.

.. _gvm-def-start-wasm:

StartWasm
~~~~~~~~~

Starts a specific WebAssembly file in GenVM.

**Type:** String (path to WebAssembly file)

Example
^^^^^^^

.. code-block:: json

   {
       "StartWasm": "path/in/arch/to/module.wasm"
   }

This is a terminal action in the runner configuration. It results in linking the module and calling ``_start`` function.

Depends
~~~~~~~

Specifies a dependency on another :term:`runner` by its ID and hash.

Example
^^^^^^^

.. code-block:: json

   {
       "Depends": "cpython:123"
   }

Dependencies are processed only once, for the first request

Seq
~~~

Executes a sequence of initialization actions.

.. code-block:: json

   {
       "Seq": [
           { "SetArgs": ["exe-name", "--verbose", "--config", "/path/to/config"] },
           { "StartWasm": "path/in/arch/to/module.wasm" }
       ]
   }

When
~~~~

Conditionally executes an action based on WebAssembly execution mode.

Properties
^^^^^^^^^^

- ``cond``: WebAssembly mode, either ``det`` (deterministic) or ``nondet`` (non-deterministic)
- ``action``: Action to execute when condition is met

Example
^^^^^^^

.. code-block:: json

   {
       "When": {
           "cond": "det",
           "action": { "AddEnv": {"name": "MODE", "val": "deterministic"} }
       }
   }

With
~~~~

Sets a :term:`runner` as current without executing its action, useful for reusing files or creating :term:`runner` locks.

Example
^^^^^^^

.. code-block:: json

   {
       "With": {
           "runner": "base-environment",
           "action": { "MapFile": {"file": "patched.foo", "to": "foo" } }
       }
   }

Startup
-------

Runner actions are executed left-recursively, until :ref:`gvm-def-start-wasm` is reached.
If it was not reached, it will result in a :ref:`gvm-def-vm-error` with ``error_inval`` code.
