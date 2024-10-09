# Calldata

## Generic format

Each calldata value is prefixed by uleb128 number, which is treated as follows:

|type|least significant 3 bits|number shifted by this 3 bits|followed by|
|----|----|----|----|
|special|0|0 => null<br>1 => false<br>2 => true<br>3 => address| addres => 32 bytes of address<br> - otherwise |
|positive int or 0|1|value|-|
|negative int|2|abs(value) - 1|-|
|bytes|3|`length`|bytes\[`length`]|
|string|4|`length`|bytes\[`length`] of utf-8 encoded string
|array|5|`length`|_calldata_\[`length`]
|map|6|`length`|Pair(_FastString_, _calldata_)\[`length`] sorted by keys

_FastString_ is encoded as uleb128 length followed by utf8 encoded bytes (difference is that it doesn't have a type)

## What contract expects as a calldata?
See [*abi*](./internal/solved%20problems/5.%20abi.md)
