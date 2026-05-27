Manager API
===========

The GenVM Manager is an HTTP server that provides an API for managing GenVM instances, modules, and related operations.

Two endpoints below are tightly coupled with the runner manifest published as
:doc:`available-runners` (generated from ``runners-versions.json``):

- ``POST /contract/detect-version`` returns the public-ABI ``specified_major`` that the
  node MUST persist into the contract's root-slot ``major`` field (see
  :doc:`/impl-spec/02-vm/02-version-management` and
  :doc:`/spec/04-contract-interface/03-storage`).
- ``POST /manifest/reload`` re-reads the runner manifest from disk; until reload, runner
  versions added to the JSON are not visible to the manager.

.. openapi:: manager-api.yaml

.. include:: manager-api.yaml
   :literal:
