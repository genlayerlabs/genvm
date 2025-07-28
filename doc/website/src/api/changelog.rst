Change Log
==========

.. run `git diff <previous tag> <next tag> -- runners/genlayer-py-std/` . Create user-facing changelog in doc/website/src/api/changelog.rst , but before writing analyze style and structure of existing contents to follow. Write small migration guide by looking into diff of `executor/testdata/cases/`. If there are no changes to include, omit this section

v0.1.8
------

Breaking Changes
~~~~~~~~~~~~~~~~

#. Calldata encoding behavior changed - dataclasses now encoded automatically without requiring custom ``default`` parameter
#. ``genlayer.py.public_abi`` file marked as auto-generated - manual edits will be overwritten

New Features
~~~~~~~~~~~~

#. **Enhanced calldata encoding**: Automatic dataclass encoding support without custom default functions
#. **Standardized error types**: New ``VmError`` enum for consistent error handling across the platform
#. **Extended public ABI**: Additional constants for internal VM operations (``CODE_SLOT_OFFSET``)

API Improvements
~~~~~~~~~~~~~~~~

#. Simplified dataclass serialization - no longer requires explicit ``default`` parameter in ``calldata.encode()``
#. Better type safety with standardized error enums
#. Cleaner API for encoding complex data structures

v0.1.3
------

Migration Guide
~~~~~~~~~~~~~~~

This section provides code examples for migrating from v0.1.0 to v0.1.3:

Contract Interfaces
^^^^^^^^^^^^^^^^^^^

.. code-block:: python

    # v0.1.0
    contract = gl.ContractAt(address)
    contract.view().some_method()
    contract.emit().some_method()

    # v0.1.3
    contract = gl.get_contract_at(address)
    contract.view().some_method()
    contract.emit().some_method()

Error handling
^^^^^^^^^^^^^^

.. code-block:: python

    # v0.1.0
    from genlayer import Rollback
    gl.advanced.rollback_immediate("error message")

    # v0.1.3
    from genlayer.gl.vm import UserError  # or use gl.vm.UserError
    gl.advanced.user_error_immediate("error message")

VM operations
^^^^^^^^^^^^^

.. code-block:: python

    # v0.1.0
    gl.advanced.run_nondet(leader_fn, validator_fn)

    # v0.1.3
    gl.vm.run_nondet(leader_fn, validator_fn)

EVM contracts
^^^^^^^^^^^^^

.. code-block:: python

    # v0.1.0
    @gl.eth_contract
    class MyEthContract:
        # contract definition

    # v0.1.3
    @gl.evm.contract_interface
    class MyEthContract:
        # contract definition

Non-deterministic functions
^^^^^^^^^^^^^^^^^^^^^^^^^^^

.. code-block:: python

    # v0.1.0
    result = gl.get_webpage(url, mode='text')
    response = gl.exec_prompt("prompt text")

    # v0.1.3
    result = gl.nondet.web.render(url, mode='text')
    response = gl.nondet.exec_prompt("prompt text")

Equivalence principles
^^^^^^^^^^^^^^^^^^^^^^

.. code-block:: python

    # v0.1.0
    result = gl.eq_principle_strict_eq(fn)
    result = gl.eq_principle_prompt_comparative(fn, principle)

    # v0.1.3
    result = gl.eq_principle.strict_eq(fn)
    result = gl.eq_principle.prompt_comparative(fn, principle)

Breaking Changes
~~~~~~~~~~~~~~~~

#. Removed ``Rollback`` exception from top-level imports - use ``genlayer.gl.vm.UserError`` instead
#. Error handling methods now use ``#error`` and ``#get-schema`` special method names from ``genlayer.py.public_abi.SpecialMethod``
#. Contract interface changes: ``eth_contract`` renamed to ``evm_contract_interface`` in EVM module

New Features
~~~~~~~~~~~~

#. **Module reorganization**: Core functionality moved to ``genlayer.gl`` namespace with lazy loading for better performance
#. **Contract interface system**: New ``@contract_interface`` decorator for type-safe contract interactions
#. **Enhanced contract deployment**: ``deploy_contract`` function with deterministic addressing via salt nonce
#. **Contract proxy system**: ``get_contract_at`` function returns proxy objects with ``view()`` and ``emit()`` methods
#. **Advanced event system**: ``Event`` class with proper topic generation and indexed field support
#. **VM operations**: New ``genlayer.gl.vm`` module with ``run_nondet``, ``spawn_sandbox`` functions
#. **EVM integration**: Enhanced ``genlayer.py.evm`` module with contract generation capabilities
#. **Equivalence principles**: New ``genlayer.gl.eq_principle`` module with ``strict_eq``, ``prompt_comparative``, ``prompt_non_comparative``
#. **Advanced utilities**: ``genlayer.gl.advanced`` module with ``user_error_immediate`` and ``emit_raw_event``

API Improvements
~~~~~~~~~~~~~~~~

#. Storage system improvements with better slot management
#. Type annotations added for WASI bindings (``_genlayer_wasi.pyi``)
#. Enhanced error handling with ``UserError`` replacing ``Rollback``
#. Better lazy loading system for improved import performance
#. Documentation generation support with ``GENERATING_DOCS`` environment variable

v0.1.0
------

Initial release
