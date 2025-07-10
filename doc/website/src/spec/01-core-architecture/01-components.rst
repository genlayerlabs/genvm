:term:`GenVM` Components Overview
==================================

Introduction
------------

GenVM is a WebAssembly-based virtual machine that executes intelligent contracts through a dual-mode execution model. The architecture separates deterministic blockchain operations from non-deterministic AI/web operations using a supervisor pattern with multiple isolated :term:`sub-VM`\s.

Architecture Overview
---------------------

The system consists of:

- **Supervisor**: Manages multiple :term:`sub-VM` instances for different execution contexts
- **:term:`Sub-VM` instances**: Execute code in deterministic mode, non-deterministic mode, or sandboxed environments
- **:term:`Runners <Runner>`**: Define execution environments and dependencies for contracts, supporting multiple formats (WASM, ZIP archives, text-based with headers)

The supervisor enforces resource limits, manages memory isolation between :term:`sub-VM` instances, and handles result validation through consensus mechanisms. :term:`Runners <Runner>` provide configurable execution environments that can depend on other :term:`runners <Runner>` and specify initialization actions for contract deployment.
