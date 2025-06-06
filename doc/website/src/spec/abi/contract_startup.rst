.. _startup-process-reference:

Contract Startup Process
========================

#. GenVM is started with "message"
#. GenVM receives contract code for recipient address from *Host*
#. GenVM inspects contract code to find construct a file tree of it
#. If contract code is a zip it is equivalent to tree
#. If it is a plain-text contract, then GenVM tries to parse "runner comment" from it (as of now, ``#``, ``//``, ``--`` are supported),
    and constructs following tree: ``/file = <original file>, /runner.json = <parsed comment>``
#. It acts upon ``/runner.json`` provided actions
#. In the end this actions must lead to ``StartWasm`` one
#. GenVM starts wasm, with ``stdin`` of calldata-encoded *ExtendedMessage*
#. For startup contract must use following fields:

    #. ``entry_kind`` one of main, sandbox and consensus_stage.
        Main means regular contract entry, sandbox stands for sandbox and consensus_stage has ``entry_stage_data`` used for non-deterministic operation
    #. ``entry_data`` blob of bytes
    #. ``entry_stage_data`` calldata information provided by "consensus implementation".
        Right now it is ``null`` for leader and ``{leaders_result: <calldata>}`` for validator
