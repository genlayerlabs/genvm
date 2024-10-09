# Calldata

- [x] has ADR (3+ alternatives)

## Initial problem statement
GenVM needs to have standardized encoding of of "calldata", so that any language can read and write it

## Context
Calldata it is a set of arguments (and a method name) that a message carry, any language should be able to parse it and to produce it (to call other contracts `public view` or send messages to them)

"Code" is handled by the genvm _Host_ and message data (value, is_init, etc) is out of this context as well

## Sub problems
1. Absolute byte-perfect consistency: hash after _serialize ∘ deserialize_ must be the same
2. Compact size to lower gas cost
3. Supporting `bigint`
4. Supporting `Address` type

## Solution
Having a [custom format](../../calldata.md)

## Pros
1. It solves all of the problems above

## Cons
1. It is not a well-known format
2. There is no existing tooling for it
2. Parsing it into `uint64` ensuring that it fits may be a bit tricky
