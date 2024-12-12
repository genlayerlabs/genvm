## Running the GenVM

To run a genvm, one must start a genvm process with following arguments:
- `--host` tcp-it address or `unix://` prefixed unix domain socket
- `--message` (potential subject to change) message data as json
  ```typescript
  {
    "$schema": "https://raw.githubusercontent.com/yeagerai/genvm/refs/heads/main/doc/schemas/message.json", // optional
    "contract_account": "AQAAAAAAAAAAAAAAAAAAAAAAAAA=", // base64 address of contract account (callee)
    "sender_account": "AgAAAAAAAAAAAAAAAAAAAAAAAAA=",   // base64 address of caller
    "origin_account": "AgAAAAAAAAAAAAAAAAAAAAAAAAA=",   // base64 address of initiator
    "chain_id": "0",                                    // chain id, see consensus docs
    "value": null,
    "is_init": false,                                   // whenever contract is being instantiated (this allows to call a private method)
  }
  ```
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
const GENERIC_ADDR_SIZE = 32

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
      address := read_bytes(ACCOUNT_ADDR_SIZE)
      write_bytes_with_len host_code[address]
    json/methods/storage_read:
      read_type := read_byte as json/storage_type
      address := read_bytes(ACCOUNT_ADDR_SIZE)
      slot := read_bytes(GENERIC_ADDR_SIZE)
      index := read_u32_le
      len := read_u32_le
      write_bytes_with_len host_storage[address][slot][index..index+len] # must be exactly len in size
    json/methods/storage_write:
      # as per genvm definition this address can be only the address of entrypoint account
      address := read_bytes(ACCOUNT_ADDR_SIZE)
      slot := read_bytes(GENERIC_ADDR_SIZE)
      index := read_u32_le
      len := read_u32_le
      data := read_bytes(len)
      host_storage[address][slot][index..index+len] = data
    json/methods/consume_result:
      host_result := read_result()
      # this is needed to ensure that genvm doesn't close socket before all data is read
      write_byte 0x00
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
      address := read_bytes(ACCOUNT_ADDR_SIZE)
      len_calldata := read_u32_le
      calldata := read_bytes(len_calldata)
      len_code := read_u32_le
      code := read_bytes(len_code)
```

See [mock implementation](../../executor/testdata/runner/mock_host.py)

## Types

### Calldata
`append_calldata` method must return [calldata encoded](../calldata.md) bytes that conform to ABI:
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
