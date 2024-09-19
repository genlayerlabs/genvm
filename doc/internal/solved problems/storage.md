# Persistent storage on blockchain from GenVM point of view

- [x] has ADR (no sensible alternatives)

## Initial problem statement
GenVM needs to provide contracts with ability to store data persistently

## Context
Storage can be language dependent: only a contract needs to read it and nothing else

## Sub problems
1. Usual data structures aren’t suitable:
    1. Allocated addresses are not persistent
    2. Allocation requires knowledge about all allocated addresses, which takes a lot of space and would cost a lot of reads at start time
    3. Disallows reference types (multiple references to a same location in the storage)
    4. Serialization works poorly as it will rewrite entire storage (consider rehash)
      Dynamic formats work head-recursively, consider the following deltas
        ```py
        >>> x = {1: '1', 2: '2345'}
        >>> ''.join(':{:02x}'.format(b) for b in pickle.dumps(x))
        ':80:04:95:14:00:00:00:00:00:00:00:7d:94:28:4b:01:8c:01:31:94:4b:02:8c:04:32:33:34:35:94:75:2e'
        #                                                        ^
        >>> x[1] = '0'
        >>> ''.join(':{:02x}'.format(b) for b in pickle.dumps(x))
        ':80:04:95:14:00:00:00:00:00:00:00:7d:94:28:4b:01:8c:01:30:94:4b:02:8c:04:32:33:34:35:94:75:2e'
        #                                                        ^
        >>> x[1] = '02'
        >>> ''.join(':{:02x}'.format(b) for b in pickle.dumps(x))
        ':80:04:95:15:00:00:00:00:00:00:00:7d:94:28:4b:01:8c:02:30:32:94:4b:02:8c:04:32:33:34:35:94:75:2e'
        #                                                    ^^^^^^^^
        ```
2. Freeing memory: there is no purpose in freeing memory, as it will be persistently stored on the blockchain. Optimizing for pruned node will force to implement GC running which is extremely expensive


## Solution
### Lowlevel
Storage consists of slots of linear memory, basis operations are `read` & `write`. If these are the only, `read` must return `0`s for unitinialzied memory
### Mid level
Host optimizes hot reads/writes
### High level
Have a custom encoding that will map storage format to language constructs
1. Constant size types are stored in-place one after another
2. Arbitrary sized types store data at slot `hash_combine(current_slot_addr, offset_in_slot)` and have size greater than 0 (so that if one such structure follows another they don’t compute to the same address)

### Python level
Storage must be statically typed. For this purpose type annotations can be used to generate a view class

## Pros
1. Storage efficiency

## Cons
1. Python must encode structures for every read/write
2. Python needs to have static typing at this place
3. "Hiding" it from user can lead to some unexpected behaviours
4. Storage can't work with built-in `list` and `dict` types
