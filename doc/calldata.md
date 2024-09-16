# Calldata

## Generic format

Each calldata value is prefixed by uleb128 number, which is treated as follows:

|type|least significant 3 bits|number shifted by this 3 bits|followed by|
|----|----|----|----|
|null|0|0|-|
|positive int or 0|1|value|-|
|negative int|2|abs(value) - 1|-|
|bytes|3|`length`|bytes\[`length`]|
|address|4|0|32 bytes of address|
|string|5|`length`|bytes\[`length`] of utf-8 encoded string
|array|6|`length`|_calldata_\[`length`]
|map|7|`length`|Pair(_FastString_, _calldata_)\[`length`] sorted by keys

_FastString_ is encoded as uleb128 length followed by utf8 encoded bytes (difference is that it doesn't have a type)

## What contract expects as a calldata?
```json
{
    "method": string,
    "args": array
}
```
