AR_SCRIPT="CREATE /opt/wasm32-wasip1-root/lib/libffi.a"

for i in stub_ffi.c src/closures.c src/prep_cif.c src/tramp.c src/debug.c src/raw_api.c src/types.c
do
	FNAME="$(basename "$i")"
	clang \
		-o "$i.o" \
		-fPIC \
		-I/opt/wasm32-wasip1-root/include/ \
		-Iinclude -Iwasm32-unknown-wasi/ \
		-c "$i"
	AR_SCRIPT="$AR_SCRIPT"$'\n'"ADDMOD $i.o"
done

AR_SCRIPT="$AR_SCRIPT"$'\n'"SAVE"
AR_SCRIPT="$AR_SCRIPT"$'\n'"END"

echo "$AR_SCRIPT" | ar -M
