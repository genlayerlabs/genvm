.. _key-concepts:

Key Concepts
============

Intelligent Contracts
---------------------

Smart contracts that can perform non-deterministic operations such as accessing the web and calling LLMs, while still maintaining blockchain consensus. See :doc:`/spec/01-core-architecture/index` for the full specification.

Deterministic vs Non-deterministic Execution
---------------------------------------------

GenVM enforces deterministic execution by default. Non-deterministic operations (web access, LLM calls) are handled through controlled interfaces that allow validators to independently execute and then compare results. See :doc:`/spec/02-execution-environment/index` for execution details.

Runners
-------

Language-specific runtimes compiled to WebAssembly that serve as execution backends within GenVM. Each runner provides the standard library and runtime support for its language. Python is the currently supported runner. See :doc:`/impl-spec/01-core-architecture/index` for implementation details.

Equivalence Principle
---------------------

The mechanism by which validators agree on the outcome of non-deterministic operations. Rather than requiring identical outputs, validators check that their results are *equivalent* according to contract-defined criteria. See :doc:`/spec/02-execution-environment/index` for the specification.

Sub-VMs
-------

Isolated WebAssembly virtual machine instances within GenVM. Each Sub-VM provides a sandboxed execution context for contract code. See :doc:`/spec/03-vm/index` for VM architecture details.
