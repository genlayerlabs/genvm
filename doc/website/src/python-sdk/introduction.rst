Introduction
============

The ``genlayer`` package provides the Python SDK for writing intelligent
contracts that run inside GenVM. It includes storage primitives, contract
interfaces, non-deterministic operations, equivalence principles, and EVM
interoperability -- everything needed to build contracts in Python.

Under the hood, Python code executes via CPython compiled to WebAssembly as
a "runner" inside GenVM. The SDK abstracts the VM interface so contract
authors can write idiomatic Python while GenVM handles deterministic execution
and consensus.

For tutorials and getting started, see the
`main documentation <https://docs.genlayer.com/developers/intelligent-contracts/introduction>`_.
