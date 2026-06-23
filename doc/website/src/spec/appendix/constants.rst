Constants
=========

.. _gvm-def-enum-result-code:

result_code
-----------

Type: u8

.. _gvm-def-enum-value-result-code-return:

return
~~~~~~

Value: ``0``

.. _gvm-def-enum-value-result-code-user-error:

user_error
~~~~~~~~~~

Value: ``1``

.. _gvm-def-enum-value-result-code-vm-error:

vm_error
~~~~~~~~

Value: ``2``

.. _gvm-def-enum-value-result-code-internal-error:

internal_error
~~~~~~~~~~~~~~

Value: ``3``

.. _gvm-def-enum-storage-type:

storage_type
------------

Type: u8

.. _gvm-def-enum-value-storage-type-default:

default
~~~~~~~

Value: ``0``

.. _gvm-def-enum-value-storage-type-latest-final:

latest_final
~~~~~~~~~~~~

Value: ``1``

.. _gvm-def-enum-value-storage-type-latest-non-final:

latest_non_final
~~~~~~~~~~~~~~~~

Value: ``2``

.. _gvm-def-enum-entry-kind:

entry_kind
----------

Type: u8

.. _gvm-def-enum-value-entry-kind-main:

main
~~~~

Value: ``0``

.. _gvm-def-enum-value-entry-kind-sandbox:

sandbox
~~~~~~~

Value: ``1``

.. _gvm-def-enum-value-entry-kind-consensus-stage:

consensus_stage
~~~~~~~~~~~~~~~

Value: ``2``

.. _gvm-def-consts-memory-limiter-consts:

memory_limiter_consts
---------------------

Type: u32

.. _gvm-def-consts-value-memory-limiter-consts-table-entry:

table_entry
~~~~~~~~~~~

Value: ``64``

.. _gvm-def-consts-value-memory-limiter-consts-file-mapping:

file_mapping
~~~~~~~~~~~~

Value: ``256``

.. _gvm-def-consts-value-memory-limiter-consts-fd-allocation:

fd_allocation
~~~~~~~~~~~~~

Value: ``96``

.. _gvm-def-consts-root-offsets:

root_offsets
------------

Type: u32

.. _gvm-def-consts-value-root-offsets-major:

major
~~~~~

Value: ``0``

.. _gvm-def-consts-value-root-offsets-contract:

contract
~~~~~~~~

Value: ``1``

.. _gvm-def-consts-value-root-offsets-code:

code
~~~~

Value: ``2``

.. _gvm-def-consts-value-root-offsets-locked-slots:

locked_slots
~~~~~~~~~~~~

Value: ``3``

.. _gvm-def-consts-value-root-offsets-upgraders:

upgraders
~~~~~~~~~

Value: ``4``

.. _gvm-def-consts-value-root-offsets-code-slot:

code_slot
~~~~~~~~~

Value: ``5``

.. _gvm-def-consts-top-limits:

top_limits
----------

Type: u32

.. _gvm-def-consts-value-top-limits-nondet-blocks:

nondet_blocks
~~~~~~~~~~~~~

Value: ``4096``

.. _gvm-def-consts-value-top-limits-locked-slots:

locked_slots
~~~~~~~~~~~~

Value: ``256``

.. _gvm-def-consts-value-top-limits-upgraders:

upgraders
~~~~~~~~~

Value: ``32``

.. _gvm-def-consts-value-top-limits-vm-recursion:

vm_recursion
~~~~~~~~~~~~

Value: ``512``

.. _gvm-def-consts-value-top-limits-web-request-min-space:

web_request_min_space
~~~~~~~~~~~~~~~~~~~~~

Value: ``65536``

.. _gvm-def-consts-value-top-limits-web-render-min-space:

web_render_min_space
~~~~~~~~~~~~~~~~~~~~

Value: ``134217728``

.. _gvm-def-consts-value-top-limits-max-fds:

max_fds
~~~~~~~

Value: ``1024``

.. _gvm-def-enum-special-method:

special_method
--------------

Type: str

.. _gvm-def-enum-value-special-method-get-schema:

get_schema
~~~~~~~~~~

Value: ``#get-schema``

.. _gvm-def-enum-value-special-method-errored-message:

errored_message
~~~~~~~~~~~~~~~

Value: ``#error``

.. _gvm-def-str-trie-vm-error:

vm_error
--------

Type: str_trie

.. _gvm-def-str-trie-value-vm-error-timeout:

``timeout``
~~~~~~~~~~~

.. _gvm-def-str-trie-value-vm-error-absent-leader-nondet-output:

``absent_leader_nondet_output``
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

.. _gvm-def-str-trie-value-vm-error-exit-code:

``exit_code``
~~~~~~~~~~~~~

Param: i32

.. _gvm-def-str-trie-value-vm-error-wasm-trap:

``wasm_trap``
~~~~~~~~~~~~~

Param: str

.. _gvm-def-str-trie-value-vm-error-OOM-RAM:

``OOM RAM``
~~~~~~~~~~~

.. _gvm-def-str-trie-value-vm-error-OOM-RAM-table:

``OOM RAM table``
~~~~~~~~~~~~~~~~~

.. _gvm-def-str-trie-value-vm-error-OOM-RAM-memory:

``OOM RAM memory``
~~~~~~~~~~~~~~~~~~

.. _gvm-def-str-trie-value-vm-error-OOM-RAM-limit:

``OOM RAM limit``
~~~~~~~~~~~~~~~~~

.. _gvm-def-str-trie-value-vm-error-OOM-storage:

``OOM storage``
~~~~~~~~~~~~~~~

.. _gvm-def-str-trie-value-vm-error-OOM-receipt-nondet-output:

``OOM receipt nondet_output``
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

.. _gvm-def-str-trie-value-vm-error-OOM-receipt-message-internal:

``OOM receipt message internal``
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

.. _gvm-def-str-trie-value-vm-error-OOM-receipt-message-external:

``OOM receipt message external``
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

.. _gvm-def-str-trie-value-vm-error-OOM-fees-internal:

``OOM fees internal``
~~~~~~~~~~~~~~~~~~~~~

.. _gvm-def-str-trie-value-vm-error-OOM-fees-external:

``OOM fees external``
~~~~~~~~~~~~~~~~~~~~~

.. _gvm-def-str-trie-value-vm-error-invalid-contract:

``invalid_contract``
~~~~~~~~~~~~~~~~~~~~

.. _gvm-def-str-trie-value-vm-error-invalid-contract-absent-runner-comment:

``invalid_contract absent_runner_comment``
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

.. _gvm-def-str-trie-value-vm-error-invalid-contract-not-utf8-text:

``invalid_contract not_utf8_text``
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

.. _gvm-def-str-trie-value-vm-error-invalid-contract-malformed-runner:

``invalid_contract malformed_runner``
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

.. _gvm-def-str-trie-value-vm-error-invalid-contract-major-mismatch:

``invalid_contract major_mismatch``
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

.. _gvm-def-str-trie-value-vm-error-invalid-contract-wasm-validating:

``invalid_contract wasm validating``
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

.. _gvm-def-str-trie-value-vm-error-invalid-contract-wasm-linking:

``invalid_contract wasm linking``
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

.. _gvm-def-str-trie-value-vm-error-invalid-contract-wasm-entrypoint:

``invalid_contract wasm entrypoint``
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

.. _gvm-def-str-trie-value-vm-error-host:

``host``
~~~~~~~~

Param: str

.. _gvm-def-const-event-max-topics:

event_max_topics
----------------

Type: u32

Value: ``4``
