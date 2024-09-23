# Floats

- [ ] has ADR (no alternatives yet)

## Initial problem statement
Floats are [partially non deterministic](https://github.com/WebAssembly/design/blob/main/Nondeterminism.md):
> 1. Except when otherwise specified, when an arithmetic operator returns NaN, there is nondeterminism in determining the specific bits of the NaN. However, wasm does still provide the guarantee that NaN values returned from an operation will not have 1 bits in their fraction field that aren't set in any NaN values in the input operands, except for the most significant bit of the fraction field (which most operators set to 1).
> 2. Except when otherwise specified, when an arithmetic operator with a floating point result type receives no NaN input values and produces a NaN result value, the sign bit of the NaN result value is nondeterministic. Fixed-width SIMD may want some flexibility 🦄
> 3. In SIMD.js, floating point values may or may not have subnormals flushed to zero.
> 4. In SIMD.js, operators ending in "Approximation" return approximations that may vary between platforms.

## Sub problems
1. Generic programs assume that floats are there
    - python:
        - parsing literals (`0.5`)
        - file stat timestamps are floats used in importlib
        - reading bytes uses characters/bytes ratio as a float
        - `datetime` module uses floats
2. Same contract has both deterministic and non-deterministic modes, so float instructions can't be banned at compile time

## Solution
1. Banning floats at runtime by replacing them in deterministic VM with unreachable in codegen
    - except for bitcast from integer types, constant loading and passing them around
2. Providing a "software float" library which is determinisitc
3. Patching binaries as `f64.add |-> call softfloat.f64_add`

### Pros
1. Most floating point operations work out of the box, including parsing and printing

### Cons
1. If binary was patched, non deterministic mode will use slow software implementation as well
    - this can be addressed with SIMD
