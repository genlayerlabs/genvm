.. _runners-reference:

Runners
=======

It is concept that is similar to libraries

Each runner is identified by ``<human-readable-id>:<hash>`` and is a set of files, with a single mandatory one: ``runner.json``

Runner json has a recursive structure and supports following actions

AddEnv
------
Add an environment variable to the GenVM environment. It supports variable interpolation using ``${}`` syntax to access existing environment variables

.. code-block::
    :caption: Example

        {
            "AddEnv": {
                "name": "DEBUG",
                "val": "true"
            }
        }

MapFile
-------
Map a file or directory from an archive to a specific path in the GenVM filesystem.
Properties:

file (string): Path within an archive

If the path ends with /, it recursively maps all files in that directory


to (string): Absolute destination path in the GenVM filesystem

.. code-block::
    :caption: Example

        {
            "MapFile": {
                "file": "config/",
                "to": "/etc/myapp/"
            }
        }

SetArgs
-------

Set process arguments for the GenVM environment.
Type: Array of strings

.. code-block::
    :caption: Example

        {
            "SetArgs": ["exe-name", "--verbose", "--config", "/path/to/config"]
        }

LinkWasm
--------

Link a WebAssembly file to make it available in GenVM.
Type: String (path to Wasm file)

.. code-block::
    :caption: Example

        {
            "LinkWasm": "path/in/arch/to/module.wasm"
        }

StartWasm
---------

Start a specific WebAssembly file in GenVM.
Type: String (path to Wasm file)

.. code-block::
    :caption: Example

        {
            "StartWasm": "path/in/arch/to/module.wasm"
        }

Depends
-------

Specify a dependency on another runner by its ID and hash.

.. code-block::
    :caption: Example

        {
            "Depends": "cpython:123"
        }

Seq
---
Execute a sequence of initialization actions.

.. code-block::
    :caption: Example

        {
            "Seq": [
                { "SetArgs": ["exe-name", "--verbose", "--config", "/path/to/config"] },
                { "StartWasm": "path/in/arch/to/module.wasm" }
            ]
        }

When
----

Conditionally executes an action based on Wasm mode.

``cond`` property is a WebAssembly mode, either "det" (deterministic) or "nondet" (non-deterministic)

.. code-block::
    :caption: Example

        {
            "When": {
                "cond": "det",
                "action": { "AddEnv": {"name": "MODE", "val": "deterministic"} }
            }
        }

With
----
Set a runner as current without executing its action, useful for reusing files or creating runner "locks".

.. code-block::
    :caption: Example

        {
            "With": {
                "runner": "base-environment",
                "action": { "MapFile": {"file": "patched.foo", "to": "foo" } }
            }
        }
