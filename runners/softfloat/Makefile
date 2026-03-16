CFLAGSBASE=-DINLINE_LEVEL=9 -DSOFTFLOAT_FAST_INT64 -Ispec -Iberkeley-softfloat-3/source/include

%.o: %.c
	clang -nostdlib -c -o $@ $< $(CFLAGSBASE) $(CFLAGS)

simd.o: simd.wat
	wat2wasm --relocatable $(WAT2WASMFLAGS) $^ -o simd.o

base.o: base.wat
	wat2wasm --relocatable $(WAT2WASMFLAGS) $^ -o base.o

lib: $(patsubst %.c,%.o,$(wildcard berkeley-softfloat-3/source/*.c spec/*.c)) simd.o base.o
	clang -v -nostdlib -o softfloat-out.wasm --no-wasm-opt -frandom-seed=0 -Wl,--no-entry -Wl,--export-dynamic -static -O0 $^
