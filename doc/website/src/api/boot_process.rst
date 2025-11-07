Startup Process
===============

Standard library provides a built-in bootloader that bridges :ref:`internal ABI <contract-execution-flow>`
to a more Pythonic interface.

Bootloader is located in module ``_genlayer_runner`` and is executed on import.

Bootloader Module
-----------------

``_genlayer_runner`` handles contract execution in three distinct modes:

Entry Points
~~~~~~~~~~~~

The bootloader dispatches execution based on ``entry_kind`` from the VM message:

MAIN
^^^^
Handles standard contract method calls and initialization.

* Loads user ``contract`` module and resolves the target method of a declared ``Contract`` class
* Validates method access permissions (public/private, payable)
* Handles special methods: ``__receive__``, ``__handle_undefined_method__``, ``__on_errored_message__``
* Routes initialization calls to ``__init__`` method
* Enforces security restrictions on dunder methods

SANDBOX
^^^^^^^
Executes pickled function objects.

CONSENSUS_STAGE
^^^^^^^^^^^^^^^
Runs consensus stage functions with additional stage data parameter (leader non-deterministic blocks outputs or ``None``).

Method Resolution
~~~~~~~~~~~~~~~~~

For MAIN entry kind, the bootloader implements comprehensive method resolution:

* **Initialization**: Routes to private ``__init__`` method during contract deployment
* **Named Methods**: Resolves public method calls by name with validation
* **Special Handling**: Supports ``__receive__`` for direct calls and ``__handle_undefined_method__`` for undefined method calls
* **Security**: Blocks calls to methods starting with ``__`` and unknown special methods starting with ``#``

Error Handling
~~~~~~~~~~~~~~

The bootloader provides centralized error management:

* Catches :py:class:`genlayer.gl.vm.UserError` exceptions and triggers rollback with error message
* Validates contract structure and method accessibility
* Returns appropriate error messages for invalid method calls

Profiling Support
~~~~~~~~~~~~~~~~~

Optional performance profiling when ``GENLAYER_ENABLE_PROFILER`` environment variable is set to ``true``:

* Uses ``cProfile`` with microsecond timing precision
* Outputs compressed, base64-encoded statistics to stderr on exit
* Integrates with :term:`GenVM` :ref:`debugging infrastructure <tracing-runtime-microsec>`

Storage Integration
~~~~~~~~~~~~~~~~~~~

Manages contract storage lifecycle:

* Locks default :term:`Storage Slot`\s during contract initialization
* Provides contract instance from the root slot
