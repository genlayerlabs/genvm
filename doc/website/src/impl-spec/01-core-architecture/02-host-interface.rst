:term:`Host` Interface Protocol
===============================

Overview
--------

The :term:`Host` Interface defines the communication protocol between GenVM and
the blockchain node. GenVM launches with ``--host`` (socket address) and
``--message`` (JSON data) parameters and communicates via a binary
protocol over TCP or Unix domain sockets.

Process Management
------------------

GenVM Execution
~~~~~~~~~~~~~~~

**Launch Parameters**:

- ``--host``: TCP address or ``unix://`` prefixed Unix domain socket
- ``--message``: Message data as JSON following message schema

**Process Control**:

- **Termination**: The executor has no internal timeout or graceful-shutdown path and may be killed at any moment with ``SIGKILL`` (see :doc:`04-executor`). Timeouts are enforced by the :term:`Manager`.
- **Crash Detection**: Process exit before sending result indicates a kill/timeout or a crash (a crash should be reported as a bug)

**Node Responsibilities**: Node decides how to receive code and messages from users. GenVM only knows about calldata and message data.

Communication Protocol
----------------------

Pseudocode is available in :doc:`../appendix/host-loop`

Binary Protocol Design
~~~~~~~~~~~~~~~~~~~~~~



Data Types and Results
----------------------

VM Result Codes
~~~~~~~~~~~~~~~

**Result Types**:

- ``Return``: Successful execution
- ``VMError``: VM-produced error that usually can't be handled
- ``UserError``: User-produced error

Result Encoding
~~~~~~~~~~~~~~~

**Non-deterministic Blocks and Sandbox Encoding**:

- 1 byte of result code
- Result data: calldata for ``Return``, tagged calldata for ``UserError``, string for ``VMError``

**Parent VM Result Encoding**:

- 1 byte of result code
- Data format:

    - ``Return``: calldata
    - ``VMError`` / ``UserError``: ``{ "message": "string", "fingerprint": ... }``

**Host Responsibility**: Calculating storage updates, hashes, and state
management (similar to Ethereum's dirty storage override pattern).

Method ID Reference
-------------------

Method IDs are available as JSON in the build system for code
generation.

Error Handling
--------------

- **Protocol Errors**: Most methods return error codes, with ``json/errors/ok`` indicating success
- **Communication Failures**: Socket communication errors indicate protocol violations
- **Process Termination**: Unexpected process exit indicates GenVM crash and should be reported
