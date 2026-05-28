Version Management
==================

This page describes how the GenVM implementation maps a *contract* (a blob of bytes
submitted by a deployer) and a *host* (a particular GenVM build, identified by its
``CURRENT_MAJOR`` and its runner manifest) onto a concrete set of WASM modules to execute.

There are two independent notions of "version" in play:

#. The **public ABI major** (``host_fns::CURRENT_MAJOR``, a ``u8``) — the wire-level
   contract between the host and contract code: calldata layout, storage layout,
   ``gl_call`` message shapes. A change here breaks contracts. Currently ``0``.
#. The **runner version** (a ``v<major>.<minor>.<patch>`` string) — the version of the
   bundled runtime (e.g. ``py-genlayer``) that a particular contract was built to load.
   Multiple runner versions may coexist for the same ABI major; selection happens at
   load time via the runner manifest.

Detection from contract bytes
-----------------------------

``executor/src/runners/parse.rs::detect_version_from_wasm`` walks the WASM custom sections
and returns the contents of the ``genvm.version`` section as the runner version string.
When the contract is not a raw WASM but a zip-packaged archive the version is read from
the ``version`` file in the archive. For single-file text contracts (Python source) the
first comment line is inspected: if it starts with ``v`` it is taken as the version
string, otherwise ``CURRENT_MAJOR_STR`` (``"v0.0.0"``) is used.

If detection fails, a warning is logged and ``CURRENT_MAJOR_STR`` is used as the fallback.

``POST /contract/detect-version``
---------------------------------

The manager exposes this detection logic via HTTP for the node to call before deploy.
The request body is the raw contract bytecode; the response is::

   { "specified_major": <u8> }

The node MUST store the returned value in the contract's :ref:`genvm-def-root-slot` at
offset ``4`` (the ``major`` field — see :doc:`/spec/04-contract-interface/03-storage`).
On every subsequent load :term:`GenVM` re-reads that byte and refuses to execute when it
does not match its own ``CURRENT_MAJOR``. Without this check a node running a newer
GenVM could silently mis-interpret an older contract's calldata or storage.

Runner manifest
---------------

The manager maintains an executor version manifest (the JSON document published as
:doc:`available-runners` and shipped at ``doc/website/src/impl-spec/appendix/runners-versions.json``).
For every supported runner version it lists the content hashes of every runtime artifact
(``cpython``, ``py-genlayer``, ``py-lib-genlayer-std``, ``softfloat``, ...). When a contract
selects a runner version (via its ``runner.json`` or the bundled `StartWasm` action) the
manager resolves each artifact name to a hash from this manifest and pins it. The manifest
can be hot-reloaded via ``POST /manifest/reload``.

Compatibility envelope
----------------------

The relationship between the two version notions is:

- ``CURRENT_MAJOR`` is checked **per load**, against the byte stored in the root slot.
- The runner version is checked **per resolution**: the manager refuses to start a contract
  whose runner version is absent from the active manifest, even if its ``major`` byte matches.

Mismatched ``major`` produces a :ref:`gvm-def-vm-error`; missing runner produces an
:ref:`gvm-def-internal-error` from the manager. Both are reported through the standard host
error channel.
