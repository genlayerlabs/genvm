## Running the GenVM

To run a genvm, one must start a genvm process with following arguments:
- `--host` tcp-it address or `unix://` prefixed unix domain socket
- `--message` message data as json, follows [schema](../schemas/message.json)
  See [example](../../executor/testdata/templates/message.json) that is used in tests

## How to ask GenVM to quit?
Send it `SIGTERM`. If it doesn't quit in some sensible amount of time just `SIGKILL` it

## How node receives code, message, ... from user
It is for node to decide. GenVM knows only about the calldata (and potentially message) and nothing else

## Communication protocol
All further communication is done via socket. If genvm process exited before sending the result, it means that genvm crushed. Potential bug should be reported

Method ids list is available as [json](../../executor/codegen/data/host-fns.json). It is advised to use it in the build system to codegen constants

Below is pseudo code, but [real code](../../executor/testdata/runner/base_host.py) is always up to date and is as readable as this code is (see `async def host_loop(handler: IHost):`)

```
const ACCOUNT_ADDR_SIZE = 20
const SLOT_ID_SIZE = 32

fn write_byte_slice(arr):
  write_u32_le len(arr)
  write_bytes arr
fn read_slice():
  len := read_u32_le
  data := read_bytes(len)
  return data
fn read_result():
  result_type := read_byte
  data := read_slice()
  return result_type, data

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
      # this is needed to ensure that genvm doesn't close socket before all data is read
      write_byte 0x00
      break
    json/methods/get_leader_nondet_result:
      call_no := read_u32_le
      data, err := host_get_leader_nondet_result(call_no)
      if err != json/errors/ok:
        write_byte err
      else:
        if data is error_code:
          write_byte data
        else:
          write_byte json/errors/ok
          result_type, result_data := data
          write_byte result_type
          write_byte_slice result_data
    json/methods/post_nondet_result:
      call_no := read_u32_le
      result := read_result()
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
```

See [mock implementation](../../executor/testdata/runner/mock_host.py)

## Types

### VM results
There are following codes:
- `Return`
- `VMError` indicating vm-produced error that usually can't be handled
- `UserError` user-produced error

#### Nondet blocks and sandbox encoding
- 1 byte of result code
- result as-is (calldata for `Return`, string for `VMError|UserError`)

#### Parent VM result
- 1 byte of result code
- calldata for `Return`, `{ "message": "string", "fingerprint": ... }` for `VMError|UserError`

### Calldata
`get_calldata` method must return [calldata encoded](../calldata.md) bytes that conform to ABI:
```typescript
{
  method?: string,  // only for non-consturctors
  args: Array<any>,
  kwargs?: { [key: string]: any }
}
```

### Read result
It has code followed by bytes, codes are:
- return, it is followed by calldata
- rollback and contract error, followed by a string; from host point of view there is no distinction between them
- just error, which is internal error, like llm's modules being absent

### Storage format
Storage can be seen as a mapping from account address to slot address to linear memory. It supports two operations: `read` and `write`. Reading undefined memory **must** return zeroes

Storage can be seen as a file system tree containing directories named as contracts which contain files named as slots, then following implementation is valid:
```bash
# read contract_a slot_b 10...100
cat db/contract_a/slot_b.bytes /dev/null | tail -c +10 | head -c +100
```

**NOTE**: calculating storage updates, hashes and so on is host's (node's) responsibility. It is [the same in geth](https://github.com/ethereum/go-ethereum/blob/67a3b087951a3f3a8e341ae32b6ec18f3553e5cc/core/state/state_object.go#L232): they have dirty override for the store
