What is GenVM?
==============

GenVM is a WebAssembly-based virtual machine that powers **Intelligent Contracts** on the `GenLayer protocol <https://docs.genlayer.com/>`_. Unlike traditional smart contracts, Intelligent Contracts can access the web and call large language models (LLMs) while still reaching blockchain consensus.

GenVM is language-agnostic by design. Language-specific **runners** compile to WebAssembly and plug into GenVM as execution backends. Python is currently supported, with Rust support planned. Execution is deterministic by default, but GenVM provides controlled pathways for non-deterministic operations (web requests, LLM inference) and uses the :ref:`Equivalence Principle <key-concepts>` to reconcile results across validators.

This site is the **SDK and specification reference**. For tutorials, getting started guides, and contract development walkthroughs, see the `GenLayer documentation <https://docs.genlayer.com/>`_ and its `developer section <https://docs.genlayer.com/developers/>`_.
