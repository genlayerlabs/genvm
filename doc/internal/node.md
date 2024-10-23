## Running the GenVM

To run a genvm, one must start a genvm process with following arguments:
- `--host` tcp-it address or `unix://` prefixed unix domain socket
- `--message` (potential subject to change) message data
  ```json
  {
    "contract_account": "AQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=", // base64 address of contract account
    "gas": 9007199254740991, // initial gas amout. <= u64::max
    "is_init": false, // whenever it is contract being instantiated (this allows to call private method)
    "sender_account": "AgAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=", // base64 address of who is calling the contract
    "value": null // value attached to message, see solidity msg.value
  }
  ```

## How node receives code, message, ... from user
It is for node to decide. GenVM knows only about the calldata (and potentially message) and nothing else

## Storage format
Storage can be seen as a mapping from account address to slot address to linear memory. It supports two operations: read and write. Reading undefined memory must return zeroes

## Communication protocol
All further communication is done via socket. If genvm process exited before sending the result, it means that genvm crushed. Potential bug should be reported

Method ids list is available as [json](../../executor/codegen/data/host-fns.json)

```
fn write_bytes_with_len(arr):
  write_u32_le len(arr)
  write_bytes arr
fn read_result():
  result_type := read_byte
  len := read_u32_le
  data := read_bytes(len)
  return result_type, data

loop:
  method_id := read_byte
  match method_id
    json/methods/append_calldata:
      write_bytes_with_len host_calldata
    json/methods/get_code:
      address := read_bytes(32)
      write_bytes_with_len host_code[address]
    json/methods/storage_read:
      available_gas := read_u64_le
      address := read_bytes(32)
      slot := read_bytes(32)
      index := read_u32_le
      len := read_u32_le
      write_u64_le host_consumed_gas
      write_bytes_with_len host_storage[address][slot][index..index+len] # must be exactly len in size
    json/methods/storage_write:
      available_gas := read_u64_le
      # as per genvm definition this address can be only the address of entrypoint account
      address := read_bytes(32)
      slot := read_bytes(32)
      index := read_u32_le
      len := read_u32_le
      data := read_bytes(len)
      host_storage[address][slot][index..index+len] = data
      write_u64_le consumed_gas
    json/methods/consume_result:
      host_result := read_result()
      break
    json/methods/get_leader_nondet_result:
      call_no = read_u32_le
      if host_is_leader:
        write_byte json/result_code/none
      else:
        # note: code here can't be an error
        write_byte host_leader_result_code[call_no]
        write_bytes_with_len host_leader_result_data[call_no]
    json/methods/post_nondet_result:
      call_no = read_u32_le
      host_nondet_result[call_no] = read_result()
      # validator can just skip this bytes if this command was sent
    json/methods/post_message:
      address := read_bytes(32)
      len_calldata := read_u32_le
      calldata := read_bytes(len_calldata)
      len_code := read_u32_le
      code := read_bytes(len_code)
```

See [mock implementation](../../executor/testdata/runner/mock_host.py)
