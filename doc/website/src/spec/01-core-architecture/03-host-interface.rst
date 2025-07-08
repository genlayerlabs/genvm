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

- **Graceful Shutdown**: Send ``SIGTERM`` signal
- **Force Termination**: Send ``SIGKILL`` if not responding
- **Crash Detection**: Process exit before sending result indicates crash (should be reported as bug)

**Node Responsibilities**: Node decides how to receive code and messages from users. GenVM only knows about calldata and message data.

Communication Protocol
----------------------

Binary Protocol Design
~~~~~~~~~~~~~~~~~~~~~~

**Constants**:

::

   ACCOUNT_ADDR_SIZE = 20
   SLOT_ID_SIZE = 32

**Data Encoding Functions**:

::

   write_byte_slice(arr):
     write_u32_le len(arr)
     write_bytes arr

   read_slice():
     len := read_u32_le
     data := read_bytes(len)
     return data

Protocol Loop
~~~~~~~~~~~~~

The :term:`host` processes requests in a loop until ``consume_result``:

::

   loop:
     method_id := read_byte
     match method_id
       json/methods/get_calldata:
         calldata, err := host_get_calldata()
         if err != json/errors/ok:
           write_byte err
         else:
           write_byte json/errors/ok
           write_byte_slice calldata

       json/methods/storage_read:
         read_type := read_byte as json/storage_type
         address := read_bytes(ACCOUNT_ADDR_SIZE)
         slot := read_bytes(SLOT_ID_SIZE)
         index := read_u32_le
         len := read_u32_le
         data, err := host_storage_read(read_type, address, slot, index, len)
         if err != json/errors/ok:
           write_byte err
         else:
           write_byte json/errors/ok
           write_bytes data # must be exactly len in size

       json/methods/storage_write:
         slot := read_bytes(SLOT_ID_SIZE)
         index := read_u32_le
         len := read_u32_le
         data := read_bytes(len)
         err := host_storage_write(slot, index, data)
         if err != json/errors/ok:
           write_byte err
         else:
           write_byte json/errors/ok

       json/methods/consume_result:
         host_result := read_result()
         # ensures genvm doesn't close socket before all data is read
         write_byte 0x00
         break

       json/methods/get_leader_nondet_result:
         call_no := read_u32_le
         data, err := host_get_leader_nondet_result(call_no)
         if err != json/errors/ok:
           write_byte err
         else:
           write_byte json/errors/ok
           write_byte_slice data

       json/methods/post_nondet_result:
         call_no := read_u32_le
         result := read_slice()
         err := host_post_nondet_result(call_no, result)
         if err != json/errors/ok:
           write_byte err
         else:
           write_byte json/errors/ok

       json/methods/post_message:
         address := read_bytes(ACCOUNT_ADDR_SIZE)
         calldata := read_slice()
         message_data := read_slice() # JSON string
         err := host_post_message(address, calldata, message_data)
         if err != json/errors/ok:
           write_byte err
         else:
           write_byte json/errors/ok

       json/methods/consume_fuel:
         gas := read_u64_le
         host_consume_fuel(gas)
         # note: this method doesn't send any response

       json/methods/deploy_contract:
         calldata := read_slice()
         code := read_slice()
         message_data := read_slice() # JSON string
         err := host_deploy_contract(calldata, code, message_data)
         if err != json/errors/ok:
           write_byte err
         else:
           write_byte json/errors/ok

       json/methods/eth_call:
         address := read_bytes(ACCOUNT_ADDR_SIZE)
         calldata := read_slice()
         result, err := host_eth_call(address, calldata)
         if err != json/errors/ok:
           write_byte err
         else:
           write_byte json/errors/ok
           write_byte_slice result

       json/methods/eth_send:
         address := read_bytes(ACCOUNT_ADDR_SIZE)
         calldata := read_slice()
         message_data := read_slice() # JSON string
         err := host_eth_send(address, calldata, message_data)
         if err != json/errors/ok:
           write_byte err
         else:
           write_byte json/errors/ok

       json/methods/get_balance:
         address := read_bytes(ACCOUNT_ADDR_SIZE)
         balance, err := host_get_balance(address)
         if err != json/errors/ok:
           write_byte err
         else:
           write_byte json/errors/ok
           write_bytes balance.to_le_bytes(32) # 256-bit integer

       json/methods/remaining_fuel_as_gen:
         fuel, err := host_remaining_fuel_as_gen()
         if err != json/errors/ok:
           write_byte err
         else:
           write_byte json/errors/ok
           write_bytes fuel.to_le_bytes(8) # 64-bit integer, must be safe integer (fits in double)

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
- Result data: calldata for ``Return``, string for ``VMError`` or ``UserError``

**Parent VM Result Encoding**:
- 1 byte of result code - Data format:
- ``Return``: calldata - ``VMError``/``UserError``: ``{ "message": "string", "fingerprint": ... }``

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
