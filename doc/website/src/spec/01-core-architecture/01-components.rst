:term:`GenVM` Components Overview
==================================

Introduction
------------

:term:`GenVM` is a WebAssembly-based virtual machine that enables “Intelligent
Contracts” - smart contracts capable of performing non-deterministic
operations (AI inference, web scraping, real-world data access) while
maintaining blockchain consensus. This document provides an
architectural overview of :term:`GenVM`'s major components and how they work
together.

High-Level Architecture
-----------------------

.. mermaid::

   graph LR
      subgraph Host
         Storage
         LO["Leader's non-det outputs"]
         Storage ~~~ LO
         LO ~~~ Messages["Emitted messages, ..."]
      end
      subgraph GenVM
         subgraph Runners
               Libs@{ shape: docs, label: "Other: libs, model weights" }
               CPython["CPython (wasm build)"]
               gl["genlayer py sdk"]
               CPython -.-> gl
         end
         WASI -.-> CPython
         subgraph Executor
               wasmtime
               subgraph WASI
                  preview1["preview1 standard"]
                  gwasi["genlayer sdk"]
               end
               WASI <---> wasmtime
         end
         subgraph Modules
               web
               llm
         end
         gwasi ---> Modules
      end
      Program["Contract"]
      gl ~~~ Program
      Host <---> WASI
      Program <---> wasmtime
      Runners -.-> Program

:term:`GenVM` Executor
----------------------

The :term:`GenVM` Executor is the heart of the system, providing a modified
WebAssembly runtime with blockchain-specific capabilities. Executor itself is a supervisor of :term:`sub-VM`\s.

**Key Responsibilities:**

- Contract execution in deterministic and non-deterministic modes
- RAM management (memory)
- State management and storage operations
- Communication with :term:`modules <Module>` and the :term:`host`

**Major Subcomponents:**

- **VM Core**: Dual-mode WebAssembly execution engine
- **WASI Implementation**: Standard and GenLayer-specific system interfaces
- **:term:`Host` Functions**: Bridge between contracts and the :term:`host` environment
- **Caching System**: Module compilation and execution optimization

:term:`Sub-VM`
~~~~~~~~~~~~~~

:term:`GenVM`'s unique dual execution model is implemented by using multiple wasm :term:`sub-VM`\s.

**Deterministic Mode:** - Executes blockchain consensus logic - Provides
reproducible results across all validators - Handles storage operations,
message passing, and standard computation

**Non-Deterministic Mode:** - Executes AI inference, web scraping, and
external data access - Results are validated through consensus
mechanisms - Isolated from deterministic state to prevent contamination

WASI Interfaces
~~~~~~~~~~~~~~~

:term:`GenVM` exposes two WebAssembly System Interfaces:

**WASI Preview 1 (``wasip1``)** - Standard WASI interface with
deterministic modifications - File system operations, environment
access, time functions - Modified to ensure reproducible behavior across
validators

**:term:`GenLayer WASI SDK` (``genlayer_sdk``)** - Blockchain-specific operations and
primitives - Storage access, message passing, contract deployment -
Non-deterministic operation triggers and validation

:term:`Modules <Module>`
~~~~~~~~~~~~~~~~~~~~~~~~

:term:`Modules <Module>` provide non-deterministic capabilities through isolated
services:

**LLM :term:`Module`** - Large Language Model inference capabilities - Supports
multiple AI providers and models - Configurable prompts and response
processing - Support for :term:`greyboxing`

**Web :term:`Module`** - Web scraping and HTTP request capabilities - Webpage
rendering and content extraction - Domain filtering and security
controls

They are separated from executor for following reasons:

- replace-ability
- privileges containment

Runners (libraries)
~~~~~~~~~~~~~~~~~~~

Language runtimes provide the execution environment for different
programming languages:

**Python Runtime** - Custom CPython build compiled to WebAssembly with
software floating point implementation for deterministic mode

- GenLayer Python SDK for blockchain primitives
- Curated standard library for deterministic execution
- Support for some necessary libraries (NumPy, PIL)

GenVM requires some built-in runners to be accessible by contracts. They are identified by hashes of their ``tar`` contents

:term:`Host` Interface
~~~~~~~~~~~~~~~~~~~~~~

The :term:`Host` Interface manages communication between :term:`GenVM` and the
blockchain node.

Host is responsible for providing blockchain state to :term:`GenVM` and updating it.
