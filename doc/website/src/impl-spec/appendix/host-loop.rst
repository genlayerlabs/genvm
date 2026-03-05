Host Loop Pseudocode
====================

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

Multi-Host
~~~~~~~~~~

GenVM supports multiple host connections. Each host method is routed to a
specific host connection based on a ``method_hosts`` mapping in
``ExecutionData``. The mapping is a byte array where each index corresponds to a
method ID and the value is the host connection index. When the index is out of
bounds or the array is empty, host 0 is used as the default.

The executor accepts multiple ``--host`` arguments. Each host connection
independently runs the protocol described below.

In the manager deployment, host 0 is the node's host loop and host 1 is a
socketpair to the manager. The manager routes ``consume_result`` to itself
(host 1) while all other methods go to host 0.

Protocol Loop
~~~~~~~~~~~~~

The :term:`host` processes requests in a loop. Each host connection runs
independently, handling only the methods routed to it:

::

   loop:
     method_id := read_byte
     match method_id
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

       json/methods/consume_result:
         host_result := read_slice()
         # this is needed to ensure that genvm doesn't close socket before all data is read
         write_byte 0x00

       json/methods/consume_fuel:
         gas := read_u64_le
         host_consume_fuel(gas)
         # note: this method doesn't send any response

       json/methods/eth_call:
         address := read_bytes(ACCOUNT_ADDR_SIZE)
         calldata := read_slice()
         result, err := host_eth_call(address, calldata)
         if err != json/errors/ok:
           write_byte err
         else:
           write_byte json/errors/ok
           write_byte_slice result

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

       json/methods/notify_nondet_disagreement:
         call_no := read_u32_le
         host_notify_nondet_disagreement(call_no)
         # note: this method doesn't send any response

       json/methods/notify_finished:
         # signals that all execution is complete
         write_byte 0x00
