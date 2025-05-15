CFLAGSBASE=-DINLINE_LEVEL=9 -DSOFTFLOAT_FAST_INT64 -Ispec -Iberkeley-softfloat-3/source/include

%.o: %.c
	clang -nostdlib -c -o $@ $< $(CFLAGSBASE) $(CFLAGS)

lib: $(patsubst %.c,%.o,$(wildcard berkeley-softfloat-3/source/*.c spec/*.c))
	clang -v -nostdlib -o softfloat-out.wasm -flto -O3 -frandom-seed=0 -Wl,--no-entry -Wl,--export-dynamic -static $^
