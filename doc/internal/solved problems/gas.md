# Gas

- [ ] has ADR (no alternatives)

## Initial problem statement
As a blockchain VM GenVM needs to know how much gas was consumed

## Sub problems
GenVM can spawn multiple wasm vms that can optionally run in parallel

## Solution
1. Using concept of a fuel from wasmtime
2. Patching wasmtime to make fuel shared between instances of wasmtime

## Pros
1. Allows to count wasm instructions that were actually executed
2. Allows to run VMs in parallel and consume fuel in parallel

## Cons
1. Making counter shared will slow down the execution: previously operations were not atomic and variable could be cached locally
2. Implementation complexity due to two fuel variables, current implementation has a lock for some operations
