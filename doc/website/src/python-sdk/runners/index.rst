Runners
=======

Runners are language runtimes compiled to WebAssembly that execute inside GenVM.
For the Python SDK, this includes CPython and supporting libraries (cloudpickle,
protobuf, embeddings, etc.) compiled to WASM.

Each runner is identified by a content hash. When GenVM executes a contract, it
loads the specific runner versions declared in the contract's dependencies.

Using Runners
-------------

In your contract's first line, specify the runner dependency:

.. code-block:: python

   # { "Depends": "py-genlayer:1jb45aa8ynh2a9c9xn3b7qqh8sm5q93hwfp7jqmwsfhh8jpz09h6" }

For multi-file contracts, use ``py-genlayer-multi`` instead.

For contracts using embeddings or semantic search, add ``py-lib-genlayer-embeddings``
before the main runner using a Seq block:

.. code-block:: python

   # {
   #   "Seq": [
   #     { "Depends": "py-lib-genlayer-embeddings:<hash>" },
   #     { "Depends": "py-genlayer:<hash>" }
   #   ]
   # }

For the full runner specification, see :doc:`/spec/02-execution-environment/04-runners`.

For version history and what changed, see the :doc:`Changelog </python-sdk/changelog>`.

For list of available runners and their hashes, see the :doc:`Available Runners </impl-spec/appendix/available-runners>` in the appendix.
