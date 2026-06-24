Response Schemas
================

Operations that return data via file descriptor use specific encoding formats
based on operation type.

.. _gvm-def-subvm-result-encoding:

Sub-VM Result Encoding
----------------------

Operations that spawn sub-VMs (``CallContract``, ``RunNondet``, ``Sandbox``) return
results through a file descriptor with the following binary format:

.. code-block::

   [result_code: u8][data: bytes]

Where ``result_code`` is a :ref:`gvm-def-enum-result-code`:

- :ref:`gvm-def-enum-value-result-code-return` (``0``): Successful execution
- :ref:`gvm-def-enum-value-result-code-user-error` (``1``): Contract-initiated error
- :ref:`gvm-def-enum-value-result-code-vm-error` (``2``): VM-level error

The ``data`` portion depends on the result code:

- **return**: :ref:`Calldata Encoded <gvm-def-calldata-encoding>` return value
- **user_error**: :ref:`Calldata Encoded <gvm-def-calldata-encoding>` error value
- **vm_error**: UTF-8 encoded :ref:`gvm-def-str-trie-vm-error` string

.. _gvm-def-module-result-encoding:

Module Result Encoding
----------------------

Operations that invoke external modules (``WebRender``, ``WebRequest``, ``ExecPrompt``,
``ExecPromptTemplate``) return results as :ref:`Calldata Encoded <gvm-def-calldata-encoding>`
data with a success/error wrapper:

.. code-block::

   // On success
   {
     "ok": <result>
   }

   // On error
   {
     "error": <error_details>
   }

The ``<result>`` structure depends on the operation:

``WebRender`` Response
~~~~~~~~~~~~~~~~~~~~~~

Returns one of the following based on render mode:

.. code-block::

   // For mode "text"
   {"ok": {"text": String}}

   // For mode "html" (returns HTTP response)
   {"ok": {"response": Response}}

   // For mode "screenshot"
   {"ok": {"image": Bytes}}

Where ``Response`` is:

.. code-block::

   {
     "status": Number,           // HTTP status code (u16)
     "headers": Map,             // String -> Bytes mapping
     "body": Bytes               // Response body
   }

``WebRequest`` Response
~~~~~~~~~~~~~~~~~~~~~~~

.. code-block::

   {
     "ok": {
       "status": Number,         // HTTP status code (u16)
       "headers": Map,           // String -> Bytes mapping of response headers
       "body": Bytes             // Response body
     }
   }

``ExecPrompt`` Response
~~~~~~~~~~~~~~~~~~~~~~~

.. code-block::

   // When response_format is "text"
   {"ok": String}

   // When response_format is "json"
   {"ok": Object}

``ExecPromptTemplate`` Response
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

.. code-block::

   // For EqComparative template (boolean result)
   {"ok": Bool}

   // For EqNonComparativeValidator template (boolean result)
   {"ok": Bool}

   // For EqNonComparativeLeader template (text result)
   {"ok": String}

Module Error Format
~~~~~~~~~~~~~~~~~~~

When a module operation fails, the error is returned as:

.. code-block::

   {
     "error": <GenericValue>
   }

Where ``<GenericValue>`` may contain error details from the module.
